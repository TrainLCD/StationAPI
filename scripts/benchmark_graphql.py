#!/usr/bin/env python3
"""GraphQL エンドポイントを 2 つ並べて、比較として成立する形で計測する。

schema/public.graphql の Query 型にあるルートフィールドを全て扱う。
既定では本番 (gql.trainlcd.app) とステージング (gql-stg.trainlcd.app) を比べる。

  python3 scripts/benchmark_graphql.py --iterations 30 --report-out bench.md
  python3 scripts/benchmark_graphql.py --parity

依存は Python 3.9 以降の標準ライブラリのみ。pip での用意は要らない。

速度の数字は条件次第でいくらでも動くので、比較として成立させるために以下を持つ:

* **1 クエリにつき複数の引数を巡回する。** 同じ引数を何度も投げると、上流の
  メモ化や暖まった経路にだけ当たり続けて実態とずれる。同じ反復では両環境に
  同じ引数を渡すので、workload は完全に揃う。
* **フィールド選択を 2 段用意する** (`--profile`)。応答量が環境間で違うと、
  検索の速さではなく直列化と転送の量を測ってしまう。minimal は ID 程度しか
  引かないので、応答量の差に鈍い。
* **`GET /__ping` をケースごとに挟む。** エッジまでの往復を計測中ずっと追い、
  ネットワークの揺らぎと処理時間を切り分ける。
* **応答の食い違いを種類で分ける** (`--parity`)。件数や null の有無が違えば
  やっている仕事量が違うので速度を比較できない。一方、色や距離のような値だけの
  違いはデータ版の差なので比較できる。レポートは前者を除いて集計する。

なお **計測地点は結果に直接効く**。上流へ往復する構成が相手の場合、計測地から
上流までの距離がそのまま相手側の下駄になる。公表する数字は利用者のいる地域から測ること。
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

PROFILES = ("full", "minimal")

# ---------------------------------------------------------------- フィールド選択

# full: クライアントが実際に引く程度の量。ネストも 1 段含め、応答生成の費用を測る。
# minimal: 応答量をほぼ無くし、探索そのものの費用に寄せる。
# 2 つを比べると、環境間の差が探索由来か直列化・転送由来か切り分けられる。
STATION_FIELDS = {
    "full": """
  id groupId name nameKatakana nameRoman nameChinese nameKorean
  threeLetterCode prefectureId postalCode address latitude longitude
  openedAt closedAt status stopCondition distance hasTrainTypes transportType
  stationNumbers { lineSymbol lineSymbolColor lineSymbolShape stationNumber }
  line { id nameShort nameRoman color lineType status
         lineSymbols { symbol color shape }
         company { id nameShort nameEnglishShort type status } }
  lines { id nameShort nameRoman color lineType status }
  trainType { id typeId groupId name nameRoman color direction kind }
""",
    "minimal": " id groupId name ",
}

LINE_FIELDS = {
    "full": """
  id nameShort nameKatakana nameFull nameRoman nameChinese nameKorean
  color lineType status averageDistance transportType
  lineSymbols { symbol color shape }
  company { id railroadId nameShort nameFull nameEnglishShort url type status }
  station { id groupId name nameRoman }
  trainType { id typeId groupId name nameRoman color direction kind }
""",
    "minimal": " id nameShort color ",
}

TRAIN_TYPE_FIELDS = {
    "full": """
  id typeId groupId name nameKatakana nameRoman nameChinese nameKorean
  color direction kind
  line { id nameShort nameRoman color }
  lines { id nameShort nameRoman color }
""",
    "minimal": " id typeId groupId name ",
}

ROUTE_STOP_FIELDS = {
    "full": "id groupId name nameRoman line { id nameShort color } "
            "trainType { id name kind }",
    "minimal": "id groupId",
}

# ---------------------------------------------------------------- 引数の組

# 実在する駅・路線だけを使う。存在しない ID は空応答になり計測の意味が無くなる。
# 1 つの引数を投げ続けると上流のメモ化や暖まった経路にだけ当たるため、
# 地域と規模を散らした組を巡回する。全ての組が両環境で通ることが前提。
STATION_IDS = [
    (1130205, "渋谷"),
    (1160213, "新大阪"),
    (1110315, "札幌"),
]
STATION_ID_SETS = [
    ([1130205, 1130208, 1130101, 1130224, 1130105], "関東 5 駅"),
    ([1160213, 1160120, 1141101, 1150233, 1150801], "関西・中部 5 駅"),
    ([1110315, 1190101, 9910206, 1131310, 1130102], "全国 5 駅"),
]
COORDS = [
    (35.658034, 139.701636, "渋谷駅前"),
    (34.702500, 135.495900, "大阪駅前"),
    (35.170600, 136.881600, "名古屋駅前"),
]
STATION_NAMES = [("新宿", "新宿"), ("京都", "京都"), ("博多", "博多")]
STATION_GROUPS = [
    (1130205, "渋谷"),
    (1160213, "新大阪"),
    (1141101, "名古屋"),
]
LINE_GROUPS = [
    (363, "山手線 普通 (30 駅)"),
    (585, "中央・総武線 各駅停車 (39 駅)"),
    (182, "みなとみらい線 各駅停車 (62 駅)"),
]
LINE_GROUP_SETS = [
    ([363], "山手線"),
    ([585, 27], "中央総武 + 東海道快速"),
    ([182], "みなとみらい線"),
]
LINE_IDS = [
    (11302, "山手線"),
    (11602, "JR京都線"),
    (99102, "札幌市営地下鉄南北線"),
]
LINE_ID_SETS = [
    ([11302, 11321, 26001, 28001, 24006], "関東 5 路線"),
    ([1002, 1003, 11602, 11641, 99618], "関西 5 路線"),
    ([11103, 11109, 11112, 99102, 99103], "北海道 5 路線"),
]
LINE_NAMES = [("山手", "山手"), ("本線", "本線"), ("地下鉄", "地下鉄")]
LINE_ID_PAIRS = [
    ([11302, 26001], "山手 + 東横"),
    ([11602, 11641], "JR京都 + おおさか東"),
    ([11103, 99102], "函館本線 + 札幌南北"),
]
TRAIN_TYPE_STATIONS = [
    (1130205, "渋谷"),
    (1130105, "横浜"),
    (2600101, "渋谷 (東急)"),
]
GROUP_PAIRS = [
    (1130205, 1130208, "渋谷→新宿"),
    (1130101, 1130105, "東京→横浜"),
    (1160213, 1160120, "新大阪→京都"),
]
# 同一系統に属する駅の組。lineGroupStations の先頭と末尾から取っている。
LINE_GROUP_TRIPS = [
    (1130205, 1130224, 363, "山手線 渋谷→東京"),
    (1131301, 1131339, 585, "中央・総武線 三鷹→千葉"),
    (1130101, 1130121, 27, "東海道線 東京→熱海"),
]


@dataclass
class Variant:
    """1 ケース内で巡回する引数の 1 つ。"""

    label: str
    query: str


@dataclass
class Case:
    """1 クエリ分の計測ケース。"""

    name: str
    note: str
    variants: list[Variant]


def build_cases(profile: str) -> list[Case]:
    """schema/public.graphql の Query 型を 1 フィールドずつ網羅する。"""
    st = STATION_FIELDS[profile]
    ln = LINE_FIELDS[profile]
    tt = TRAIN_TYPE_FIELDS[profile]
    stop = ROUTE_STOP_FIELDS[profile]

    def ids(values: list[int]) -> str:
        return ", ".join(str(v) for v in values)

    return [
        Case("station", "単一駅", [
            Variant(label, "{ station(id: %d) { %s } }" % (sid, st))
            for sid, label in STATION_IDS
        ]),
        Case("stations", "駅 5 件の一括取得", [
            Variant(label, "{ stations(ids: [%s]) { %s } }" % (ids(values), st))
            for values, label in STATION_ID_SETS
        ]),
        Case("stationsNearby", "座標から半径検索 10 件", [
            Variant(
                label,
                "{ stationsNearby(latitude: %f, longitude: %f, limit: 10) { %s } }"
                % (lat, lon, st),
            )
            for lat, lon, label in COORDS
        ]),
        Case("stationsByName", "駅名検索 (全件走査) 10 件", [
            Variant(label, '{ stationsByName(name: "%s", limit: 10) { %s } }' % (name, st))
            for name, label in STATION_NAMES
        ]),
        Case("stationGroupStations", "同一駅グループの全乗り入れ", [
            Variant(label, "{ stationGroupStations(groupId: %d) { %s } }" % (gid, st))
            for gid, label in STATION_GROUPS
        ]),
        Case("lineGroupStations", "系統の全駅", [
            Variant(label, "{ lineGroupStations(lineGroupId: %d) { %s } }" % (gid, st))
            for gid, label in LINE_GROUPS
        ]),
        Case("line", "単一路線", [
            Variant(label, "{ line(lineId: %d) { %s } }" % (lid, ln))
            for lid, label in LINE_IDS
        ]),
        Case("lines", "路線 5 件の一括取得", [
            Variant(label, "{ lines(lineIds: [%s]) { %s } }" % (ids(values), ln))
            for values, label in LINE_ID_SETS
        ]),
        Case("linesByName", "路線名検索 (全件走査)", [
            Variant(label, '{ linesByName(name: "%s", limit: 10) { %s } }' % (name, ln))
            for name, label in LINE_NAMES
        ]),
        Case("lineStations", "路線の全駅", [
            Variant(label, "{ lineStations(lineId: %d) { %s } }" % (lid, st))
            for lid, label in LINE_IDS
        ]),
        Case("lineListStations", "複数路線の駅を一括取得", [
            Variant(label, "{ lineListStations(lineIds: [%s]) { %s } }" % (ids(values), st))
            for values, label in LINE_ID_PAIRS
        ]),
        Case("lineGroupListStations", "複数系統の駅を一括取得", [
            Variant(
                label,
                "{ lineGroupListStations(lineGroupIds: [%s]) { %s } }" % (ids(values), st),
            )
            for values, label in LINE_GROUP_SETS
        ]),
        Case("stationTrainTypes", "駅の種別一覧", [
            Variant(label, "{ stationTrainTypes(stationId: %d) { %s } }" % (sid, tt))
            for sid, label in TRAIN_TYPE_STATIONS
        ]),
        Case("routes", "経路探索 (10 件)", [
            Variant(
                label,
                "{ routes(fromStationGroupId: %d, toStationGroupId: %d, pageSize: 10) "
                "{ nextPageToken routes { id stops { %s } } } }" % (a, b, stop),
            )
            for a, b, label in GROUP_PAIRS
        ]),
        Case("routeTypes", "経路の種別一覧", [
            Variant(
                label,
                "{ routeTypes(fromStationGroupId: %d, toStationGroupId: %d, pageSize: 10) "
                "{ nextPageToken trainTypes { %s } } }" % (a, b, tt),
            )
            for a, b, label in GROUP_PAIRS
        ]),
        Case("connectedRoutes", "直通経路", [
            Variant(
                label,
                "{ connectedRoutes(fromStationGroupId: %d, toStationGroupId: %d) "
                "{ id stops { %s } } }" % (a, b, stop),
            )
            for a, b, label in GROUP_PAIRS
        ]),
        Case("estimateArrivalTimes", "到達時分の推定", [
            Variant(
                label,
                "{ estimateArrivalTimes(fromStationId: %d, toStationId: %d) "
                "{ routes { id stops { stationId stationGroupId cumulativeMinutes "
                "stopsHere departureCumulativeMinutes } } } }" % (a, b),
            )
            for a, b, _group, label in LINE_GROUP_TRIPS
        ]),
        Case("trainRoute", "走行区間の生成", [
            Variant(
                label,
                "{ trainRoute(fromStationId: %d, toStationId: %d, lineGroupId: %d) "
                "{ segments { stops distanceFromPrevious maxSpeed maxAcceleration "
                "maxDeceleration station { %s } } } }" % (a, b, group, st),
            )
            for a, b, group, label in LINE_GROUP_TRIPS
        ]),
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
        """環境変数のプロキシ設定を拾う。プロキシ越しの環境でも同じ手順で測れるようにする。"""
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
        for _ in range(2):  # idle で切られた接続の張り直しに 1 回だけ付き合う
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
        return self.send("GET", self.path.rstrip("/") + "/__ping", None)


# ---------------------------------------------------------------- 集計


@dataclass
class Samples:
    """1 系列の計測結果。"""

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


# ---------------------------------------------------------------- 応答の突き合わせ

# 差分の種類。速度を比較してよいかの判断がこれで決まる。
KIND_STRUCTURE = "structure"  # 件数・null の有無・型 -> 仕事量が違う
KIND_ORDER = "order"          # 中身は同じで並びだけ違う -> 仕事量は近いが結果は違う
KIND_VALUE = "value"          # 値だけ違う -> データ版の差

KIND_LABEL = {
    KIND_STRUCTURE: "構造",
    KIND_ORDER: "並び",
    KIND_VALUE: "値",
}


def _ids_of(items: list) -> list | None:
    """リストの要素から並び順の比較に使える ID を取り出す。"""
    out = []
    for item in items:
        if not isinstance(item, dict):
            return None
        key = next((k for k in ("id", "stationId", "groupId") if k in item), None)
        if key is None or item[key] is None:
            return None
        out.append(item[key])
    return out


def compare(a, b, path: str, acc: list[dict], limit: int) -> None:
    """2 つの応答を突き合わせ、差分を種類つきで集める。"""
    if len(acc) >= limit:
        return
    if a is None or b is None:
        if a is not b:
            acc.append({"kind": KIND_STRUCTURE, "path": path,
                        "detail": f"片方だけ null ({a!r} / {b!r})"})
        return
    if isinstance(a, dict) and isinstance(b, dict):
        for key in sorted(set(a) | set(b)):
            if key not in a or key not in b:
                acc.append({"kind": KIND_STRUCTURE, "path": f"{path}.{key}",
                            "detail": "片方にしか無い"})
            else:
                compare(a[key], b[key], f"{path}.{key}", acc, limit)
            if len(acc) >= limit:
                return
        return
    if isinstance(a, list) and isinstance(b, list):
        if len(a) != len(b):
            acc.append({"kind": KIND_STRUCTURE, "path": path,
                        "detail": f"件数が違う ({len(a)} / {len(b)})"})
            return
        ids_a, ids_b = _ids_of(a), _ids_of(b)
        if ids_a and ids_b and ids_a != ids_b:
            if sorted(map(str, ids_a)) == sorted(map(str, ids_b)):
                acc.append({"kind": KIND_ORDER, "path": path,
                            "detail": f"並びが違う (先頭 {ids_a[0]} / {ids_b[0]})"})
                # 並びだけの違いなら ID で揃えてから中身を比べる。
                # 揃えずに比べると同じ値が全部差分として出てしまう。
                index = {str(i): item for i, item in zip(ids_b, b)}
                for i, item in zip(ids_a, a):
                    mate = index.get(str(i))
                    if mate is not None:
                        compare(item, mate, f"{path}[{i}]", acc, limit)
                    if len(acc) >= limit:
                        return
                return
            acc.append({"kind": KIND_STRUCTURE, "path": path,
                        "detail": f"含まれるものが違う (先頭 {ids_a[0]} / {ids_b[0]})"})
            return
        for i, (x, y) in enumerate(zip(a, b)):
            compare(x, y, f"{path}[{i}]", acc, limit)
            if len(acc) >= limit:
                return
        return
    if type(a) is not type(b) and not (
        isinstance(a, (int, float)) and isinstance(b, (int, float))
    ):
        acc.append({"kind": KIND_STRUCTURE, "path": path,
                    "detail": f"型が違う ({type(a).__name__} / {type(b).__name__})"})
        return
    if isinstance(a, float) or isinstance(b, float):
        # 浮動小数は表現差 (0 と 0.0 など) が出るので相対誤差で見る
        if abs(a - b) > max(1e-9, abs(a) * 1e-9):
            acc.append({"kind": KIND_VALUE, "path": path, "detail": f"{a} / {b}"})
        return
    if a != b:
        acc.append({"kind": KIND_VALUE, "path": path, "detail": f"{a!r} / {b!r}"})


def count_leaves(value) -> int:
    if isinstance(value, dict):
        return sum(count_leaves(v) for v in value.values())
    if isinstance(value, list):
        return sum(count_leaves(v) for v in value)
    return 1


def parity(endpoints: dict[str, Endpoint], cases: list[Case], limit: int,
           quiet: bool = False) -> dict:
    """同じクエリの応答が 2 環境で一致するか、引数の組ごとに調べる。

    速度の主張には「本当に同じ仕事をしているのか」という反論が必ず付く。
    件数や null の有無が違えば仕事量が違うので、そのクエリは速度比較から外す。
    値だけの違い (色、距離など) はデータ版の差なので比較は成立する。
    """
    names = list(endpoints)
    if len(names) != 2:
        raise SystemExit("--parity は 2 環境でのみ使えます")
    left, right = names
    out = []
    for case in cases:
        variants = []
        for variant in case.variants:
            entry: dict = {"label": variant.label}
            bodies = {}
            failed = False
            for name in names:
                try:
                    _, status, body, _ = endpoints[name].graphql(variant.query)
                except TransportError as exc:
                    entry["error"] = f"{name}: {exc}"
                    failed = True
                    break
                if status != 200:
                    entry["error"] = f"{name}: HTTP {status}"
                    failed = True
                    break
                bodies[name] = json.loads(body)
            if not failed:
                gql = {n: bodies[n].get("errors") for n in names}
                if any(gql.values()):
                    entry["gql_errors"] = {
                        n: (e[0]["message"][:160] if e else None) for n, e in gql.items()
                    }
                diffs: list[dict] = []
                compare(bodies[left].get("data"), bodies[right].get("data"),
                        "data", diffs, limit)
                entry["diffs"] = diffs
                entry["kinds"] = sorted({d["kind"] for d in diffs})
                entry["leaves"] = {
                    left: count_leaves(bodies[left].get("data")),
                    right: count_leaves(bodies[right].get("data")),
                }
            variants.append(entry)

        kinds = sorted({k for v in variants for k in v.get("kinds", [])})
        errored = [v for v in variants if "error" in v]
        # 構造が違うクエリだけ速度比較から外す。並びと値の違いは仕事量を変えない。
        comparable = not errored and KIND_STRUCTURE not in kinds
        out.append({
            "name": case.name,
            "note": case.note,
            "variants": variants,
            "kinds": kinds,
            "identical": not errored and not kinds,
            "comparable": comparable,
            "reason": _parity_reason(errored, kinds),
        })
        if not quiet:
            print(f"[{case.name}] {out[-1]['reason']}", file=sys.stderr, flush=True)
    return {"left": left, "right": right, "cases": out}


def _parity_reason(errored: list, kinds: list[str]) -> str:
    if errored:
        return "取得に失敗"
    if not kinds:
        return "一致"
    return "差分: " + " / ".join(KIND_LABEL[k] for k in kinds)


# ---------------------------------------------------------------- 計測


def run(endpoints: dict[str, Endpoint], cases: list[Case], iterations: int,
        warmup: int) -> dict:
    """全ケースを計測する。

    同じ反復では両環境に同じ引数を渡し、反復ごとに環境の順番を入れ替える。
    こうすると workload は完全に揃い、ネットワークの揺らぎも片方に偏らない。
    """
    order = list(endpoints)
    pooled: dict[str, dict[str, Samples]] = {
        name: {case.name: Samples() for case in cases} for name in order
    }
    per_variant: dict[str, dict[str, dict[str, Samples]]] = {
        name: {c.name: {v.label: Samples() for v in c.variants} for c in cases}
        for name in order
    }
    ping_trace: dict[str, list[float]] = {name: [] for name in order}

    for case in cases:
        # ケースの区切りで疎通を測り、計測中ずっとネットワークの状態を追う。
        for name in order:
            try:
                elapsed_ms, _, _, _ = endpoints[name].ping()
                ping_trace[name].append(elapsed_ms)
            except TransportError:
                pass

        print(f"[{case.name}] warmup...", file=sys.stderr, flush=True)
        for name in order:
            for i in range(warmup):
                try:
                    endpoints[name].graphql(case.variants[i % len(case.variants)].query)
                except TransportError:
                    pass

        for i in range(iterations):
            variant = case.variants[i % len(case.variants)]  # 両環境に同じ引数
            seq = order if i % 2 == 0 else list(reversed(order))
            for name in seq:
                try:
                    elapsed_ms, status, body, wire = endpoints[name].graphql(variant.query)
                except TransportError as exc:
                    pooled[name][case.name].transport_errors.append(str(exc)[:200])
                    per_variant[name][case.name][variant.label].transport_errors.append(
                        str(exc)[:200]
                    )
                    continue
                record(pooled[name][case.name], elapsed_ms, status, body, wire)
                record(per_variant[name][case.name][variant.label],
                       elapsed_ms, status, body, wire)

        done = {n: pooled[n][case.name].summary() for n in order}
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
                "variants": [
                    {
                        "label": v.label,
                        "query": " ".join(v.query.split()),
                        "results": {
                            n: per_variant[n][case.name][v.label].summary() for n in order
                        },
                    }
                    for v in case.variants
                ],
                "results": {n: pooled[n][case.name].summary() for n in order},
            }
            for case in cases
        ],
        "ping_trace": {
            name: {
                "n": len(values),
                "min_ms": min(values) if values else float("nan"),
                "max_ms": max(values) if values else float("nan"),
                "median_ms": statistics.median(values) if values else float("nan"),
                "samples_ms": [round(v, 1) for v in values],
            }
            for name, values in ping_trace.items()
        },
    }


def baseline(endpoints: dict[str, Endpoint], iterations: int) -> dict:
    """GraphQL を通さない疎通 (`GET /__ping`) をまとめて測る。

    エッジまでの往復にあたる。各クエリの所要時間からこれを引けば、
    残りがサーバー側の処理時間になる。
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


# ---------------------------------------------------------------- 出力


def render_markdown(report: dict) -> str:
    names = list(report["endpoints"])
    parity_result = report.get("parity")
    comparable = (
        {c["name"]: c["comparable"] for c in parity_result["cases"]}
        if parity_result else {}
    )
    lines: list[str] = []
    head = "| クエリ | " + " | ".join(f"{n} p50/p95/p99 (ms)" for n in names)
    if comparable:
        head += " | 比較可"
    if len(names) == 2:
        head += " | 倍率 (p50)"
    lines.append(head + " |")
    lines.append("|---|" + "---|" * (len(names) + (1 if comparable else 0)
                                     + (1 if len(names) == 2 else 0)))
    for case in report["cases"]:
        cells = []
        p50s = {}
        for n in names:
            r = case["results"][n]
            p50s[n] = r["p50_ms"]
            flag = " ⚠" if (r["gql_errors"] or r["http_errors"] or r["transport_errors"]) else ""
            cells.append(f"{r['p50_ms']:.0f} / {r['p95_ms']:.0f} / {r['p99_ms']:.0f}{flag}")
        row = f"| `{case['name']}` | " + " | ".join(cells)
        if comparable:
            row += " | " + ("○" if comparable.get(case["name"]) else "×")
        if len(names) == 2:
            row += f" | {p50s[names[0]] / p50s[names[1]]:.1f}x" if p50s[names[1]] > 0 else " | -"
        lines.append(row + " |")
    return "\n".join(lines)


def _aggregate(report: dict, names: list[str], only_comparable: bool) -> dict[str, float]:
    parity_result = report.get("parity")
    comparable = (
        {c["name"]: c["comparable"] for c in parity_result["cases"]}
        if parity_result else {}
    )
    totals = {n: 0.0 for n in names}
    count = 0
    for case in report["cases"]:
        if only_comparable and not comparable.get(case["name"], True):
            continue
        for n in names:
            totals[n] += case["results"][n]["p50_ms"]
        count += 1
    totals["_count"] = count
    return totals


def render_report(report: dict, argv: list[str]) -> str:
    """公表・共有できる形の Markdown レポートを組み立てる。

    数字だけ切り出されると測定条件が落ちるので、条件・注意点・応答の一致まで
    1 つの文書に入れる。集計は応答が比較可能なクエリだけで出し、
    仕事量の違うクエリを混ぜた見かけの倍率にならないようにする。
    """
    names = list(report["endpoints"])
    meta = report["meta"]
    ping = report["baseline_ping"]
    trace = report.get("ping_trace", {})
    parity_result = report.get("parity")
    cases_by_name = {c["name"]: c for c in report["cases"]}
    parity_by_name = (
        {c["name"]: c for c in parity_result["cases"]} if parity_result else {}
    )

    out: list[str] = []
    out.append(f"# GraphQL ベンチマーク: {' vs '.join(names)}")
    out.append("")
    out.append(
        "`schema/public.graphql` の `Query` 型にあるルートフィールドを、"
        "2 つのエンドポイントで同一条件に計測した記録。"
        "`scripts/benchmark_graphql.py` が生成している。"
    )
    out.append("")

    out.append("## 計測条件")
    out.append("")
    out.append(f"- 計測日時: {meta['local']} (UTC {meta['utc']})")
    out.append(f"- 計測ホスト: {meta['host']} / {meta['platform']} / Python {meta['python']}")
    for name in names:
        out.append(f"- {name}: `{report['endpoints'][name]}`")
    out.append(f"- フィールド選択: `{report['profile']}`")
    out.append(
        f"- 1 クエリあたり warmup {report['warmup']} 回 + 計測 {report['iterations']} 回。"
        "接続は環境ごとに keep-alive で使い回すため、TLS ハンドシェイクの費用は warmup に吸われる。"
    )
    out.append(
        "- 1 クエリにつき複数の引数を巡回する。同じ反復では両環境に同じ引数を渡すので、"
        "両者の workload は完全に一致する。"
    )
    out.append(
        "- 反復ごとに環境の順番を入れ替え、ネットワークの揺らぎが片方に偏らないようにしている。"
    )
    out.append(
        "- 所要時間はリクエスト送信から本文を読み切るまでの実測。"
        "実クライアントに合わせて gzip を要求している。"
    )
    out.append(f"- 生成コマンド: `{' '.join(argv)}`")
    out.append("")

    out.append("### エッジまでの往復 (baseline)")
    out.append("")
    out.append("| 環境 | `GET /__ping` min | p50 | p95 | 計測中の振れ |")
    out.append("|---|---:|---:|---:|---:|")
    for name in names:
        stats = ping[name]
        t = trace.get(name, {})
        # 最大値は keep-alive が切れた直後の張り直しを拾うことがあるので、
        # 中央値も併記して条件が荒れていたのかを見分けられるようにする。
        drift = (
            f"中央 {t['median_ms']:.0f} / {t['min_ms']:.0f}〜{t['max_ms']:.0f} ms "
            f"({t['n']} 回)"
            if t.get("n") else "-"
        )
        out.append(
            f"| {name} | {stats['min_ms']:.0f} ms | {stats['p50_ms']:.0f} ms "
            f"| {stats['p95_ms']:.0f} ms | {drift} |"
        )
    out.append("")
    out.append(
        "GraphQL を通さない疎通の値。各クエリの所要時間からこれを引いた分がサーバー側の"
        "処理時間にあたる。右端はケースの区切りごとに測った値で、計測中に条件が"
        "変わっていないかの確認に使う。**この値は計測地点に依存する**ため、"
        "公表する数字は利用者のいる地域から測ること。上流へ往復する構成が相手の場合、"
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
    if parity_result:
        header += " 比較可 |"
        sep += " :---: |"
    if len(names) == 2:
        header += " 倍率 (p50) |"
        sep += " ---: |"
    out.append(header)
    out.append(sep)

    for case in report["cases"]:
        row = f"| `{case['name']}` | {case['note']} |"
        for name in names:
            r = case["results"][name]
            flag = " ⚠" if (r["gql_errors"] or r["http_errors"] or r["transport_errors"]) else ""
            row += f" {r['p50_ms']:.0f} / {r['p95_ms']:.0f} / {r['p99_ms']:.0f} ms{flag} |"
        ok = True
        if parity_result:
            ok = parity_by_name[case["name"]]["comparable"]
            row += f" {'○' if ok else '**×**'} |"
        if len(names) == 2:
            a, b = case["results"][names[0]]["p50_ms"], case["results"][names[1]]["p50_ms"]
            row += f" {a / b:.1f}x |" if b > 0 else " - |"
        out.append(row)
    out.append("")

    if parity_result and len(names) == 2:
        comp = _aggregate(report, names, only_comparable=True)
        allc = _aggregate(report, names, only_comparable=False)
        out.append("### 集計")
        out.append("")
        out.append("| 対象 | クエリ数 | " + " | ".join(f"{n} p50 合計" for n in names) + " | 倍率 |")
        out.append("| --- | ---: | ---: | ---: | ---: |")
        for label, agg in (("応答が比較可能なものだけ", comp), ("全クエリ (参考)", allc)):
            ratio = (
                f"{agg[names[0]] / agg[names[1]]:.1f}x" if agg[names[1]] > 0 else "-"
            )
            out.append(
                f"| {label} | {int(agg['_count'])} | "
                + " | ".join(f"{agg[n]:.0f} ms" for n in names)
                + f" | **{ratio}** |"
            )
        out.append("")
        out.append(
            "**公表するなら上段を使うこと。**下段は応答の違うクエリを含むので、"
            "同じ処理の比較になっていない。"
        )
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
        "引いた残りが 1ms 前後まで縮むクエリでは分母が小さすぎて倍率が暴れる。"
        "その行は「エッジ以外に測れる時間がほとんど無い」とだけ読むこと。"
    )
    out.append("")

    out.append("### 引数ごとの内訳 (p50)")
    out.append("")
    out.append("| クエリ | 引数 |" + "".join(f" {n} |" for n in names))
    out.append("| --- | --- |" + " ---: |" * len(names))
    for case in report["cases"]:
        for variant in case["variants"]:
            row = f"| `{case['name']}` | {variant['label']} |"
            for name in names:
                row += f" {variant['results'][name]['p50_ms']:.0f} ms |"
            out.append(row)
    out.append("")
    out.append(
        "1 つの引数だけで測ると、暖まった経路やメモ化にだけ当たって実態とずれる。"
        "巡回した引数ごとの値がここ。特定の引数だけ極端に遅ければ、"
        "全体の p50 ではなくその行を見ること。"
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
        out.append("## 応答の一致")
        out.append("")
        out.append(
            "同じ引数で 1 回ずつ投げ、`data` を突き合わせた結果。"
            "差分は種類で分けている。**構造**の違い (件数・null の有無・型) は"
            "やっている仕事量が違うので速度を比較できない。"
            "**並び**と**値**の違いは仕事量を変えないので、比較は成立する"
            "(ただし結果としては違うので、それ自体は別途確認が要る)。"
        )
        out.append("")
        out.append("| クエリ | 判定 | 差分の種類 | 例 |")
        out.append("| --- | :---: | --- | --- |")
        for case in parity_result["cases"]:
            first = ""
            for variant in case["variants"]:
                if variant.get("diffs"):
                    d = variant["diffs"][0]
                    first = f"`{d['path']}` {d['detail']}"
                    break
                if "error" in variant:
                    first = variant["error"]
                    break
            kinds = " / ".join(KIND_LABEL[k] for k in case["kinds"]) or "-"
            mark = "○" if case["comparable"] else "**×**"
            out.append(f"| `{case['name']}` | {mark} | {kinds} | {first} |")
        out.append("")
        blocked = [c["name"] for c in parity_result["cases"] if not c["comparable"]]
        if blocked:
            out.append(
                "**× のクエリは 2 環境で返すものが違う。**速度の数字を"
                "「同じ処理が N 倍速い」として引用できない。実装を変えた結果として"
                "出すなら、新しい応答が正しいことを説明できる状態にしてから、"
                "変わった点と併せて書くこと。"
            )
            out.append("")

    out.append("## この数字を外に出すときの注意")
    out.append("")
    out.append(
        "- **計測地点を明記する。** エッジまでの往復は計測地で変わる。"
        "上流へ往復する構成を相手にすると、その往復も相手側の時間に乗る。"
    )
    out.append("- **集計は「応答が比較可能なものだけ」を使う。**")
    out.append(
        "- **p50 だけを出さない。** p95 / p99 を併記する。"
        "新しく isolate が起きる分の跳ねを伏せると後で指摘される。"
    )
    out.append(
        "- **`--profile minimal` と `full` の両方を見る。** 差が profile で大きく変わるなら、"
        "その差は探索ではなく直列化と転送に由来している。"
    )
    out.append(
        "- **時間帯を変えて複数回測る。** `--append-jsonl` で貯めて `--summarize` でまとめられる。"
    )
    out.append("")
    return "\n".join(out)


def render_parity(result: dict) -> str:
    left, right = result["left"], result["right"]
    lines = [f"| クエリ | 判定 | 差分の種類 | 値の数 ({left} / {right}) | 例 |",
             "|---|:---:|---|---:|---|"]
    for case in result["cases"]:
        first, leaves = "", "-"
        for variant in case["variants"]:
            if "error" in variant:
                first = variant["error"]
                break
            if variant.get("diffs"):
                d = variant["diffs"][0]
                first = f"{d['path']} {d['detail']}"
                leaves = f"{variant['leaves'][left]} / {variant['leaves'][right]}"
                break
        kinds = " / ".join(KIND_LABEL[k] for k in case["kinds"]) or "-"
        mark = "○" if case["comparable"] else "×"
        lines.append(f"| `{case['name']}` | {mark} | {kinds} | {leaves} | {first} |")
    return "\n".join(lines)


def metadata(endpoints: dict[str, str], profile: str) -> dict:
    """どこで・いつ測ったかを結果に残す。数字だけ独り歩きさせないため。"""
    now = datetime.now(timezone.utc)
    return {
        "utc": now.isoformat(timespec="seconds"),
        "local": now.astimezone().isoformat(timespec="seconds"),
        "host": socket.gethostname(),
        "platform": platform.platform(),
        "python": platform.python_version(),
        "endpoints": endpoints,
        "profile": profile,
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
    names = list(runs[0]["endpoints"])
    by_name: dict[str, dict[str, list[float]]] = {}
    for run_data in runs:
        for case in run_data["cases"]:
            slot = by_name.setdefault(case["name"], {n: [] for n in names})
            for name in names:
                slot[name].append(case["results"][name]["p50_ms"])
    lines = [
        f"{len(runs)} 回分 ({runs[0]['meta']['local']} 〜 {runs[-1]['meta']['local']})",
        "",
        "| クエリ | " + " | ".join(f"{n} p50 中央値 (最小〜最大)" for n in names) + " | 倍率 |",
        "|---|" + "---|" * (len(names) + 1),
    ]
    for case_name, values in by_name.items():
        cells, medians = [], {}
        for name in names:
            medians[name] = statistics.median(values[name])
            cells.append(f"{medians[name]:.0f} ({min(values[name]):.0f}〜{max(values[name]):.0f})")
        ratio = (
            f"{medians[names[0]] / medians[names[1]]:.1f}x"
            if len(names) == 2 and medians[names[1]] > 0
            else "-"
        )
        lines.append(f"| `{case_name}` | " + " | ".join(cells) + f" | {ratio} |")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--iterations", type=int, default=30,
                        help="1 クエリあたりの計測回数 (引数の組に均等に割り振る)")
    parser.add_argument("--warmup", type=int, default=3, help="計測前に捨てる回数")
    parser.add_argument("--timeout", type=float, default=30.0, help="1 リクエストの上限秒")
    parser.add_argument("--profile", choices=PROFILES, default="full",
                        help="フィールド選択。minimal は応答量を削って探索の費用に寄せる")
    parser.add_argument("--production", default=DEFAULT_ENDPOINTS["production"])
    parser.add_argument("--staging", default=DEFAULT_ENDPOINTS["staging"])
    parser.add_argument("--only", nargs="*", help="対象のクエリ名を絞る")
    parser.add_argument("--list", action="store_true", help="クエリと引数の組を並べて終わる")
    parser.add_argument("--parity", action="store_true",
                        help="速度ではなく応答の一致を調べる")
    parser.add_argument("--parity-limit", type=int, default=8,
                        help="1 引数あたりに集める差分の数")
    parser.add_argument("--no-parity", action="store_true",
                        help="レポートに応答の一致を含めない (速度だけ測る)")
    parser.add_argument("--json-out", help="生の結果を JSON で書き出す先")
    parser.add_argument("--report-out", help="Markdown のレポートを書き出す先")
    parser.add_argument("--append-jsonl",
                        help="結果を JSONL に 1 行追記する (時間帯を変えて貯める用)")
    parser.add_argument("--summarize", help="--append-jsonl で貯めた JSONL をまとめて表示する")
    args = parser.parse_args()

    if args.summarize:
        print(summarize_jsonl(args.summarize))
        return 0

    cases = build_cases(args.profile)
    if args.list:
        for case in cases:
            print(f"{case.name:24} {case.note}")
            for variant in case.variants:
                print(f"  - {variant.label}")
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

    try:
        if args.parity:
            result = parity(endpoints, cases, args.parity_limit)
            result["meta"] = metadata(urls, args.profile)
            if args.json_out:
                with open(args.json_out, "w", encoding="utf-8") as fp:
                    json.dump(result, fp, ensure_ascii=False, indent=2)
            print(render_parity(result))
            blocked = [c["name"] for c in result["cases"] if not c["comparable"]]
            print()
            print(f"速度比較が成立する: {len(result['cases']) - len(blocked)} / "
                  f"{len(result['cases'])} クエリ")
            if blocked:
                print("成立しない (構造が違う): " + ", ".join(blocked))
            return 0

        report = run(endpoints, cases, args.iterations, args.warmup)
        report["meta"] = metadata(urls, args.profile)
        report["endpoints"] = urls
        report["profile"] = args.profile
        report["iterations"] = args.iterations
        report["warmup"] = args.warmup
        report["baseline_ping"] = baseline(endpoints, max(5, args.iterations // 3))

        if not args.no_parity:
            # 「本当に同じ仕事をしているのか」まで見ないと速度差を解釈できない。
            # 速度計測の後に回すので計測値には影響しない。
            print("応答の一致を確認中...", file=sys.stderr, flush=True)
            report["parity"] = parity(endpoints, cases, args.parity_limit, quiet=True)

        if args.json_out:
            with open(args.json_out, "w", encoding="utf-8") as fp:
                json.dump(report, fp, ensure_ascii=False, indent=2)
        if args.report_out:
            with open(args.report_out, "w", encoding="utf-8") as fp:
                fp.write(render_report(report, sys.argv))
            print(f"レポートを書き出した: {args.report_out}", file=sys.stderr)
        if args.append_jsonl:
            with open(args.append_jsonl, "a", encoding="utf-8") as fp:
                fp.write(json.dumps(report, ensure_ascii=False) + "\n")

        print(f"計測: {report['meta']['local']} / {report['meta']['host']} "
              f"/ profile={args.profile}")
        print()
        print(render_markdown(report))
        print()
        if report.get("parity"):
            names = list(urls)
            comp = _aggregate(report, names, only_comparable=True)
            print(f"応答が比較可能な {int(comp['_count'])} クエリの p50 合計: "
                  + " / ".join(f"{n} {comp[n]:.0f}ms" for n in names)
                  + (f" ({comp[names[0]] / comp[names[1]]:.1f}x)"
                     if comp[names[1]] > 0 else ""))
        for name, stats in report["baseline_ping"].items():
            print(
                f"baseline GET /__ping {name}: min={stats['min_ms']:.0f}ms "
                f"p50={stats['p50_ms']:.0f}ms p95={stats['p95_ms']:.0f}ms"
            )
        return 0
    finally:
        for endpoint in endpoints.values():
            endpoint.close()


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except BrokenPipeError:
        # head などで途中打ち切りされたとき。stderr を閉じておかないと
        # インタプリタ終了時に同じ例外がもう一度出る。
        os.dup2(os.open(os.devnull, os.O_WRONLY), sys.stdout.fileno())
        raise SystemExit(0)
