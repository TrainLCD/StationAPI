#!/usr/bin/env python3
"""GraphQL エンドポイントを 2 つ並べて計測し、応答の食い違いも検出する。

schema/public.graphql の Query 型にあるルートフィールドを全て 1 件ずつ扱う。
既定では本番 (gql.trainlcd.app) とステージング (gql-stg.trainlcd.app) を比べる。

  python3 scripts/benchmark_graphql.py --iterations 30          # 速度を測る
  python3 scripts/benchmark_graphql.py --parity                 # 応答の差を調べる

依存は Python 3.9 以降の標準ライブラリのみ。pip での用意は要らない。

計測値は測定地点に強く依存する。上流へ往復する構成を相手にする場合、
計測地から上流までの距離がそのまま相手側の下駄になるため、公表する数字は
利用者のいる地域から測ること。GraphQL を通さない `GET /__ping` を併せて測るので、
エッジまでの往復とサーバー側の処理時間は切り分けられる。
"""

from __future__ import annotations

import argparse
import gzip
import http.client
import json
import os
import platform
import socket
import statistics
import sys
import time
import urllib.parse
from dataclasses import dataclass, field
from datetime import datetime, timezone

DEFAULT_ENDPOINTS = {
    "production": "https://gql.trainlcd.app/",
    "staging": "https://gql-stg.trainlcd.app/",
}

# 計測に使う実データ。存在しない ID を渡すと空応答になり計測の意味が無くなるため、
# 実在する駅/路線 (渋谷・新宿・東京・山手線ほか) を使う。
SHIBUYA = 1130205  # 渋谷 (山手線)
SHIBUYA_GROUP = 1130205
SHINJUKU_GROUP = 1130208
TOKYO_YAMANOTE = 1130224  # 東京 (山手線)
YAMANOTE_LINE = 11302
YAMANOTE_LINE_GROUP = 363

# 駅の共通フィールド。クライアントが実際に引く程度の量を並べ、
# 応答生成のコストが測れるようにネストも 1 段含める。
STATION_FIELDS = """
  id groupId name nameKatakana nameRoman nameChinese nameKorean
  threeLetterCode prefectureId postalCode address latitude longitude
  openedAt closedAt status stopCondition distance hasTrainTypes transportType
  stationNumbers { lineSymbol lineSymbolColor lineSymbolShape stationNumber }
  line { id nameShort nameRoman color lineType status
         lineSymbols { symbol color shape }
         company { id nameShort nameEnglishShort type status } }
  lines { id nameShort nameRoman color lineType status }
  trainType { id typeId groupId name nameRoman color direction kind }
"""

LINE_FIELDS = """
  id nameShort nameKatakana nameFull nameRoman nameChinese nameKorean
  color lineType status averageDistance transportType
  lineSymbols { symbol color shape }
  company { id railroadId nameShort nameFull nameEnglishShort url type status }
  station { id groupId name nameRoman }
  trainType { id typeId groupId name nameRoman color direction kind }
"""

TRAIN_TYPE_FIELDS = """
  id typeId groupId name nameKatakana nameRoman nameChinese nameKorean
  color direction kind
  line { id nameShort nameRoman color }
  lines { id nameShort nameRoman color }
"""


@dataclass
class Case:
    """1 クエリ分の計測ケース。"""

    name: str
    query: str
    note: str = ""


def build_cases() -> list[Case]:
    """schema/public.graphql の Query 型を 1 フィールドずつ網羅する。"""
    return [
        Case(
            "station",
            "{ station(id: %d) { %s } }" % (SHIBUYA, STATION_FIELDS),
            "単一駅 (渋谷)",
        ),
        Case(
            "stations",
            "{ stations(ids: [%s]) { %s } }"
            % (
                ", ".join(str(i) for i in [1130205, 1130208, 1130101, 1130224, 1130105]),
                STATION_FIELDS,
            ),
            "駅 5 件の一括取得",
        ),
        Case(
            "stationsNearby",
            "{ stationsNearby(latitude: 35.658034, longitude: 139.701636, limit: 10) { %s } }"
            % STATION_FIELDS,
            "渋谷駅前から半径検索 10 件",
        ),
        Case(
            "stationsByName",
            '{ stationsByName(name: "新宿", limit: 10) { %s } }' % STATION_FIELDS,
            "駅名検索 (全件走査) 10 件",
        ),
        Case(
            "stationGroupStations",
            "{ stationGroupStations(groupId: %d) { %s } }" % (SHIBUYA_GROUP, STATION_FIELDS),
            "同一駅グループ (渋谷) の全乗り入れ",
        ),
        Case(
            "lineGroupStations",
            "{ lineGroupStations(lineGroupId: %d) { %s } }"
            % (YAMANOTE_LINE_GROUP, STATION_FIELDS),
            "系統 (山手線 普通) の全駅",
        ),
        Case(
            "line",
            "{ line(lineId: %d) { %s } }" % (YAMANOTE_LINE, LINE_FIELDS),
            "単一路線 (山手線)",
        ),
        Case(
            "lines",
            "{ lines(lineIds: [%s]) { %s } }"
            % (", ".join(str(i) for i in [11302, 11321, 26001, 28001, 24006]), LINE_FIELDS),
            "路線 5 件の一括取得",
        ),
        Case(
            "linesByName",
            '{ linesByName(name: "山手", limit: 10) { %s } }' % LINE_FIELDS,
            "路線名検索 (全件走査)",
        ),
        Case(
            "lineStations",
            "{ lineStations(lineId: %d) { %s } }" % (YAMANOTE_LINE, STATION_FIELDS),
            "路線 (山手線) の全駅",
        ),
        Case(
            "lineListStations",
            "{ lineListStations(lineIds: [%d, %d]) { %s } }"
            % (YAMANOTE_LINE, 26001, STATION_FIELDS),
            "複数路線の駅を一括取得",
        ),
        Case(
            "lineGroupListStations",
            "{ lineGroupListStations(lineGroupIds: [%d]) { %s } }"
            % (YAMANOTE_LINE_GROUP, STATION_FIELDS),
            "複数系統の駅を一括取得",
        ),
        Case(
            "stationTrainTypes",
            "{ stationTrainTypes(stationId: %d) { %s } }" % (SHIBUYA, TRAIN_TYPE_FIELDS),
            "駅 (渋谷) の種別一覧",
        ),
        Case(
            "routes",
            "{ routes(fromStationGroupId: %d, toStationGroupId: %d, pageSize: 10) "
            "{ nextPageToken routes { id stops { id groupId name nameRoman "
            "line { id nameShort color } trainType { id name kind } } } } }"
            % (SHIBUYA_GROUP, SHINJUKU_GROUP),
            "経路探索 渋谷→新宿 (10 件)",
        ),
        Case(
            "routeTypes",
            "{ routeTypes(fromStationGroupId: %d, toStationGroupId: %d, pageSize: 10) "
            "{ nextPageToken trainTypes { %s } } }"
            % (SHIBUYA_GROUP, SHINJUKU_GROUP, TRAIN_TYPE_FIELDS),
            "経路の種別一覧 渋谷→新宿",
        ),
        Case(
            "connectedRoutes",
            "{ connectedRoutes(fromStationGroupId: %d, toStationGroupId: %d) "
            "{ id stops { id groupId name nameRoman line { id nameShort color } } } }"
            % (SHIBUYA_GROUP, SHINJUKU_GROUP),
            "直通経路 渋谷→新宿",
        ),
        Case(
            "estimateArrivalTimes",
            "{ estimateArrivalTimes(fromStationId: %d, toStationId: %d) "
            "{ routes { id stops { stationId stationGroupId cumulativeMinutes "
            "stopsHere departureCumulativeMinutes } } } }" % (SHIBUYA, TOKYO_YAMANOTE),
            "到達時分の推定 渋谷→東京",
        ),
        Case(
            "trainRoute",
            "{ trainRoute(fromStationId: %d, toStationId: %d, lineGroupId: %d) "
            "{ segments { stops distanceFromPrevious maxSpeed maxAcceleration "
            "maxDeceleration station { id groupId name nameRoman } } } }"
            % (SHIBUYA, TOKYO_YAMANOTE, YAMANOTE_LINE_GROUP),
            "走行区間の生成 渋谷→東京",
        ),
    ]


# ---------------------------------------------------------------- HTTP


class TransportError(Exception):
    """接続や読み取りに失敗した (HTTP のステータス異常は含まない)。"""


class Endpoint:
    """1 エンドポイントへの keep-alive 接続。

    毎回つなぎ直すと TLS ハンドシェイクが計測に混ざるため、接続は使い回す。
    相手が idle でつないだ接続を切ることはあるので、送信時の失敗は 1 度だけ
    つなぎ直して再送する (再送も失敗したら諦める)。
    """

    def __init__(self, name: str, url: str, timeout: float):
        self.name = name
        self.url = url
        self.timeout = timeout
        parsed = urllib.parse.urlsplit(url)
        self.scheme = parsed.scheme or "https"
        self.host = parsed.hostname or ""
        self.port = parsed.port or (443 if self.scheme == "https" else 80)
        self.path = parsed.path or "/"
        self.proxy = self._proxy_from_env()
        self._conn: http.client.HTTPConnection | None = None

    def _proxy_from_env(self) -> tuple[str, int] | None:
        """環境変数のプロキシ設定を拾う。CI コンテナ越しでも同じ手順で測れるようにする。"""
        no_proxy = os.environ.get("no_proxy", os.environ.get("NO_PROXY", ""))
        for entry in (e.strip() for e in no_proxy.split(",")):
            if entry and (self.host == entry or self.host.endswith(entry.lstrip("*"))):
                return None
        key = "https_proxy" if self.scheme == "https" else "http_proxy"
        raw = os.environ.get(key, os.environ.get(key.upper(), ""))
        if not raw:
            return None
        parsed = urllib.parse.urlsplit(raw)
        if not parsed.hostname:
            return None
        return parsed.hostname, parsed.port or 8080

    def _connect(self) -> http.client.HTTPConnection:
        if self.proxy:
            proxy_host, proxy_port = self.proxy
            if self.scheme == "https":
                conn: http.client.HTTPConnection = http.client.HTTPSConnection(
                    proxy_host, proxy_port, timeout=self.timeout
                )
                conn.set_tunnel(self.host, self.port)
            else:
                conn = http.client.HTTPConnection(proxy_host, proxy_port, timeout=self.timeout)
        elif self.scheme == "https":
            conn = http.client.HTTPSConnection(self.host, self.port, timeout=self.timeout)
        else:
            conn = http.client.HTTPConnection(self.host, self.port, timeout=self.timeout)
        return conn

    def close(self) -> None:
        if self._conn is not None:
            self._conn.close()
            self._conn = None

    def _request(self, method: str, path: str, body: bytes | None, headers: dict):
        if self._conn is None:
            self._conn = self._connect()
        try:
            self._conn.request(method, path, body=body, headers=headers)
            return self._conn.getresponse().read(), self._conn
        except (http.client.HTTPException, OSError):
            self.close()
            raise

    def send(self, method: str, path: str, payload: bytes | None) -> tuple[float, int, bytes, int]:
        """(経過ミリ秒, ステータス, 展開後の本文, 転送バイト数) を返す。

        実クライアントに合わせて gzip を要求する。転送量は圧縮後、
        応答量は展開後で見たいので両方返す。
        """
        headers = {
            "content-type": "application/json",
            "accept": "application/json",
            "accept-encoding": "gzip",
            "user-agent": "stationapi-benchmark/1.0",
        }
        last_error: Exception | None = None
        for attempt in range(2):  # idle で切られた接続の張り直しに 1 回だけ付き合う
            started = time.perf_counter()
            try:
                if self._conn is None:
                    self._conn = self._connect()
                self._conn.request(method, path, body=payload, headers=headers)
                response = self._conn.getresponse()
                raw = response.read()
                elapsed_ms = (time.perf_counter() - started) * 1000
            except (http.client.HTTPException, OSError, socket.timeout) as exc:
                self.close()
                last_error = exc
                continue
            if response.getheader("content-encoding", "").lower() == "gzip":
                try:
                    body = gzip.decompress(raw)
                except OSError:
                    body = raw
            else:
                body = raw
            if response.getheader("connection", "").lower() == "close":
                self.close()
            return elapsed_ms, response.status, body, len(raw)
        raise TransportError(str(last_error))

    def graphql(self, query: str) -> tuple[float, int, bytes, int]:
        payload = json.dumps({"query": query}).encode("utf-8")
        return self.send("POST", self.path, payload)

    def ping(self) -> tuple[float, int, bytes, int]:
        base = self.path.rstrip("/")
        return self.send("GET", base + "/__ping", None)


# ---------------------------------------------------------------- 集計


@dataclass
class Samples:
    """1 (環境, ケース) の計測結果。"""

    durations_ms: list[float] = field(default_factory=list)
    statuses: list[int] = field(default_factory=list)
    sizes: list[int] = field(default_factory=list)
    wire_sizes: list[int] = field(default_factory=list)
    gql_errors: list[str] = field(default_factory=list)
    transport_errors: list[str] = field(default_factory=list)

    def percentile(self, pct: float) -> float:
        """試行回数が少ないので補間せず、順位で取る (悲観側に寄せる)。"""
        if not self.durations_ms:
            return float("nan")
        ordered = sorted(self.durations_ms)
        rank = int(-(-pct * len(ordered) // 100))  # 切り上げ
        return ordered[min(len(ordered), max(1, rank)) - 1]

    def summary(self) -> dict:
        d = self.durations_ms
        return {
            "n": len(d),
            "ok": sum(1 for s in self.statuses if s == 200) - len(self.gql_errors),
            "http_errors": sum(1 for s in self.statuses if s != 200),
            "gql_errors": len(self.gql_errors),
            "transport_errors": len(self.transport_errors),
            "min_ms": min(d) if d else float("nan"),
            "p50_ms": self.percentile(50),
            "p90_ms": self.percentile(90),
            "p95_ms": self.percentile(95),
            "p99_ms": self.percentile(99),
            "max_ms": max(d) if d else float("nan"),
            "mean_ms": statistics.fmean(d) if d else float("nan"),
            "stdev_ms": statistics.stdev(d) if len(d) > 1 else 0.0,
            "bytes": statistics.fmean(self.sizes) if self.sizes else 0.0,
            "wire_bytes": statistics.fmean(self.wire_sizes) if self.wire_sizes else 0.0,
            "first_gql_error": self.gql_errors[0] if self.gql_errors else "",
            "first_transport_error": (
                self.transport_errors[0] if self.transport_errors else ""
            ),
        }


def record(samples: Samples, elapsed_ms: float, status: int, body: bytes, wire: int) -> None:
    samples.durations_ms.append(elapsed_ms)
    samples.statuses.append(status)
    samples.sizes.append(len(body))
    samples.wire_sizes.append(wire)
    if status != 200:
        return
    try:
        payload = json.loads(body)
    except json.JSONDecodeError as exc:
        samples.gql_errors.append(f"JSON 解釈に失敗: {exc}")
        return
    if payload.get("errors"):
        samples.gql_errors.append(str(payload["errors"][0].get("message", ""))[:200])


# ---------------------------------------------------------------- 計測


def run(endpoints: dict[str, Endpoint], cases: list[Case], iterations: int,
        warmup: int) -> dict:
    results: dict[str, dict[str, Samples]] = {
        name: {case.name: Samples() for case in cases} for name in endpoints
    }
    order = list(endpoints)

    for case in cases:
        print(f"[{case.name}] warmup...", file=sys.stderr, flush=True)
        for name in order:
            for _ in range(warmup):
                try:
                    endpoints[name].graphql(case.query)
                except TransportError:
                    pass

        for i in range(iterations):
            # 反復ごとに環境の順番を入れ替え、順番による偏りを打ち消す。
            seq = order if i % 2 == 0 else list(reversed(order))
            for name in seq:
                try:
                    elapsed_ms, status, body, wire = endpoints[name].graphql(case.query)
                except TransportError as exc:
                    results[name][case.name].transport_errors.append(str(exc)[:200])
                    continue
                record(results[name][case.name], elapsed_ms, status, body, wire)
        done = {n: results[n][case.name].summary() for n in order}
        print(
            "  " + "  ".join(f"{n}: p50={done[n]['p50_ms']:.0f}ms" for n in order),
            file=sys.stderr,
            flush=True,
        )

    return {
        "cases": [
            {
                "name": case.name,
                "note": case.note,
                "query": " ".join(case.query.split()),
                "results": {n: results[n][case.name].summary() for n in order},
            }
            for case in cases
        ],
    }


def baseline(endpoints: dict[str, Endpoint], iterations: int) -> dict:
    """GraphQL を通さない疎通 (`GET /__ping`) の所要時間。

    エッジまでの往復にあたる。これを引けば残りがサーバー側の処理時間になるので、
    計測地点による下駄と処理時間の差を切り分けられる。
    """
    out = {}
    for name, endpoint in endpoints.items():
        samples = Samples()
        for _ in range(3):
            try:
                endpoint.ping()
            except TransportError:
                pass
        for _ in range(iterations):
            try:
                elapsed_ms, status, body, wire = endpoint.ping()
            except TransportError as exc:
                samples.transport_errors.append(str(exc)[:200])
                continue
            record(samples, elapsed_ms, status, body, wire)
        out[name] = samples.summary()
    return out


# ---------------------------------------------------------------- 応答の突き合わせ


def walk_diff(a, b, path: str, out: list[str], limit: int) -> None:
    """2 つの応答を突き合わせ、食い違う場所を最大 limit 件まで書き出す。"""
    if len(out) >= limit:
        return
    if type(a) is not type(b) and not (
        isinstance(a, (int, float)) and isinstance(b, (int, float))
    ):
        out.append(f"{path}: 型が違う ({type(a).__name__} / {type(b).__name__})")
        return
    if isinstance(a, dict):
        for key in sorted(set(a) | set(b)):
            if key not in a:
                out.append(f"{path}.{key}: 左に無い")
            elif key not in b:
                out.append(f"{path}.{key}: 右に無い")
            else:
                walk_diff(a[key], b[key], f"{path}.{key}", out, limit)
            if len(out) >= limit:
                return
    elif isinstance(a, list):
        if len(a) != len(b):
            out.append(f"{path}: 件数が違う ({len(a)} / {len(b)})")
            return
        for i, (x, y) in enumerate(zip(a, b)):
            walk_diff(x, y, f"{path}[{i}]", out, limit)
            if len(out) >= limit:
                return
    elif isinstance(a, float) or isinstance(b, float):
        # 浮動小数は表現差 (0 と 0.0 など) が出るので相対誤差で見る
        if abs(a - b) > max(1e-9, abs(a) * 1e-9):
            out.append(f"{path}: {a} / {b}")
    elif a != b:
        out.append(f"{path}: {a!r} / {b!r}")


def count_leaves(value) -> int:
    if isinstance(value, dict):
        return sum(count_leaves(v) for v in value.values())
    if isinstance(value, list):
        return sum(count_leaves(v) for v in value)
    return 1


def parity(endpoints: dict[str, Endpoint], cases: list[Case], limit: int) -> dict:
    """同じクエリの応答が 2 環境で一致するか調べる。

    速度の主張に「同じ仕事をしているのか」という反論が付くため、
    どのクエリが比較可能でどれが比較不能かを先に切り分ける。
    """
    names = list(endpoints)
    if len(names) != 2:
        raise SystemExit("--parity は 2 環境でのみ使えます")
    left, right = names
    out = []
    for case in cases:
        entry = {"name": case.name, "note": case.note}
        bodies = {}
        for name in names:
            try:
                _, status, body, _ = endpoints[name].graphql(case.query)
            except TransportError as exc:
                entry["error"] = f"{name}: {exc}"
                break
            if status != 200:
                entry["error"] = f"{name}: HTTP {status}"
                break
            bodies[name] = json.loads(body)
        if "error" in entry:
            out.append(entry)
            print(f"[{case.name}] {entry['error']}", file=sys.stderr, flush=True)
            continue

        errors = {n: bodies[n].get("errors") for n in names}
        if any(errors.values()):
            entry["gql_errors"] = {n: (e[0]["message"][:160] if e else None)
                                   for n, e in errors.items()}
        a, b = bodies[left].get("data"), bodies[right].get("data")
        diffs: list[str] = []
        walk_diff(a, b, "data", diffs, limit)
        entry["identical"] = not diffs
        entry["diff_count_shown"] = len(diffs)
        entry["diffs"] = diffs
        entry["leaves"] = {left: count_leaves(a), right: count_leaves(b)}
        out.append(entry)
        mark = "一致" if entry["identical"] else f"差分あり ({diffs[0]})"
        print(f"[{case.name}] {mark}", file=sys.stderr, flush=True)
    return {"left": left, "right": right, "cases": out}


# ---------------------------------------------------------------- 出力


def render_markdown(report: dict) -> str:
    names = list(report["endpoints"])
    lines: list[str] = []
    lines.append(
        "| クエリ | " + " | ".join(f"{n} p50/p95/p99 (ms)" for n in names) + " | 倍率 (p50) |"
    )
    lines.append("|---|" + "---|" * (len(names) + 1))
    for case in report["cases"]:
        cells = []
        p50s = {}
        for n in names:
            r = case["results"][n]
            p50s[n] = r["p50_ms"]
            flag = ""
            if r["gql_errors"] or r["http_errors"] or r["transport_errors"]:
                flag = " ⚠"
            cells.append(f"{r['p50_ms']:.0f} / {r['p95_ms']:.0f} / {r['p99_ms']:.0f}{flag}")
        if len(names) == 2 and p50s[names[1]] > 0:
            diff = f"{p50s[names[0]] / p50s[names[1]]:.1f}x"
        else:
            diff = "-"
        lines.append(f"| `{case['name']}` | " + " | ".join(cells) + f" | {diff} |")
    return "\n".join(lines)


def render_parity(result: dict) -> str:
    left, right = result["left"], result["right"]
    lines = [f"| クエリ | 一致 | 値の数 ({left} / {right}) | 最初の差分 |", "|---|---|---|---|"]
    for case in result["cases"]:
        if "error" in case:
            lines.append(f"| `{case['name']}` | 取得失敗 | - | {case['error']} |")
            continue
        leaves = case["leaves"]
        first = case["diffs"][0] if case["diffs"] else ""
        mark = "○" if case["identical"] else "×"
        lines.append(
            f"| `{case['name']}` | {mark} | {leaves[left]} / {leaves[right]} | {first} |"
        )
    return "\n".join(lines)


def render_report(report: dict, parity_result: dict | None, argv: list[str]) -> str:
    """公表・共有できる形の Markdown レポートを組み立てる。

    数字だけを切り出されると測定条件が落ちるので、条件・注意点・応答の一致まで
    1 つの文書に入れる。応答が食い違うクエリには印を付け、そのまま速度比較として
    引用されないようにする。
    """
    names = list(report["endpoints"])
    meta = report["meta"]
    ping = report["baseline_ping"]
    notes = {c["name"]: c["note"] for c in report["cases"]}
    identical = {}
    if parity_result:
        for case in parity_result["cases"]:
            identical[case["name"]] = case.get("identical")

    out: list[str] = []
    out.append(f"# GraphQL ベンチマーク: {' vs '.join(names)}")
    out.append("")
    out.append(
        "`schema/public.graphql` の `Query` 型にあるルートフィールドを 1 件ずつ、"
        "2 つのエンドポイントで同一条件に計測した記録。"
    )
    out.append("`scripts/benchmark_graphql.py` が生成している。")
    out.append("")

    out.append("## 計測条件")
    out.append("")
    out.append(f"- 計測日時: {meta['local']} (UTC {meta['utc']})")
    out.append(f"- 計測ホスト: {meta['host']} / {meta['platform']} / Python {meta['python']}")
    for name in names:
        out.append(f"- {name}: `{report['endpoints'][name]}`")
    out.append(
        f"- 1 クエリあたり warmup {report['warmup']} 回 + 計測 {report['iterations']} 回。"
        "接続は環境ごとに keep-alive で使い回すため、TLS ハンドシェイクの費用は warmup に吸われる。"
    )
    out.append(
        "- 1 反復ごとに環境の順番を入れ替えて交互に投げ、"
        "ネットワークの揺らぎが片方の環境に偏らないようにしている。"
    )
    out.append(
        "- 所要時間はリクエスト送信から本文を読み切るまでの実測。"
        "実クライアントに合わせて gzip を要求している。"
    )
    out.append(f"- 生成コマンド: `{' '.join(argv)}`")
    out.append("")

    out.append("### エッジまでの往復 (baseline)")
    out.append("")
    out.append("| 環境 | `GET /__ping` min | p50 | p95 |")
    out.append("|---|---:|---:|---:|")
    for name in names:
        stats = ping[name]
        out.append(
            f"| {name} | {stats['min_ms']:.0f} ms | {stats['p50_ms']:.0f} ms "
            f"| {stats['p95_ms']:.0f} ms |"
        )
    out.append("")
    out.append(
        "GraphQL を通さない疎通の値。各クエリの所要時間からこれを引いた分がサーバー側の"
        "処理時間にあたる。**この値は計測地点に依存する**ため、公表する数字は"
        "利用者のいる地域から測ること。上流へ往復する構成が相手の場合、"
        "計測地から上流までの距離がそのまま相手側の下駄になる。"
    )
    out.append("")

    out.append("## 結果")
    out.append("")
    header = "| クエリ | 内容 |"
    sep = "| --- | --- |"
    for name in names:
        header += f" {name} p50 / p95 / p99 |"
        sep += " ---: |"
    if identical:
        header += " 応答一致 |"
        sep += " :---: |"
    if len(names) == 2:
        header += " 倍率 (p50) |"
        sep += " ---: |"
    out.append(header)
    out.append(sep)

    totals = {n: 0.0 for n in names}
    for case in report["cases"]:
        row = f"| `{case['name']}` | {notes[case['name']]} |"
        for name in names:
            r = case["results"][name]
            totals[name] += r["p50_ms"]
            flag = " ⚠" if (r["gql_errors"] or r["http_errors"] or r["transport_errors"]) else ""
            row += f" {r['p50_ms']:.0f} / {r['p95_ms']:.0f} / {r['p99_ms']:.0f} ms{flag} |"
        if identical:
            mark = {True: "○", False: "**×**", None: "-"}[identical.get(case["name"])]
            row += f" {mark} |"
        if len(names) == 2:
            a, b = case["results"][names[0]]["p50_ms"], case["results"][names[1]]["p50_ms"]
            row += f" {a / b:.1f}x |" if b > 0 else " - |"
        out.append(row)

    total_row = "| **合計** | 全クエリの p50 合計 |"
    for name in names:
        total_row += f" **{totals[name]:.0f} ms** |"
    if identical:
        total_row += " |"
    if len(names) == 2 and totals[names[1]] > 0:
        total_row += f" **{totals[names[0]] / totals[names[1]]:.1f}x** |"
    elif len(names) == 2:
        total_row += " - |"
    out.append(total_row)
    out.append("")

    errors = sum(
        c["results"][n]["http_errors"] + c["results"][n]["gql_errors"]
        + c["results"][n]["transport_errors"]
        for c in report["cases"] for n in names
    )
    if errors:
        out.append(f"⚠ エラーが {errors} 件発生している。原因を確かめるまで結果を使わないこと。")
    else:
        out.append(
            f"エラー (HTTP 非 200 / GraphQL `errors` / 接続断) は"
            f"全 {len(report['cases'])} クエリ・全 {report['iterations']} 反復で 0 件。"
        )
    out.append("")

    out.append("### baseline を引いたサーバー側の処理時間 (p50)")
    out.append("")
    out.append("| クエリ |" + "".join(f" {n} |" for n in names)
               + (" 倍率 |" if len(names) == 2 else ""))
    out.append("| --- |" + " ---: |" * (len(names) + (1 if len(names) == 2 else 0)))
    for case in report["cases"]:
        row = f"| `{case['name']}` |"
        adjusted = {}
        for name in names:
            value = max(0.0, case["results"][name]["p50_ms"] - ping[name]["min_ms"])
            adjusted[name] = value
            row += f" {value:.0f} ms |"
        if len(names) == 2:
            b = adjusted[names[1]]
            row += f" {adjusted[names[0]] / b:.1f}x |" if b >= 1 else " 測定限界 |"
        out.append(row)
    out.append("")
    out.append(
        "引いた残りが 1ms 前後まで縮むクエリでは、割り算の分母が小さすぎて倍率が暴れる。"
        "その行は「エッジ以外に測れる時間がほとんど無い」とだけ読むこと。"
    )
    out.append("")

    out.append("### 応答量")
    out.append("")
    out.append("| クエリ |" + "".join(f" {n} 展開後 / 転送 |" for n in names))
    out.append("| --- |" + " ---: |" * len(names))
    for case in report["cases"]:
        row = f"| `{case['name']}` |"
        for name in names:
            r = case["results"][name]
            row += f" {r['bytes'] / 1024:.1f} / {r['wire_bytes'] / 1024:.1f} KB |"
        out.append(row)
    out.append("")

    if parity_result:
        left, right = parity_result["left"], parity_result["right"]
        mismatched = [c for c in parity_result["cases"] if not c.get("identical", True)]
        out.append("## 応答の一致")
        out.append("")
        out.append(
            f"同じクエリを 1 回ずつ投げ、`data` を突き合わせた結果。"
            f"{len(parity_result['cases']) - len(mismatched)} / "
            f"{len(parity_result['cases'])} クエリが一致。"
        )
        out.append("")
        if mismatched:
            out.append(f"| クエリ | 値の数 ({left} / {right}) | 最初の差分 |")
            out.append("| --- | ---: | --- |")
            for case in mismatched:
                if "error" in case:
                    out.append(f"| `{case['name']}` | - | {case['error']} |")
                    continue
                leaves = case["leaves"]
                first = case["diffs"][0] if case["diffs"] else ""
                out.append(
                    f"| `{case['name']}` | {leaves[left]} / {leaves[right]} | {first} |"
                )
            out.append("")
            out.append(
                "**上のクエリは 2 環境で返すものが違う。**返す量が違えばやっている仕事量も"
                "違うので、速度の数字をそのまま「同じ処理が N 倍速い」として引用できない。"
                "実装を変えた結果として出すなら、新しい応答が正しいことを説明できる状態に"
                "してから、変わった点と併せて書くこと。"
            )
        else:
            out.append("全クエリで応答が一致した。速度の比較対象として扱える。")
        out.append("")

    out.append("## この数字を外に出すときの注意")
    out.append("")
    out.append(
        "- **計測地点を明記する。** 上の baseline のとおり、エッジまでの往復は計測地で変わる。"
        "上流へ往復する構成を相手にすると、その往復も相手側の時間に乗る。"
    )
    out.append(
        "- **p50 だけを出さない。** p95 / p99 を併記する。"
        "新しく isolate が起きる分の跳ねを伏せると後で指摘される。"
    )
    out.append(
        "- **応答が一致しないクエリは別枠にする。** 速度ではなく設計変更の話として書く方が正確で、"
        "隠すより強い。"
    )
    out.append(
        "- **時間帯を変えて複数回測る。** `--append-jsonl` で貯めて `--summarize` でまとめられる。"
    )
    out.append("")
    return "\n".join(out)


def metadata(endpoints: dict[str, str]) -> dict:
    """どこで・いつ測ったかを結果に残す。数字だけ独り歩きさせないため。"""
    now = datetime.now(timezone.utc)
    return {
        "utc": now.isoformat(timespec="seconds"),
        "local": now.astimezone().isoformat(timespec="seconds"),
        "host": socket.gethostname(),
        "platform": platform.platform(),
        "python": platform.python_version(),
        "endpoints": endpoints,
    }


def summarize_jsonl(path: str) -> str:
    """--append-jsonl で貯めた複数回の結果をまとめる。

    公表する数字は時間帯を変えた複数回から出したいので、
    実行ごとの p50 の中央値と幅を出す。
    """
    runs = []
    with open(path, encoding="utf-8") as fp:
        for line in fp:
            line = line.strip()
            if line:
                runs.append(json.loads(line))
    if not runs:
        return "結果がありません"
    names = list(runs[0]["meta"]["endpoints"])
    lines = [f"{len(runs)} 回分 ({runs[0]['meta']['local']} 〜 {runs[-1]['meta']['local']})", ""]
    lines.append(
        "| クエリ | " + " | ".join(f"{n} p50 中央値 (最小〜最大)" for n in names) + " | 倍率 |"
    )
    lines.append("|---|" + "---|" * (len(names) + 1))
    for i, case in enumerate(runs[0]["cases"]):
        cells, medians = [], {}
        for n in names:
            values = [r["cases"][i]["results"][n]["p50_ms"] for r in runs]
            medians[n] = statistics.median(values)
            cells.append(f"{medians[n]:.0f} ({min(values):.0f}〜{max(values):.0f})")
        ratio = (
            f"{medians[names[0]] / medians[names[1]]:.1f}x"
            if len(names) == 2 and medians[names[1]] > 0
            else "-"
        )
        lines.append(f"| `{case['name']}` | " + " | ".join(cells) + f" | {ratio} |")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--iterations", type=int, default=20, help="1 ケースあたりの計測回数")
    parser.add_argument("--warmup", type=int, default=3, help="計測前に捨てる回数")
    parser.add_argument("--timeout", type=float, default=30.0, help="1 リクエストの上限秒")
    parser.add_argument("--production", default=DEFAULT_ENDPOINTS["production"])
    parser.add_argument("--staging", default=DEFAULT_ENDPOINTS["staging"])
    parser.add_argument("--only", nargs="*", help="対象のクエリ名を絞る")
    parser.add_argument("--list", action="store_true", help="クエリ名を並べて終わる")
    parser.add_argument("--parity", action="store_true",
                        help="速度ではなく応答の一致を調べる")
    parser.add_argument("--parity-limit", type=int, default=5,
                        help="1 クエリあたりに表示する差分の数")
    parser.add_argument("--report-out",
                        help="Markdown のレポートを書き出す先 "
                             "(応答の一致も併せて調べて載せる)")
    parser.add_argument("--json-out", help="生の結果を JSON で書き出す先")
    parser.add_argument("--append-jsonl",
                        help="結果を JSONL に 1 行追記する (時間帯を変えて貯める用)")
    parser.add_argument("--summarize", help="--append-jsonl で貯めた JSONL をまとめて表示する")
    args = parser.parse_args()

    if args.summarize:
        print(summarize_jsonl(args.summarize))
        return 0

    cases = build_cases()
    if args.list:
        for case in cases:
            print(f"{case.name:24} {case.note}")
        return 0
    if args.only:
        wanted = set(args.only)
        unknown = wanted - {c.name for c in cases}
        if unknown:
            print(f"知らないクエリ名: {', '.join(sorted(unknown))}", file=sys.stderr)
            return 1
        cases = [c for c in cases if c.name in wanted]

    urls = {"production": args.production, "staging": args.staging}
    endpoints = {name: Endpoint(name, url, args.timeout) for name, url in urls.items()}
    meta = metadata(urls)

    try:
        if args.parity:
            result = parity(endpoints, cases, args.parity_limit)
            result["meta"] = meta
            if args.json_out:
                with open(args.json_out, "w", encoding="utf-8") as fp:
                    json.dump(result, fp, ensure_ascii=False, indent=2)
            print(render_parity(result))
            mismatched = [c["name"] for c in result["cases"] if not c.get("identical", True)]
            print()
            print(f"一致: {len(result['cases']) - len(mismatched)} / {len(result['cases'])}")
            if mismatched:
                print("差分あり: " + ", ".join(mismatched))
                print("差分のあるクエリは、速度の比較対象として扱う前に")
                print("どちらの応答が正しいかを確かめること。")
            return 0

        report = run(endpoints, cases, args.iterations, args.warmup)
        report["meta"] = meta
        report["endpoints"] = urls
        report["iterations"] = args.iterations
        report["warmup"] = args.warmup
        report["baseline_ping"] = baseline(endpoints, max(5, args.iterations // 2))

        parity_result = None
        if args.report_out:
            # レポートには「同じ仕事をしているのか」まで載せたいので、
            # 各クエリ 1 回ずつ応答を突き合わせる (速度計測の後なので影響しない)。
            print("応答の一致を確認中...", file=sys.stderr, flush=True)
            parity_result = parity(endpoints, cases, args.parity_limit)
            report["parity"] = parity_result

        if args.json_out:
            with open(args.json_out, "w", encoding="utf-8") as fp:
                json.dump(report, fp, ensure_ascii=False, indent=2)
        if args.report_out:
            with open(args.report_out, "w", encoding="utf-8") as fp:
                fp.write(render_report(report, parity_result, sys.argv))
            print(f"レポートを書き出した: {args.report_out}", file=sys.stderr)
        if args.append_jsonl:
            with open(args.append_jsonl, "a", encoding="utf-8") as fp:
                fp.write(json.dumps(report, ensure_ascii=False) + "\n")

        print(f"計測: {meta['local']} / {meta['host']}")
        print()
        print(render_markdown(report))
        print()
        for name, stats in report["baseline_ping"].items():
            print(
                f"baseline GET /__ping {name}: min={stats['min_ms']:.0f}ms "
                f"p50={stats['p50_ms']:.0f}ms p95={stats['p95_ms']:.0f}ms"
            )
        print()
        print("baseline はエッジまでの往復にあたる。各クエリの p50 からこれを引いた分が")
        print("サーバー側の処理時間で、上流へ往復する構成ではその往復も含まれる。")
        return 0
    finally:
        for endpoint in endpoints.values():
            endpoint.close()


if __name__ == "__main__":
    raise SystemExit(main())
