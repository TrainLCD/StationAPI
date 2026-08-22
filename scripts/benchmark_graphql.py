#!/usr/bin/env python3
"""本番/ステージングの GraphQL エンドポイントを同一条件で叩いて所要時間を比べる。

schema/public.graphql の Query 型にあるルートフィールドを全て 1 件ずつ計測する。
片方の環境だけが遅い区間を見つけるのが目的なので、絶対値より 2 環境の差を見る。

  python3 scripts/benchmark_graphql.py --iterations 30 --warmup 3

計測は 1 反復ごとに環境の順番を入れ替えて交互に投げる。ネットワークの揺らぎが
片方の環境だけに偏らないようにするため。接続は環境ごとに keep-alive で使い回す
ので、TLS ハンドシェイクの費用は warmup に吸われる。
"""

from __future__ import annotations

import argparse
import json
import statistics
import sys
import time
from dataclasses import dataclass, field

try:
    import requests
except ImportError:  # pragma: no cover - 実行環境依存
    sys.exit("requests が必要です: pip install requests")

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


@dataclass
class Samples:
    """1 (環境, ケース) の計測結果。"""

    durations_ms: list[float] = field(default_factory=list)
    statuses: list[int] = field(default_factory=list)
    sizes: list[int] = field(default_factory=list)
    gql_errors: list[str] = field(default_factory=list)
    transport_errors: list[str] = field(default_factory=list)

    def percentile(self, pct: float) -> float:
        """線形補間なしの単純な順位統計 (試行回数が少ないので下位側に丸める)。"""
        if not self.durations_ms:
            return float("nan")
        ordered = sorted(self.durations_ms)
        idx = min(len(ordered) - 1, max(0, int(round(pct / 100 * len(ordered) + 0.5)) - 1))
        return ordered[idx]

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
            "max_ms": max(d) if d else float("nan"),
            "mean_ms": statistics.fmean(d) if d else float("nan"),
            "stdev_ms": statistics.stdev(d) if len(d) > 1 else 0.0,
            "bytes": statistics.fmean(self.sizes) if self.sizes else 0.0,
            "first_gql_error": self.gql_errors[0] if self.gql_errors else "",
            "first_transport_error": (
                self.transport_errors[0] if self.transport_errors else ""
            ),
        }


def post(session: requests.Session, url: str, query: str, timeout: float):
    """1 リクエストを投げて (経過ミリ秒, ステータス, 本文) を返す。"""
    started = time.perf_counter()
    resp = session.post(
        url,
        json={"query": query},
        timeout=timeout,
        headers={"content-type": "application/json"},
    )
    body = resp.content  # 本文を読み切るまでを所要時間に含める
    elapsed_ms = (time.perf_counter() - started) * 1000
    return elapsed_ms, resp.status_code, body


def record(samples: Samples, elapsed_ms: float, status: int, body: bytes) -> None:
    samples.durations_ms.append(elapsed_ms)
    samples.statuses.append(status)
    samples.sizes.append(len(body))
    if status != 200:
        return
    try:
        payload = json.loads(body)
    except json.JSONDecodeError as exc:
        samples.gql_errors.append(f"JSON 解釈に失敗: {exc}")
        return
    if payload.get("errors"):
        samples.gql_errors.append(str(payload["errors"][0].get("message", ""))[:200])


def run(endpoints: dict[str, str], cases: list[Case], iterations: int, warmup: int,
        timeout: float) -> dict:
    sessions = {name: requests.Session() for name in endpoints}
    results: dict[str, dict[str, Samples]] = {
        name: {case.name: Samples() for case in cases} for name in endpoints
    }
    order = list(endpoints)

    for case in cases:
        print(f"[{case.name}] warmup...", file=sys.stderr, flush=True)
        for name in order:
            for _ in range(warmup):
                try:
                    post(sessions[name], endpoints[name], case.query, timeout)
                except requests.RequestException:
                    pass

        for i in range(iterations):
            # 反復ごとに環境の順番を入れ替え、順番による偏りを打ち消す。
            seq = order if i % 2 == 0 else list(reversed(order))
            for name in seq:
                try:
                    elapsed_ms, status, body = post(
                        sessions[name], endpoints[name], case.query, timeout
                    )
                except requests.RequestException as exc:
                    results[name][case.name].transport_errors.append(str(exc)[:200])
                    continue
                record(results[name][case.name], elapsed_ms, status, body)
        done = {n: results[n][case.name].summary() for n in order}
        print(
            "  " + "  ".join(f"{n}: p50={done[n]['p50_ms']:.0f}ms" for n in order),
            file=sys.stderr,
            flush=True,
        )

    return {
        "endpoints": endpoints,
        "iterations": iterations,
        "warmup": warmup,
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


def baseline(endpoints: dict[str, str], iterations: int, timeout: float) -> dict:
    """GraphQL を通さない疎通 (`/__ping`) の所要時間。ネットワーク側の下駄を測る。"""
    out = {}
    for name, url in endpoints.items():
        session = requests.Session()
        ping = url.rstrip("/") + "/__ping"
        samples = Samples()
        for _ in range(3):
            try:
                session.get(ping, timeout=timeout)
            except requests.RequestException:
                pass
        for _ in range(iterations):
            started = time.perf_counter()
            try:
                resp = session.get(ping, timeout=timeout)
                body = resp.content
            except requests.RequestException as exc:
                samples.transport_errors.append(str(exc)[:200])
                continue
            record(samples, (time.perf_counter() - started) * 1000, resp.status_code, body)
        out[name] = samples.summary()
    return out


def concurrency_probe(endpoints: dict[str, str], case: Case, concurrency: int,
                      rounds: int, timeout: float) -> dict:
    """同時接続を増やしたときの応答時間。直列計測が待ち行列由来か切り分ける。

    本番へ余計な負荷をかけないよう、軽いクエリを少ない多重度で数回だけ投げる。
    """
    from concurrent.futures import ThreadPoolExecutor

    out = {}
    for name, url in endpoints.items():
        sessions = [requests.Session() for _ in range(concurrency)]
        for session in sessions:  # 接続確立を計測から外す
            try:
                post(session, url, case.query, timeout)
            except requests.RequestException:
                pass
        samples = Samples()
        for _ in range(rounds):
            with ThreadPoolExecutor(max_workers=concurrency) as pool:
                futures = [
                    pool.submit(post, sessions[i], url, case.query, timeout)
                    for i in range(concurrency)
                ]
                for future in futures:
                    try:
                        elapsed_ms, status, body = future.result()
                    except requests.RequestException as exc:
                        samples.transport_errors.append(str(exc)[:200])
                        continue
                    record(samples, elapsed_ms, status, body)
        out[name] = samples.summary()
    return out


def render_markdown(report: dict) -> str:
    names = list(report["endpoints"])
    lines: list[str] = []
    lines.append("| クエリ | " + " | ".join(f"{n} p50 / p95 (ms)" for n in names) + " | 差分 (p50) |")
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
            cells.append(f"{r['p50_ms']:.0f} / {r['p95_ms']:.0f}{flag}")
        if len(names) == 2 and p50s[names[0]] > 0:
            delta = (p50s[names[1]] - p50s[names[0]]) / p50s[names[0]] * 100
            diff = f"{delta:+.0f}%"
        else:
            diff = "-"
        lines.append(f"| `{case['name']}` | " + " | ".join(cells) + f" | {diff} |")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--iterations", type=int, default=20, help="1 ケースあたりの計測回数")
    parser.add_argument("--warmup", type=int, default=3, help="計測前に捨てる回数")
    parser.add_argument("--timeout", type=float, default=30.0, help="1 リクエストの上限秒")
    parser.add_argument("--production", default=DEFAULT_ENDPOINTS["production"])
    parser.add_argument("--staging", default=DEFAULT_ENDPOINTS["staging"])
    parser.add_argument("--only", nargs="*", help="計測するクエリ名を絞る")
    parser.add_argument("--concurrency", type=int, default=0,
                        help="並列プローブの多重度 (0 で実行しない)")
    parser.add_argument("--concurrency-rounds", type=int, default=3,
                        help="並列プローブの回数")
    parser.add_argument("--json-out", help="生の結果を JSON で書き出す先")
    args = parser.parse_args()

    endpoints = {"production": args.production, "staging": args.staging}
    cases = build_cases()
    if args.only:
        cases = [c for c in cases if c.name in set(args.only)]
        if not cases:
            return print("--only に一致するクエリがありません", file=sys.stderr) or 1

    report = run(endpoints, cases, args.iterations, args.warmup, args.timeout)
    report["baseline_ping"] = baseline(endpoints, max(5, args.iterations // 2), args.timeout)

    if args.concurrency > 0:
        probe_case = next(c for c in cases if c.name == "station")
        report["concurrency"] = {
            "case": probe_case.name,
            "concurrency": args.concurrency,
            "rounds": args.concurrency_rounds,
            "results": concurrency_probe(
                endpoints, probe_case, args.concurrency, args.concurrency_rounds, args.timeout
            ),
        }

    if args.json_out:
        with open(args.json_out, "w", encoding="utf-8") as fp:
            json.dump(report, fp, ensure_ascii=False, indent=2)

    print(render_markdown(report))
    print()
    for name, stats in report["baseline_ping"].items():
        print(f"baseline /__ping {name}: p50={stats['p50_ms']:.0f}ms p95={stats['p95_ms']:.0f}ms")
    if "concurrency" in report:
        probe = report["concurrency"]
        print(
            f"\n並列 {probe['concurrency']} 多重 x {probe['rounds']} 回 "
            f"(`{probe['case']}`):"
        )
        for name, stats in probe["results"].items():
            print(
                f"  {name}: p50={stats['p50_ms']:.0f}ms p95={stats['p95_ms']:.0f}ms "
                f"max={stats['max_ms']:.0f}ms errors={stats['http_errors'] + stats['gql_errors'] + stats['transport_errors']}"
            )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
