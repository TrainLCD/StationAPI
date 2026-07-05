#!/usr/bin/env python3
"""
公開 GTFS 時刻表から路線 × 列車種別ごとの実効最高速度を較正し、
stationapi/src/domain/speed_table.rs の自動生成ブロックを更新するスクリプト。

到着時間推定(arrival_estimation.rs)の運動学モデルを Python で忠実に再現し、
GTFS の実ダイヤ所要時間を再現する実効最高速度 v_max を路線 × 種別ごとに
二分探索でフィッティングする。一般則(路線種別の基本速度 × 種別倍率)から
大きく外れる路線だけをテーブルへ出力する。

データソース: 公共交通オープンデータセンター (https://ckan.odpt.org/) ほかの
             GTFS フィード。各フィードのライセンスは FEEDS の license を参照。
             出典明示が必要なもの(公共交通オープンデータ基本ライセンス等)を
             利用する場合はアプリ側のクレジット表記に追加すること。

使い方:
    # フィードを取得して較正結果を表示する(ファイルは書き換えない)
    python3 scripts/compute_speed_table.py --validate

    # speed_table.rs の自動生成ブロックを書き換える
    python3 scripts/compute_speed_table.py --apply

認証が必要なフィード(公共交通オープンデータセンターの大半)を使うには、
https://developer.odpt.org/ で発行した無料のアクセストークンを環境変数
ODPT_ACCESS_TOKEN に設定する。未設定の場合、該当フィードはスキップされる。

取得した GTFS は scripts/.gtfs_cache/ にキャッシュされ、再実行が速くなる。
"""
from __future__ import annotations

import argparse
import csv
import io
import math
import os
import re
import statistics
import sys
import unicodedata
import urllib.request
import zipfile
from dataclasses import dataclass

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
LINES_CSV = os.path.join(ROOT, "data", "2!lines.csv")
STATIONS_CSV = os.path.join(ROOT, "data", "3!stations.csv")
TYPES_CSV = os.path.join(ROOT, "data", "4!types.csv")
SST_CSV = os.path.join(ROOT, "data", "5!station_station_types.csv")
SPEED_TABLE_RS = os.path.join(ROOT, "stationapi", "src", "domain", "speed_table.rs")
CACHE_DIR = os.path.join(HERE, ".gtfs_cache")

HTTP_TIMEOUT = 120
USER_AGENT = "StationAPI-speed-table/1.0 (railway data maintenance)"


# ---------------------------------------------------------------------------
# フィード定義
# ---------------------------------------------------------------------------
@dataclass(frozen=True)
class Feed:
    key: str                # キャッシュファイル名などに使う識別子
    name: str               # 表示名
    url: str                # GTFS zip の URL({token} がトークンに置換される)
    needs_token: bool       # 公共交通オープンデータセンターのアクセストークン要否
    license: str            # ライセンスメモ(出典明示の要否確認用)


FEEDS: list[Feed] = [
    Feed(
        key="hakodate_tram",
        name="函館市電",
        url="https://api-public.odpt.org/api/v4/files/odpt/HakodateCity/Alllines.zip?date=20260615",
        needs_token=False,
        license="公共交通オープンデータセンター(認証なし公開)",
    ),
    Feed(
        key="kyoto_subway",
        name="京都市営地下鉄",
        url="https://api.odpt.org/api/v4/files/odpt/KyotoMunicipalTransportation/Kyoto_City_Subway_GTFS.zip?date=20260703&acl:consumerKey={token}",
        needs_token=True,
        license="公共交通オープンデータ基本ライセンス(要出典明示)",
    ),
    Feed(
        key="yokohama_subway",
        name="横浜市営地下鉄",
        url="https://api.odpt.org/api/v4/files/odpt/YokohamaMunicipal/Train.zip?date=20251226&acl:consumerKey={token}",
        needs_token=True,
        license="公共交通オープンデータ基本ライセンス(要出典明示)",
    ),
]

# ---------------------------------------------------------------------------
# arrival_estimation.rs の運動学モデルの再現(定数は Rust 側と一致させること)
# ---------------------------------------------------------------------------
ACCEL = 0.7
DECEL = 0.9
DWELL_MIN = 0.6
RUN_MARGIN = 1.15
PASS_PENALTY_SEC = 3.0
DETOUR_MIN = 1.0
RAIL_DETOUR_MAX = 1.35

LT_SHINKANSEN, LT_SUBWAY, LT_TRAM, LT_AGT, LT_CABLE = 1, 3, 4, 5, 0

BASE_SPEED = {LT_SHINKANSEN: 250.0, LT_SUBWAY: 75.0, LT_TRAM: 40.0, LT_AGT: 60.0, LT_CABLE: 12.0}
KIND_MULT = {3: 1.15, 4: 1.2, 5: 1.5}
FALLBACK_DETOUR = {LT_SHINKANSEN: 1.15, LT_SUBWAY: 1.20, LT_TRAM: 1.40, LT_AGT: 1.20, LT_CABLE: 1.10}

KIND_NAMES = {
    0: "Default", 1: "Branch", 2: "Rapid", 3: "Express",
    4: "LimitedExpress", 5: "HighSpeedRapid", 6: "CommuterRapid",
}

# フィッティング結果が一般則からこの比率以上離れた場合のみテーブルへ出力する。
EMIT_THRESHOLD = 0.10
# 較正に必要な最小サンプル(列車)数。
MIN_TRIPS = 3
# フィット結果の妥当範囲(km/h)。範囲外は駅マッチング異常とみなして捨てる。
V_MIN, V_MAX = 15.0, 220.0
# 駅名 + 座標マッチングの許容距離(m)。
MATCH_RADIUS_M = 500.0
# 駅名が一致しない場合に座標最近傍で救済する許容距離(m)。改称駅・表記揺れ対策。
NEAREST_RADIUS_M = 200.0
# 列車の停車駅のうちマッチ必須の割合。
MATCH_COVERAGE = 0.9
# 日中ダイヤとして採用する始発時刻の範囲(時)。
DAYTIME_HOURS = (9.5, 16.5)


def base_speed(line_type: int | None) -> float:
    return BASE_SPEED.get(line_type, 80.0)


def general_rule_speed(line_type: int | None, kind: int) -> float:
    if line_type == LT_SHINKANSEN:
        return base_speed(line_type)
    return base_speed(line_type) * KIND_MULT.get(kind, 1.0)


def haversine(lat1: float, lon1: float, lat2: float, lon2: float) -> float:
    r = 6371000.0
    p1, p2 = math.radians(lat1), math.radians(lat2)
    dphi = math.radians(lat2 - lat1)
    dlmb = math.radians(lon2 - lon1)
    a = math.sin(dphi / 2) ** 2 + math.cos(p1) * math.cos(p2) * math.sin(dlmb / 2) ** 2
    return 2 * r * math.asin(math.sqrt(a))


def seg_run_min(dist_m: float, v_kmh: float) -> float:
    """停車駅間の走行時間(分)。segment_run_minutes の再現。"""
    if dist_m <= 0 or v_kmh <= 0:
        return 0.0
    v = v_kmh / 3.6
    d_acc, d_dec = v * v / (2 * ACCEL), v * v / (2 * DECEL)
    if dist_m >= d_acc + d_dec:
        sec = v / ACCEL + v / DECEL + (dist_m - d_acc - d_dec) / v
    else:
        vp = math.sqrt(2 * dist_m * ACCEL * DECEL / (ACCEL + DECEL))
        sec = vp / ACCEL + vp / DECEL
    return sec * RUN_MARGIN / 60.0


def model_minutes(seq: list[tuple[float, bool]], v_kmh: float) -> float:
    """駅列 seq = [(前駅からのみなし走行距離m, 停車するか), ...] の総所要時間(分)。

    estimate_arrival_minutes の停車間セグメント積み上げを再現する。
    始発駅は seq[0](距離 0, 停車)。終着駅の到着までを返す(始発の dwell なし)。
    """
    total = 0.0
    seg: list[tuple[float, bool]] = []
    n = len(seq)
    for i in range(1, n):
        seg.append(seq[i])
        if seq[i][1]:
            if len(seg) == 1:
                total += seg_run_min(seg[0][0], v_kmh)
            else:
                v = v_kmh / 3.6
                sec = v / (2 * ACCEL) + v / (2 * DECEL)
                for dist_m, stops in seg:
                    sec += dist_m / v
                    if not stops:
                        sec += PASS_PENALTY_SEC
                total += sec * RUN_MARGIN / 60.0
            if i != n - 1:
                total += DWELL_MIN
            seg = []
    return total


# ---------------------------------------------------------------------------
# 自リポジトリのデータ読み込み
# ---------------------------------------------------------------------------
def norm_name(s: str) -> str:
    """駅名比較用の正規化。全角半角・空白・括弧書き・「駅」「前」等の揺れを吸収する。"""
    s = unicodedata.normalize("NFKC", s or "").strip()
    s = re.sub(r"[\s・]", "", s)
    s = re.sub(r"[((].*?[))]", "", s)  # 「(第2旅客ターミナル)」等の注記
    s = s.replace("ヶ", "ケ").replace("ノ", "の")
    s = re.sub(r"(停留場|停留所|駅前|駅)$", "", s)
    return s


def load_repo_data():
    lines = {}
    with open(LINES_CSV, newline="", encoding="utf-8") as f:
        for r in csv.DictReader(f):
            lines[int(r["line_cd"])] = {
                "name": unicodedata.normalize("NFKC", r["line_name"]).strip(),
                "line_type": int(r["line_type"]) if r["line_type"] else None,
                "avg_dist_m": float(r["average_distance"]) if r.get("average_distance") else 0.0,
                "e_status": r["e_status"],
            }

    by_line: dict[int, list[dict]] = {}
    with open(STATIONS_CSV, newline="", encoding="utf-8") as f:
        for r in csv.DictReader(f):
            if r["e_status"] != "0":
                continue
            s = {
                "cd": int(r["station_cd"]),
                "name": r["station_name"],
                "norm": norm_name(r["station_name"]),
                "line_cd": int(r["line_cd"]),
                "lat": float(r["lat"]),
                "lon": float(r["lon"]),
                "e_sort": int(r["e_sort"]),
            }
            by_line.setdefault(s["line_cd"], []).append(s)
    for sts in by_line.values():
        sts.sort(key=lambda s: (s["e_sort"], s["cd"]))

    kinds = {}
    with open(TYPES_CSV, newline="", encoding="utf-8") as f:
        for r in csv.DictReader(f):
            kinds[int(r["type_cd"])] = int(r["kind"]) if r["kind"] else 0

    # line_group_cd ごとの (station_cd, type_cd, pass) 列(格納順)。
    groups: dict[int, list[tuple[int, int, int]]] = {}
    with open(SST_CSV, newline="", encoding="utf-8") as f:
        for r in csv.DictReader(f):
            if not r["line_group_cd"]:
                continue
            groups.setdefault(int(r["line_group_cd"]), []).append(
                (int(r["station_cd"]), int(r["type_cd"]), int(r["pass"]) if r["pass"] else 0)
            )
    return lines, by_line, kinds, groups


def line_detour(lines: dict, by_line: dict, line_cd: int) -> float:
    """路線全体で較正した迂回係数 α(estimate_arrival_minutes_calibrated と同等)。"""
    li = lines[line_cd]
    sts = by_line.get(line_cd, [])
    gaps = [
        haversine(sts[i - 1]["lat"], sts[i - 1]["lon"], sts[i]["lat"], sts[i]["lon"])
        for i in range(1, len(sts))
    ]
    mean = sum(gaps) / len(gaps) if gaps else 0.0
    avg = li["avg_dist_m"]
    if avg > 0 and mean > 0:
        return min(max(avg / mean, DETOUR_MIN), RAIL_DETOUR_MAX)
    return FALLBACK_DETOUR.get(li["line_type"], 1.30)


# ---------------------------------------------------------------------------
# GTFS 取得・読み込み
# ---------------------------------------------------------------------------
def fetch_feed(feed: Feed, token: str | None) -> zipfile.ZipFile | None:
    os.makedirs(CACHE_DIR, exist_ok=True)
    cache = os.path.join(CACHE_DIR, f"{feed.key}.zip")
    if not os.path.exists(cache):
        url = feed.url
        if feed.needs_token:
            if not token:
                sys.stderr.write(
                    f"  {feed.name}: ODPT_ACCESS_TOKEN 未設定のためスキップ"
                    "(https://developer.odpt.org/ で無料発行できます)\n"
                )
                return None
            url = url.format(token=token)
        req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
        with urllib.request.urlopen(req, timeout=HTTP_TIMEOUT) as resp:
            data = resp.read()
        with open(cache, "wb") as f:
            f.write(data)
    return zipfile.ZipFile(cache)


def read_gtfs_csv(zf: zipfile.ZipFile, name: str) -> list[dict]:
    try:
        raw = zf.read(name)
    except KeyError:
        return []
    text = raw.decode("utf-8-sig")
    return list(csv.DictReader(io.StringIO(text)))


def parse_gtfs_time(t: str) -> float | None:
    """GTFS の HH:MM:SS(24 時超えあり)を分に変換する。"""
    m = re.match(r"^(\d+):(\d+):(\d+)$", (t or "").strip())
    if not m:
        return None
    h, mi, s = map(int, m.groups())
    return h * 60 + mi + s / 60.0


def weekday_service_ids(zf: zipfile.ZipFile) -> set[str]:
    """平日(月〜金すべて運行)の service_id 集合。calendar.txt が無ければ全 service。"""
    cal = read_gtfs_csv(zf, "calendar.txt")
    if not cal:
        trips = read_gtfs_csv(zf, "trips.txt")
        return {t["service_id"] for t in trips}
    return {
        c["service_id"]
        for c in cal
        if all(c.get(d) == "1" for d in ("monday", "tuesday", "wednesday", "thursday", "friday"))
    }


# ---------------------------------------------------------------------------
# 較正本体
# ---------------------------------------------------------------------------
def find_station(sts_line: list[dict], nm: str, lat: float, lon: float) -> int | None:
    """路線の駅リストから停留所に対応する駅 index を返す。

    正規化名一致(座標 500m 以内)を優先し、無ければ座標最近傍(200m 以内)で
    救済する(改称駅・「アリーナ前」⇔「函館アリーナ前」のような表記揺れ対策)。
    """
    for i, s in enumerate(sts_line):
        if s["norm"] == nm and haversine(lat, lon, s["lat"], s["lon"]) <= MATCH_RADIUS_M:
            return i
    best, best_d = None, NEAREST_RADIUS_M
    for i, s in enumerate(sts_line):
        d = haversine(lat, lon, s["lat"], s["lon"])
        if d <= best_d:
            best, best_d = i, d
    return best


def match_line(
    trip_stop_keys: list[tuple[str, float, float]],
    by_line: dict,
    lines: dict,
) -> int | None:
    """列車の停車駅列(正規化名, 緯度, 経度)から最も整合する line_cd を返す。"""
    best, best_hits = None, 0
    for line_cd, sts in by_line.items():
        if lines.get(line_cd, {}).get("e_status") != "0":
            continue
        hits = sum(1 for nm, lat, lon in trip_stop_keys if find_station(sts, nm, lat, lon) is not None)
        if hits > best_hits:
            best, best_hits = line_cd, hits
    if best is not None and best_hits >= max(2, math.ceil(len(trip_stop_keys) * MATCH_COVERAGE)):
        return best
    return None


def classify_kind(
    line_cd: int,
    served_cds: set[int],
    span_cds: list[int],
    groups: dict,
    kinds: dict,
) -> int | None:
    """列車の停車パターンを station_station_types の種別グループと照合し kind を返す。

    全駅停車なら kind=0(Default)。通過があるのにどのグループとも一致しない場合は
    None(較正対象外)。照合は列車の走行区間(span_cds)内の停車集合の一致で行う。
    """
    span_set = set(span_cds)
    if served_cds == span_set:
        return 0
    best_kind, best_score = None, 0.0
    for members in groups.values():
        member_cds = {cd for cd, _, _ in members}
        if not span_set <= member_cds:
            continue
        group_stops = {cd for cd, _, ps in members if ps == 0 and cd in span_set}
        union = group_stops | served_cds
        if not union:
            continue
        score = len(group_stops & served_cds) / len(union)
        if score > best_score:
            best_score = score
            type_cd = members[0][1]
            best_kind = kinds.get(type_cd, 0)
    return best_kind if best_score >= 0.9 else None


def fit_v_max(samples: list[tuple[list[tuple[float, bool]], float]]) -> float | None:
    """観測所要時間(分)の中央値残差が 0 になる v_max を二分探索する。"""

    def residual(v: float) -> float:
        return statistics.median(model_minutes(seq, v) - obs for seq, obs in samples)

    lo, hi = V_MIN, V_MAX
    if residual(lo) < 0:  # 最低速度でもモデルが速すぎる(異常データ)
        return None
    if residual(hi) > 0:  # 最高速度でもモデルが遅すぎる(異常データ)
        return None
    for _ in range(60):
        mid = (lo + hi) / 2
        if residual(mid) > 0:
            lo = mid
        else:
            hi = mid
    return (lo + hi) / 2


@dataclass
class Calibration:
    line_cd: int
    line_name: str
    kind: int
    v_fit: float
    v_rule: float
    n_trips: int
    median_obs: float
    feed: str


def calibrate_feed(feed: Feed, zf: zipfile.ZipFile, lines, by_line, kinds, groups) -> list[Calibration]:
    stops_by_id = {
        s["stop_id"]: (norm_name(s["stop_name"]), float(s["stop_lat"]), float(s["stop_lon"]))
        for s in read_gtfs_csv(zf, "stops.txt")
        if s.get("stop_lat") and s.get("stop_lon")
    }
    services = weekday_service_ids(zf)
    trips = [t for t in read_gtfs_csv(zf, "trips.txt") if t["service_id"] in services]
    trip_ids = {t["trip_id"] for t in trips}

    st_by_trip: dict[str, list[dict]] = {}
    for st in read_gtfs_csv(zf, "stop_times.txt"):
        if st["trip_id"] in trip_ids:
            st_by_trip.setdefault(st["trip_id"], []).append(st)
    for v in st_by_trip.values():
        v.sort(key=lambda r: int(r["stop_sequence"]))

    # (line_cd, kind) -> [(モデル入力列, 観測分), ...](同一パターン・同一所要は重複排除)
    samples: dict[tuple[int, int], dict[tuple, tuple]] = {}
    line_match_cache: dict[tuple, int | None] = {}

    for trip_id, sts in st_by_trip.items():
        # 乗降可能な停車のみ(pickup/drop_off とも 1=不可 の通過レコードを除外)。
        stop_rows = [
            r for r in sts
            if not (r.get("pickup_type") == "1" and r.get("drop_off_type") == "1")
        ]
        if len(stop_rows) < 4:
            continue
        dep0 = parse_gtfs_time(stop_rows[0].get("departure_time", ""))
        arr_last = parse_gtfs_time(stop_rows[-1].get("arrival_time", ""))
        if dep0 is None or arr_last is None or arr_last <= dep0:
            continue
        if not (DAYTIME_HOURS[0] * 60 <= dep0 % 1440 <= DAYTIME_HOURS[1] * 60):
            continue

        keys = []
        ok = True
        for r in stop_rows:
            info = stops_by_id.get(r["stop_id"])
            if info is None:
                ok = False
                break
            keys.append(info)
        if not ok:
            continue

        pattern = tuple(k[0] for k in keys)
        if pattern not in line_match_cache:
            line_match_cache[pattern] = match_line(keys, by_line, lines)
        line_cd = line_match_cache[pattern]
        if line_cd is None:
            continue

        # 停車駅を自データの駅へ対応付ける。
        sts_line = by_line[line_cd]
        matched_idx: list[int] = []
        for nm, lat, lon in keys:
            found = find_station(sts_line, nm, lat, lon)
            if found is None:
                break
            matched_idx.append(found)
        if len(matched_idx) != len(keys):
            continue
        # 進行方向が単調な列車だけを使う(環状・折返しは除外)。
        inc = all(b > a for a, b in zip(matched_idx, matched_idx[1:]))
        dec = all(b < a for a, b in zip(matched_idx, matched_idx[1:]))
        if not (inc or dec):
            continue

        lo, hi = min(matched_idx), max(matched_idx)
        span = sts_line[lo:hi + 1]
        if dec:
            span = list(reversed(span))
        served = {sts_line[i]["cd"] for i in matched_idx}
        kind = classify_kind(line_cd, served, [s["cd"] for s in span], groups, kinds)
        if kind is None:
            continue

        alpha = line_detour(lines, by_line, line_cd)
        seq: list[tuple[float, bool]] = [(0.0, True)]
        for a, b in zip(span, span[1:]):
            d = haversine(a["lat"], a["lon"], b["lat"], b["lon"]) * alpha
            seq.append((d, b["cd"] in served))
        obs = arr_last - dep0

        dedup_key = (tuple(s["cd"] for s in span), tuple(sorted(served)), round(obs, 1))
        samples.setdefault((line_cd, kind), {})[dedup_key] = (seq, obs)

    results = []
    for (line_cd, kind), dd in sorted(samples.items()):
        trips_list = list(dd.values())
        if len(trips_list) < MIN_TRIPS:
            sys.stderr.write(
                f"  {lines[line_cd]['name']} kind={kind}: サンプル {len(trips_list)} 本 < {MIN_TRIPS} → 見送り\n"
            )
            continue
        v = fit_v_max(trips_list)
        if v is None or not (V_MIN < v < V_MAX):
            sys.stderr.write(f"  {lines[line_cd]['name']} kind={kind}: フィット失敗 → 見送り\n")
            continue
        results.append(Calibration(
            line_cd=line_cd,
            line_name=lines[line_cd]["name"],
            kind=kind,
            v_fit=round(v / 5) * 5,  # 5km/h 刻みに丸めて出力の安定性を上げる
            v_rule=general_rule_speed(lines[line_cd]["line_type"], kind),
            n_trips=len(trips_list),
            median_obs=statistics.median(o for _, o in trips_list),
            feed=feed.name,
        ))
    return results


# ---------------------------------------------------------------------------
# speed_table.rs の自動生成ブロック書き換え
# ---------------------------------------------------------------------------
BEGIN_MARK = "// --- BEGIN GENERATED (scripts/compute_speed_table.py) ---"
END_MARK = "// --- END GENERATED ---"


def manual_keys_in_rust(src: str) -> set[tuple[int, str]]:
    """手動較正テーブル(自動生成ブロック外)に既にある (line_cd, kind名) を集める。"""
    gen_start = src.find(BEGIN_MARK)
    manual_src = src if gen_start < 0 else src[:gen_start]
    return {
        (int(m.group(1)), m.group(2))
        for m in re.finditer(r"\(\s*(\d+)\s*,\s*TrainTypeKind::(\w+)\s*,", manual_src)
    }


def apply_to_rust(results: list[Calibration]) -> int:
    with open(SPEED_TABLE_RS, encoding="utf-8") as f:
        src = f.read()
    start = src.find(BEGIN_MARK)
    end = src.find(END_MARK)
    if start < 0 or end < 0 or end < start:
        raise SystemExit(f"{SPEED_TABLE_RS} に生成ブロックマーカーが見つかりません")

    manual = manual_keys_in_rust(src)
    rows = []
    skipped = 0
    for c in sorted(results, key=lambda c: (c.line_cd, c.kind)):
        kind_name = KIND_NAMES[c.kind]
        if (c.line_cd, kind_name) in manual:
            sys.stderr.write(
                f"  {c.line_name} kind={kind_name}: 手動テーブルに既存エントリがあるためスキップ\n"
            )
            skipped += 1
            continue
        rows.append(
            f"    // {c.line_name} {kind_name}: {c.feed} GTFS {c.n_trips}本 "
            f"中央値{c.median_obs:.0f}分 (一般則 {c.v_rule:.0f}km/h)\n"
            f"    ({c.line_cd}, TrainTypeKind::{kind_name}, {c.v_fit:.1f}),\n"
        )

    body = "".join(rows) if rows else "    // (現在エントリなし)\n"
    # BEGIN マーカー行の直後から END マーカー行(インデント込み)の直前までを置換する。
    line_start = src.rfind("\n", 0, end) + 1
    new_src = src[: start + len(BEGIN_MARK)] + "\n" + body + src[line_start:]
    if new_src != src:
        with open(SPEED_TABLE_RS, "w", encoding="utf-8") as f:
            f.write(new_src)
    return len(rows)


# ---------------------------------------------------------------------------
# メイン
# ---------------------------------------------------------------------------
def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--validate", action="store_true", help="較正のみ(ファイル不変)")
    g.add_argument("--apply", action="store_true", help="speed_table.rs の生成ブロックを書き換える")
    args = ap.parse_args()

    token = os.environ.get("ODPT_ACCESS_TOKEN")
    lines, by_line, kinds, groups = load_repo_data()

    all_results: list[Calibration] = []
    for feed in FEEDS:
        sys.stderr.write(f"[{feed.key}] {feed.name} ({feed.license})\n")
        try:
            zf = fetch_feed(feed, token)
        except Exception as e:  # noqa: BLE001 - フィード単位で続行する
            sys.stderr.write(f"  取得失敗: {e!r} → スキップ\n")
            continue
        if zf is None:
            continue
        all_results.extend(calibrate_feed(feed, zf, lines, by_line, kinds, groups))

    print(f"\n{'路線':<24} {'kind':<16} {'フィット':>8} {'一般則':>7} {'乖離':>7} {'本数':>4} {'採用':>4}")
    emitted = []
    for c in sorted(all_results, key=lambda c: (c.line_cd, c.kind)):
        dev = c.v_fit / c.v_rule - 1.0
        emit = abs(dev) >= EMIT_THRESHOLD
        if emit:
            emitted.append(c)
        print(
            f"{c.line_name:<24} {KIND_NAMES[c.kind]:<16} {c.v_fit:6.0f}km {c.v_rule:5.0f}km "
            f"{dev:+7.1%} {c.n_trips:>4} {'○' if emit else '-':>4}"
        )

    if args.apply:
        n = apply_to_rust(emitted)
        print(f"\nspeed_table.rs 生成ブロックを更新: {n} エントリ")
    else:
        print(f"\n出力対象(一般則から ±{EMIT_THRESHOLD:.0%} 以上乖離): {len(emitted)} エントリ(--apply で書き込み)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
