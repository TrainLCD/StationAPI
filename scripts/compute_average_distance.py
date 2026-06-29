#!/usr/bin/env python3
"""
路線ごとの平均駅間距離 (average_distance) を OpenStreetMap の線路ジオメトリから
「線路に沿った実際の距離」として算出し、data/2!lines.csv を更新するスクリプト。

従来の average_distance は「駅座標の直線距離 × 路線種別ごとの固定係数(1.05〜1.25)」
という推定値だった。本スクリプトは OSM の鉄道ルート関係(route relation)から当該路線の
線路だけを取り出し、隣接駅間を線路に沿って経路探索することで実距離を求める。

データソース: OpenStreetMap (c) OpenStreetMap contributors, ODbL.
              https://www.openstreetmap.org/copyright

使い方:
    # 既知路線で較正・精度確認（CSVは書き換えない）
    python3 scripts/compute_average_distance.py --validate

    # 任意の line_cd を個別計算（CSVは書き換えない）
    python3 scripts/compute_average_distance.py --lines 11302,1002

    # 全路線を計算して data/2!lines.csv を書き換える
    python3 scripts/compute_average_distance.py --apply

取得した OSM ジオメトリは scripts/.osm_cache/ にキャッシュされ、再実行が速くなる。
"""
from __future__ import annotations

import argparse
import csv
import heapq
import json
import math
import os
import sys
import time
import urllib.parse
import urllib.request

# ---------------------------------------------------------------------------
# パス・定数
# ---------------------------------------------------------------------------
HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
LINES_CSV = os.path.join(ROOT, "data", "2!lines.csv")
STATIONS_CSV = os.path.join(ROOT, "data", "3!stations.csv")
CACHE_DIR = os.path.join(HERE, ".osm_cache")

# 複数ミラーをラウンドロビン。混雑時(504)や不通時に切り替える。
OVERPASS_ENDPOINTS = [
    "https://maps.mail.ru/osm/tools/overpass/api/interpreter",
    "https://overpass-api.de/api/interpreter",
    "https://overpass.kumi.systems/api/interpreter",
]
# 1リクエストあたりの最大待ち時間(秒)。不通ミラーで長時間ブロックしないよう短めに。
HTTP_TIMEOUT = 60
USER_AGENT = "StationAPI-average-distance/1.0 (railway data maintenance)"

# 駅をグラフノードへ対応付ける際の半径(m)。複線(上下線)の両トラックを拾うため広めに取る。
SNAP_RADIUS_M = 70.0
# ルート関係を当該路線と認める最大カバレッジ(m)。全駅がこの距離以内であること。
MATCH_MAX_M = 200.0
# 経路探索結果の妥当範囲。直線距離に対する比がこの範囲外なら失敗扱い。
MIN_RATIO = 0.8
MAX_RATIO = 4.0

# line_type -> 旧来の固定係数（フォールバック時に使用）
FALLBACK_FACTOR = {"1": 1.05, "2": 1.15, "3": 1.10, "4": 1.25, "5": 1.10, "0": 1.15}

# 較正用の既知路線（おおよその実営業キロ平均駅間距離・メートル）
KNOWN = {
    "11302": ("山手線", 1150),
    "1002": ("東海道新幹線", 32213),
    "1004": ("東北新幹線", None),
    "11329": ("中央線快速", None),
}


# ---------------------------------------------------------------------------
# 幾何ユーティリティ
# ---------------------------------------------------------------------------
def haversine(lon1: float, lat1: float, lon2: float, lat2: float) -> float:
    """2点間の大圏距離(メートル)。"""
    r = 6371000.0
    p1, p2 = math.radians(lat1), math.radians(lat2)
    dphi = math.radians(lat2 - lat1)
    dlmb = math.radians(lon2 - lon1)
    a = math.sin(dphi / 2) ** 2 + math.cos(p1) * math.cos(p2) * math.sin(dlmb / 2) ** 2
    return 2 * r * math.asin(math.sqrt(a))


# ---------------------------------------------------------------------------
# Overpass 取得（リトライ＋エンドポイント切替＋ローカルキャッシュ）
# ---------------------------------------------------------------------------
def _cache_path(key: str) -> str:
    """Overpass クエリ文字列に対応するキャッシュファイルのパスを返す。"""
    import hashlib

    h = hashlib.sha1(key.encode()).hexdigest()
    return os.path.join(CACHE_DIR, f"{h}.json")


def overpass(query: str, *, tries: int = 6) -> dict:
    """Overpass クエリを投げて JSON を返す。結果はキャッシュする。"""
    os.makedirs(CACHE_DIR, exist_ok=True)
    cp = _cache_path(query)
    if os.path.exists(cp):
        with open(cp, encoding="utf-8") as f:
            return json.load(f)

    last_err = None
    for attempt in range(tries):
        endpoint = OVERPASS_ENDPOINTS[attempt % len(OVERPASS_ENDPOINTS)]
        try:
            data = urllib.parse.urlencode({"data": query}).encode()
            req = urllib.request.Request(
                endpoint, data=data, headers={"User-Agent": USER_AGENT}
            )
            with urllib.request.urlopen(req, timeout=HTTP_TIMEOUT) as resp:
                result = json.load(resp)
            with open(cp, "w", encoding="utf-8") as f:
                json.dump(result, f)
            return result
        except Exception as e:  # noqa: BLE001 - ネットワーク系の各種例外をまとめて再試行
            last_err = e
            wait = min(4 * (attempt + 1), 30)
            sys.stderr.write(f"    overpass retry {attempt} ({endpoint}): {e!r} -> sleep {wait}s\n")
            time.sleep(wait)
    raise RuntimeError(f"overpass failed after {tries} tries: {last_err!r}")


# ---------------------------------------------------------------------------
# データ読み込み
# ---------------------------------------------------------------------------
def load_lines() -> list[dict]:
    """data/2!lines.csv を辞書のリストとして読み込む。"""
    with open(LINES_CSV, newline="", encoding="utf-8") as f:
        return list(csv.DictReader(f))


def load_stations_by_line() -> dict[str, list[dict]]:
    """稼働駅(e_status=0)を line_cd ごとにまとめた辞書を返す。"""
    by_line: dict[str, list[dict]] = {}
    with open(STATIONS_CSV, newline="", encoding="utf-8") as f:
        for s in csv.DictReader(f):
            if s["e_status"] == "0":
                by_line.setdefault(s["line_cd"], []).append(s)
    return by_line


def line_station_coords(stations: list[dict]) -> list[tuple[float, float, str]]:
    """駅リストを e_sort 順に並べ、(経度, 緯度, 駅名) のリストにして返す。"""
    sts = sorted(stations, key=lambda s: int(s["e_sort"]))
    return [(float(s["lon"]), float(s["lat"]), s["station_name"]) for s in sts]


# ---------------------------------------------------------------------------
# ルート関係の取得・マッチング
# ---------------------------------------------------------------------------
ROUTE_FILTER = '["route"~"^(train|railway|subway|light_rail|monorail|tram)$"]'


def _name_tokens(name: str) -> str:
    """路線名比較用の簡易正規化(JR 表記や全角括弧の揺れを吸収する)。"""
    # 比較用に括弧書き等を緩く扱うための簡易正規化
    return name.replace("JR", "").replace("（", "(").replace("）", ")")


def _rel_ways(rel: dict) -> list[list[tuple[float, float]]]:
    """`out geom;` で取得した関係要素から、構成way(>=2点)のジオメトリ列を取り出す。"""
    ways = [
        [(p["lon"], p["lat"]) for p in m["geometry"]]
        for m in rel.get("members", [])
        if m["type"] == "way" and "geometry" in m
    ]
    return [w for w in ways if len(w) >= 2]


def _overpass_name_regex(line_name: str) -> str:
    """路線名から Overpass の name 正規表現（メタ文字エスケープ済み部分一致）を作る。"""
    import re

    base = line_name.strip()
    return re.escape(base) if base else ""


def _pick_relation(rels: list[dict], coords, line_name: str):
    """候補関係群（ジオメトリ込み）から当該路線を1つ選ぶ。

    誤マッチ防止のため「全駅 ≤ MATCH_MAX_M」を必須とし、名前一致を優先、
    同条件ではカバレッジ最良を採用する。候補無しなら None。
    """
    norm_line = _name_tokens(line_name)

    def name_match(rel) -> bool:
        nm = rel.get("tags", {}).get("name") or ""
        nn = _name_tokens(nm)
        return bool(nn) and (norm_line in nn or nn in norm_line)

    best = None
    best_key = None
    best_ways = best_cov = best_max = None
    for rel in rels:
        if rel.get("type") != "relation":
            continue
        ways = _rel_ways(rel)
        if not ways:
            continue
        total = 0.0
        mx = 0.0
        for c in coords:
            # 駅から線路への距離は頂点ではなく線分への投影距離で測る。
            # 頂点だけだと長い線分の途中にある駅を過大に遠いと誤判定し、
            # 有効な relation を取りこぼして不要なフォールバックを招くため。
            d = min(
                _project_to_segment(c[0], c[1], w[i], w[i + 1])[0]
                for w in ways
                for i in range(len(w) - 1)
            )
            total += d
            mx = max(mx, d)
        if mx > MATCH_MAX_M:
            continue
        cov = total / len(coords)
        # 名前一致を最優先（False=0 が前）、次にカバレッジ昇順
        key = (not name_match(rel), cov)
        if best_key is None or key < best_key:
            best, best_key, best_ways, best_cov, best_max = rel, key, ways, cov, mx
    if best is None:
        return None
    return best, best_ways, best_cov, best_max


def find_best_relation(coords: list[tuple[float, float, str]], line_name: str):
    """当該路線のルート関係を1つ選び、(関係要素, 構成way群, 平均/最大カバレッジ) を返す。

    高速化のため1リクエストで候補関係をジオメトリ込み取得する（per-relation の再取得をしない）。
    まず路線名で絞り込み、ヒットしなければ bbox 内の全ルート関係に広げる。候補無しなら None。
    """
    lons = [c[0] for c in coords]
    lats = [c[1] for c in coords]
    pad = 0.005
    bb = f"{min(lats) - pad},{min(lons) - pad},{max(lats) + pad},{max(lons) + pad}"

    # 1) 路線名で絞り込み（小さく速い）
    name_re = _overpass_name_regex(line_name)
    if name_re:
        q = (
            f"[out:json][timeout:180];"
            f"relation[\"type\"=\"route\"]{ROUTE_FILTER}[\"name\"~\"{name_re}\"]({bb});"
            f"out geom;"
        )
        rels = [e for e in overpass(q)["elements"] if e["type"] == "relation"]
        picked = _pick_relation(rels, coords, line_name)
        if picked is not None:
            return picked

    # 2) bbox 内の全ルート関係に広げる（ジオメトリ込み・1リクエスト）
    q = (
        f"[out:json][timeout:180];"
        f"relation[\"type\"=\"route\"]{ROUTE_FILTER}({bb});"
        f"out geom;"
    )
    rels = [e for e in overpass(q)["elements"] if e["type"] == "relation"]
    return _pick_relation(rels, coords, line_name)


# ---------------------------------------------------------------------------
# グラフ構築 & 経路探索
# ---------------------------------------------------------------------------
def _nid(p):
    """座標を丸めてグラフのノード ID に正規化する。"""
    return (round(p[0], 7), round(p[1], 7))


def build_graph(ways: list[list[tuple[float, float]]]) -> dict:
    """way 群から、辺重みを隣接頂点間ハバサイン距離とする無向グラフを構築する。"""
    adj: dict[tuple[float, float], list[tuple[tuple[float, float], float]]] = {}
    for w in ways:
        for i in range(len(w) - 1):
            a, b = _nid(w[i]), _nid(w[i + 1])
            if a == b:
                continue
            d = haversine(w[i][0], w[i][1], w[i + 1][0], w[i + 1][1])
            adj.setdefault(a, []).append((b, d))
            adj.setdefault(b, []).append((a, d))
    return adj


def _project_to_segment(lon0: float, lat0: float, p1, p2):
    """駅(lon0,lat0)を線分 p1-p2 に投影し、(垂線距離m, 端点p1までの線路沿い距離m,
    端点p2までの線路沿い距離m) を返す。短距離なので局所平面近似で計算する。"""
    coslat = math.cos(math.radians(lat0))

    def xy(p):
        return ((p[0] - lon0) * coslat * 111320.0, (p[1] - lat0) * 110540.0)

    ax, ay = xy(p1)
    bx, by = xy(p2)
    abx, aby = bx - ax, by - ay
    l2 = abx * abx + aby * aby
    if l2 == 0.0:
        d = math.hypot(ax, ay)
        return d, 0.0, 0.0
    # 駅を原点とした投影パラメータ t（0..1にクランプ）
    t = -(ax * abx + ay * aby) / l2
    t = max(0.0, min(1.0, t))
    px, py = ax + t * abx, ay + t * aby
    perp = math.hypot(px, py)
    seglen = math.sqrt(l2)
    return perp, t * seglen, (1.0 - t) * seglen


def snap_anchors(ways, lon: float, lat: float, radius: float = SNAP_RADIUS_M):
    """駅を線路セグメントに投影し、到達アンカー {ノード: 投影点からの線路沿いオフセットm}
    を返す。半径内のセグメントが無ければ最も近い1セグメントを採用する。
    併せて最近傍セグメントへの垂線距離(スナップ品質)も返す。"""
    anchors: dict[tuple[float, float], float] = {}
    near_perp = float("inf")
    near_anchor = None
    for w in ways:
        for i in range(len(w) - 1):
            p1, p2 = w[i], w[i + 1]
            perp, d1, d2 = _project_to_segment(lon, lat, p1, p2)
            if perp < near_perp:
                near_perp = perp
                near_anchor = ((_nid(p1), d1), (_nid(p2), d2))
            if perp <= radius:
                n1, n2 = _nid(p1), _nid(p2)
                if d1 < anchors.get(n1, float("inf")):
                    anchors[n1] = d1
                if d2 < anchors.get(n2, float("inf")):
                    anchors[n2] = d2
    if not anchors and near_anchor is not None:
        for n, off in near_anchor:
            if off < anchors.get(n, float("inf")):
                anchors[n] = off
    return list(anchors.items()), near_perp


def route_anchors(adj: dict, src: list, dst: list, limit: float):
    """投影アンカー間の線路沿い最短距離。src/dst は (ノード, オフセットm) のリスト。
    各始点をオフセットで初期化し、終点到達時に終点オフセットを足した最小値を返す。"""
    dist = {}
    pq = []
    for n, off in src:
        if off < dist.get(n, float("inf")):
            dist[n] = off
            heapq.heappush(pq, (off, n))
    dst_off = {}
    for n, off in dst:
        if off < dst_off.get(n, float("inf")):
            dst_off[n] = off
    best = float("inf")
    while pq:
        d, u = heapq.heappop(pq)
        if d > dist.get(u, float("inf")) or d > limit:
            continue
        if d >= best:
            break  # d は単調増加。これ以上 best を更新できない
        if u in dst_off:
            best = min(best, d + dst_off[u])
        for v, w in adj.get(u, []):
            nd = d + w
            if nd < dist.get(v, float("inf")):
                dist[v] = nd
                heapq.heappush(pq, (nd, v))
    return best if best < float("inf") else None


# ---------------------------------------------------------------------------
# 路線ごとの平均駅間距離算出
# ---------------------------------------------------------------------------
class LineResult:
    def __init__(self, line_cd, line_name):
        self.line_cd = line_cd
        self.line_name = line_name
        self.average = None          # 採用した平均駅間距離(m)
        self.method = "fallback"     # "osm" or "fallback"
        self.rel_id = None
        self.rel_name = None
        self.coverage = None
        self.n_segments = 0
        self.n_failed = 0
        self.straight_avg = None

    def __repr__(self):
        avg = self.average if self.average is not None else 0.0
        return (
            f"{self.line_cd} {self.line_name}: {self.method} avg={avg:.1f} "
            f"rel={self.rel_name} fail={self.n_failed}/{self.n_segments}"
        )


def compute_line(line_cd: str, line_name: str, line_type: str, coords) -> LineResult:
    """1 路線の平均駅間距離を算出する。OSM 実距離を優先し、不可なら従来式へフォールバックする。"""
    res = LineResult(line_cd, line_name)
    straight = [
        haversine(coords[i][0], coords[i][1], coords[i + 1][0], coords[i + 1][1])
        for i in range(len(coords) - 1)
    ]
    res.n_segments = len(straight)
    res.straight_avg = sum(straight) / len(straight) if straight else None

    matched = None
    try:
        matched = find_best_relation(coords, line_name)
    except Exception as e:  # noqa: BLE001
        sys.stderr.write(f"  {line_cd} {line_name}: relation lookup error {e!r}\n")

    if matched is not None:
        rel, ways, cov, mx = matched
        adj = build_graph(ways)
        snapped = [snap_anchors(ways, c[0], c[1])[0] for c in coords]
        seg_dist = []
        failed = 0
        for i in range(len(coords) - 1):
            sd = straight[i]
            rd = route_anchors(
                adj, snapped[i], snapped[i + 1], max(sd * MAX_RATIO, sd + 5000)
            )
            if rd is None or rd > sd * MAX_RATIO or rd < sd * MIN_RATIO:
                failed += 1
                # 経路探索できない区間は旧来式(直線距離 x 固定係数)で軌道距離を推定する。
                # 直線距離をそのまま使うと実距離の下限になり、平均が過小評価されるため。
                factor = FALLBACK_FACTOR.get(line_type, 1.15)
                seg_dist.append(sd * factor)
            else:
                # 実距離は直線距離を下回らない（駅は線路から横にずれているため
                # 投影間の線路沿い距離が僅かに直線を下回ることがあるが、物理的下限で丸める）
                seg_dist.append(max(rd, sd))
        # 半数以上失敗した路線は信頼できないのでフォールバック扱いにする
        if seg_dist and failed <= len(seg_dist) / 2:
            res.average = sum(seg_dist) / len(seg_dist)
            res.method = "osm"
            res.rel_id = rel["id"]
            res.rel_name = rel.get("tags", {}).get("name")
            res.coverage = cov
            res.n_failed = failed
            return res

    # --- フォールバック: 旧来式（直線距離 × 固定係数） ---
    if res.straight_avg is not None:
        factor = FALLBACK_FACTOR.get(line_type, 1.15)
        res.average = res.straight_avg * factor
    res.method = "fallback"
    return res


# ---------------------------------------------------------------------------
# CSV 書き換え
# ---------------------------------------------------------------------------
def format_distance(value: float) -> str:
    """距離値を既存 CSV と同程度の桁数の文字列に整形する。"""
    # 既存CSVは小数を含む（例: 31785.47337）。同程度の桁で出力する。
    return f"{value:.5f}".rstrip("0").rstrip(".")


def apply_to_csv(results: dict[str, LineResult]) -> int:
    """算出結果で data/2!lines.csv の average_distance 列だけを書き換え、更新セル数を返す。"""
    with open(LINES_CSV, newline="", encoding="utf-8") as f:
        reader = csv.reader(f)
        rows = list(reader)
    header = rows[0]
    cd_idx = header.index("line_cd")
    avg_idx = header.index("average_distance")

    changed = 0
    for row in rows[1:]:
        if len(row) <= avg_idx:
            continue
        cd = row[cd_idx]
        r = results.get(cd)
        if r is None or r.average is None:
            continue
        new_val = format_distance(r.average)
        if row[avg_idx] != new_val:
            row[avg_idx] = new_val
            changed += 1

    with open(LINES_CSV, "w", newline="", encoding="utf-8") as f:
        writer = csv.writer(f)
        writer.writerows(rows)
    return changed


# ---------------------------------------------------------------------------
# メイン
# ---------------------------------------------------------------------------
def run(line_cds: list[str], lines_meta: dict[str, dict], by_line: dict) -> dict[str, LineResult]:
    """指定した line_cd 群を順に計算し、line_cd -> LineResult の辞書を返す。"""
    results: dict[str, LineResult] = {}
    total = len(line_cds)
    for i, cd in enumerate(line_cds, 1):
        meta = lines_meta.get(cd)
        if meta is None:
            continue
        sts = by_line.get(cd, [])
        if len(sts) < 2:
            continue
        coords = line_station_coords(sts)
        res = compute_line(cd, meta["line_name"], meta["line_type"], coords)
        results[cd] = res
        sys.stderr.write(
            f"[{i}/{total}] {cd} {meta['line_name']}: {res.method} "
            f"avg={res.average:.1f} "
            f"(straight={res.straight_avg:.1f}) "
            f"rel={res.rel_name} fail={res.n_failed}/{res.n_segments}\n"
        )
        sys.stderr.flush()
        time.sleep(0.3)  # ポライトな間隔
    return results


def main() -> int:
    """CLI エントリポイント。--validate / --lines / --apply を切り替える。"""
    ap = argparse.ArgumentParser(description=__doc__)
    g = ap.add_mutually_exclusive_group()
    g.add_argument("--validate", action="store_true", help="既知路線で較正のみ（CSV不変）")
    g.add_argument("--apply", action="store_true", help="全路線を計算しCSVを書き換える")
    g.add_argument("--lines", type=str, help="カンマ区切りの line_cd を計算（CSV不変）")
    args = ap.parse_args()

    lines = load_lines()
    lines_meta = {line["line_cd"]: line for line in lines}
    by_line = load_stations_by_line()

    if args.validate:
        targets = list(KNOWN.keys())
        results = run(targets, lines_meta, by_line)
        print("\n=== 較正結果 ===")
        for cd in targets:
            r = results.get(cd)
            if not r:
                continue
            known = KNOWN[cd][1]
            err = "" if known is None else f" / 実測~{known} (誤差 {(r.average - known) / known * 100:+.1f}%)"
            print(f"{cd} {r.line_name}: {r.method} avg={r.average:.1f}{err} fail={r.n_failed}/{r.n_segments}")
        return 0

    if args.lines:
        targets = [c.strip() for c in args.lines.split(",") if c.strip()]
        results = run(targets, lines_meta, by_line)
        print("\n=== 結果 ===")
        for cd in targets:
            r = results.get(cd)
            if r:
                print(f"{cd} {r.line_name}: {r.method} avg={r.average:.1f} rel={r.rel_name} fail={r.n_failed}/{r.n_segments}")
        return 0

    if args.apply:
        active = [
            line
            for line in lines
            if line["e_status"] == "0" and len(by_line.get(line["line_cd"], [])) >= 2
        ]
        targets = [line["line_cd"] for line in active]
        results = run(targets, lines_meta, by_line)
        n_osm = sum(1 for r in results.values() if r.method == "osm")
        n_fb = sum(1 for r in results.values() if r.method == "fallback")
        changed = apply_to_csv(results)
        print("\n=== 適用レポート ===")
        print(f"対象路線: {len(results)}")
        print(f"  OSM実距離で算出: {n_osm}")
        print(f"  フォールバック(直線×係数): {n_fb}")
        print(f"CSV更新セル数: {changed}")
        return 0

    ap.print_help()
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
