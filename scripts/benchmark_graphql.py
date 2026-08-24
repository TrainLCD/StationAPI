#!/usr/bin/env python3
"""本番 / ステージングの GraphQL エンドポイントをベンチマークする。

`schema/public.graphql` の Query ルートフィールド 18 種をすべて実行し、
2 つのエンドポイントのレイテンシを同一時間帯で交互に計測して比較する。

依存は Python 3 標準ライブラリのみ。使い方は scripts/README.md を参照。
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import http.client
import json
import os
import ssl
import statistics
import sys
import threading
import time
import urllib.parse
import zlib
from dataclasses import dataclass, field
from typing import Any

DEFAULT_PRODUCTION = "https://gql.trainlcd.app"
DEFAULT_STAGING = "https://gql-stg.trainlcd.app"
USER_AGENT = "stationapi-benchmark/1.0 (+scripts/benchmark_graphql.py)"

# ---------------------------------------------------------------------------
# フラグメント
#
# full  : TrainLCD 本体が実際に要求する程度の広い選択セット。
# light : id / 名前だけの最小セット。ペイロード量の影響を切り分けるために使う。
# ---------------------------------------------------------------------------

FRAGMENTS_FULL: dict[str, str] = {
    "CompanyFields": """
fragment CompanyFields on Company {
  id railroadId nameShort nameKatakana nameFull nameEnglishShort nameEnglishFull
  url type status name
}""",
    "LineFields": """
fragment LineFields on LineNested {
  id nameShort nameKatakana nameFull nameRoman nameChinese nameKorean color lineType
  status averageDistance transportType nameIpa nameRomanIpa
  lineSymbols { symbol color shape }
  nameTtsSegments { surface fallbackText pronunciation alphabet lang separator }
  company { ...CompanyFields }
}""",
    "TrainTypeNestedFields": """
fragment TrainTypeNestedFields on TrainTypeNested {
  id typeId groupId name nameKatakana nameRoman nameChinese nameKorean color
  direction kind nameIpa nameRomanIpa
}""",
    "StationNumberFields": """
fragment StationNumberFields on StationNumber {
  lineSymbol lineSymbolColor lineSymbolShape stationNumber
}""",
    "StationFields": """
fragment StationFields on Station {
  id groupId name nameKatakana nameRoman nameChinese nameKorean threeLetterCode
  prefectureId postalCode address latitude longitude openedAt closedAt status
  stopCondition distance hasTrainTypes transportType nameIpa nameRomanIpa
  stationNumbers { ...StationNumberFields }
  nameTtsSegments { surface fallbackText pronunciation alphabet lang separator }
  trainType { ...TrainTypeNestedFields }
  line { ...LineFields }
  lines { ...LineFields }
}""",
    "StationNestedFields": """
fragment StationNestedFields on StationNested {
  id groupId name nameKatakana nameRoman nameChinese nameKorean threeLetterCode
  prefectureId postalCode address latitude longitude openedAt closedAt status
  stopCondition distance hasTrainTypes transportType nameIpa nameRomanIpa
  stationNumbers { ...StationNumberFields }
  nameTtsSegments { surface fallbackText pronunciation alphabet lang separator }
  trainType { ...TrainTypeNestedFields }
  line { ...LineFields }
  lines { ...LineFields }
}""",
    "LineRootFields": """
fragment LineRootFields on Line {
  id nameShort nameKatakana nameFull nameRoman nameChinese nameKorean color lineType
  status averageDistance transportType nameIpa nameRomanIpa
  lineSymbols { symbol color shape }
  nameTtsSegments { surface fallbackText pronunciation alphabet lang separator }
  company { ...CompanyFields }
  trainType { ...TrainTypeNestedFields }
  station { ...StationNestedFields }
}""",
    "TrainTypeRootFields": """
fragment TrainTypeRootFields on TrainType {
  id typeId groupId name nameKatakana nameRoman nameChinese nameKorean color
  direction kind nameIpa nameRomanIpa
  nameTtsSegments { surface fallbackText pronunciation alphabet lang separator }
  line { ...LineFields }
  lines { ...LineFields }
}""",
}

FRAGMENTS_LIGHT: dict[str, str] = {
    "CompanyFields": "fragment CompanyFields on Company { id nameShort }",
    "LineFields": "fragment LineFields on LineNested { id nameShort }",
    "TrainTypeNestedFields": "fragment TrainTypeNestedFields on TrainTypeNested { id name }",
    "StationNumberFields": "fragment StationNumberFields on StationNumber { stationNumber }",
    "StationFields": "fragment StationFields on Station { id groupId name }",
    "StationNestedFields": "fragment StationNestedFields on StationNested { id groupId name }",
    "LineRootFields": "fragment LineRootFields on Line { id nameShort }",
    "TrainTypeRootFields": "fragment TrainTypeRootFields on TrainType { id name }",
}

PROFILES = {"full": FRAGMENTS_FULL, "light": FRAGMENTS_LIGHT}

# ---------------------------------------------------------------------------
# 計測ケース
#
# パラメータは data/*.csv 由来の実在する ID。JR 東日本の主要駅・路線を使い、
# 「小さい応答」と「大きい応答」の両方が出るように組んである。
# --cases file.json で丸ごと差し替えられる。
# ---------------------------------------------------------------------------

TOKYO_GROUP = 1130101  # 東京 (station_g_cd)
SHINJUKU_GROUP = 1130208  # 新宿 (station_g_cd)
YAMANOTE_SHINJUKU = 1130208  # 新宿 / 山手線 (station_cd)
CHUO_TOKYO = 1131201  # 東京 / 中央線快速 (station_cd)
CHUO_SHINJUKU = 1131211  # 新宿 / 中央線快速 (station_cd)
TOKAIDO_TOKYO = 1130101  # 東京 / 東海道本線 (station_cd)
TOKAIDO_FAR = 1170113  # 東海道本線 系統 203 の終端側 (station_cd)
YAMANOTE_LINE = 11302
CHUO_RAPID_LINE = 11312
CHUO_RAPID_GROUP = 20  # 41 駅
KEIHIN_TOHOKU_GROUP = 21
TOKAIDO_LONG_GROUP = 203  # 250 駅。最も重い系統のひとつ

BUILTIN_CASES: list[dict[str, Any]] = [
    {
        "id": "station",
        "note": "単一駅取得（最小構成の代表）",
        "operation": "query Station($id: Int!) { station(id: $id) { ...StationFields } }",
        "variables": {"id": YAMANOTE_SHINJUKU},
    },
    {
        "id": "stations",
        "note": "ID 5 件のバッチ取得",
        "operation": "query Stations($ids: [Int!]!) { stations(ids: $ids) { ...StationFields } }",
        "variables": {"ids": [1130101, 1130208, 1130205, 1131201, 1132104]},
    },
    {
        "id": "stationsNearby",
        "note": "東京駅周辺 10 件。座標グリッド索引の代表",
        "operation": (
            "query StationsNearby($lat: Float!, $lon: Float!, $limit: Int) {"
            " stationsNearby(latitude: $lat, longitude: $lon, limit: $limit) { ...StationFields } }"
        ),
        "variables": {"lat": 35.681382, "lon": 139.766084, "limit": 10},
    },
    {
        "id": "stationsNearby:heavy",
        "note": "同上を 100 件。グリッド探索の半径拡大が効く",
        "operation": (
            "query StationsNearbyHeavy($lat: Float!, $lon: Float!, $limit: Int) {"
            " stationsNearby(latitude: $lat, longitude: $lon, limit: $limit) { ...StationFields } }"
        ),
        "variables": {"lat": 35.681382, "lon": 139.766084, "limit": 100},
    },
    {
        "id": "stationsByName",
        "note": "名前検索（正規化・部分一致）",
        "operation": (
            "query StationsByName($name: String!, $limit: Int) {"
            " stationsByName(name: $name, limit: $limit) { ...StationFields } }"
        ),
        "variables": {"name": "新宿", "limit": 10},
    },
    {
        "id": "stationGroupStations",
        "note": "新宿の同一駅グループ全件",
        "operation": (
            "query StationGroupStations($groupId: Int!) {"
            " stationGroupStations(groupId: $groupId) { ...StationFields } }"
        ),
        "variables": {"groupId": SHINJUKU_GROUP},
    },
    {
        "id": "lineGroupStations",
        "note": "中央線快速 系統（41 駅）",
        "operation": (
            "query LineGroupStations($lineGroupId: Int!) {"
            " lineGroupStations(lineGroupId: $lineGroupId) { ...StationFields } }"
        ),
        "variables": {"lineGroupId": CHUO_RAPID_GROUP},
    },
    {
        "id": "lineGroupStations:heavy",
        "note": "東海道本線 系統（250 駅）。全クエリ中で最大級の応答",
        "operation": (
            "query LineGroupStationsHeavy($lineGroupId: Int!) {"
            " lineGroupStations(lineGroupId: $lineGroupId) { ...StationFields } }"
        ),
        "variables": {"lineGroupId": TOKAIDO_LONG_GROUP},
    },
    {
        "id": "line",
        "note": "単一路線取得",
        "operation": "query Line($lineId: Int!) { line(lineId: $lineId) { ...LineRootFields } }",
        "variables": {"lineId": YAMANOTE_LINE},
    },
    {
        "id": "lines",
        "note": "路線 5 件のバッチ取得",
        "operation": "query Lines($lineIds: [Int!]!) { lines(lineIds: $lineIds) { ...LineRootFields } }",
        "variables": {"lineIds": [11302, 11301, 11312, 11332, 99336]},
    },
    {
        "id": "linesByName",
        "note": "路線名検索",
        "operation": (
            "query LinesByName($name: String!, $limit: Int) {"
            " linesByName(name: $name, limit: $limit) { ...LineRootFields } }"
        ),
        "variables": {"name": "山手", "limit": 10},
    },
    {
        "id": "lineStations",
        "note": "山手線の駅一覧（各停系統フォールバックを含む）",
        "operation": (
            "query LineStations($lineId: Int!) { lineStations(lineId: $lineId) { ...StationFields } }"
        ),
        "variables": {"lineId": YAMANOTE_LINE},
    },
    {
        "id": "lineListStations",
        "note": "複数路線の駅一覧をまとめて取得",
        "operation": (
            "query LineListStations($lineIds: [Int!]!) {"
            " lineListStations(lineIds: $lineIds) { ...StationFields } }"
        ),
        "variables": {"lineIds": [YAMANOTE_LINE, CHUO_RAPID_LINE]},
    },
    {
        "id": "lineGroupListStations",
        "note": "複数系統の駅一覧をまとめて取得",
        "operation": (
            "query LineGroupListStations($lineGroupIds: [Int!]!) {"
            " lineGroupListStations(lineGroupIds: $lineGroupIds) { ...StationFields } }"
        ),
        "variables": {"lineGroupIds": [CHUO_RAPID_GROUP, KEIHIN_TOHOKU_GROUP]},
    },
    {
        "id": "stationTrainTypes",
        "note": "東京駅（中央線）の種別一覧",
        "operation": (
            "query StationTrainTypes($stationId: Int!) {"
            " stationTrainTypes(stationId: $stationId) { ...TrainTypeRootFields } }"
        ),
        "variables": {"stationId": CHUO_TOKYO},
    },
    {
        "id": "routes",
        "note": "東京 → 新宿 の経路探索",
        "operation": (
            "query Routes($from: Int!, $to: Int!) {"
            " routes(fromStationGroupId: $from, toStationGroupId: $to) {"
            " nextPageToken routes { id stops { ...StationNestedFields } } } }"
        ),
        "variables": {"from": TOKYO_GROUP, "to": SHINJUKU_GROUP},
    },
    {
        "id": "routeTypes",
        "note": "東京 → 新宿 の種別一覧",
        "operation": (
            "query RouteTypes($from: Int!, $to: Int!) {"
            " routeTypes(fromStationGroupId: $from, toStationGroupId: $to) {"
            " nextPageToken trainTypes { ...TrainTypeRootFields } } }"
        ),
        "variables": {"from": TOKYO_GROUP, "to": SHINJUKU_GROUP},
    },
    {
        "id": "connectedRoutes",
        "note": "乗り継ぎ探索（幅優先探索）。計算量が最も大きいクエリ",
        "operation": (
            "query ConnectedRoutes($from: Int!, $to: Int!) {"
            " connectedRoutes(fromStationGroupId: $from, toStationGroupId: $to) {"
            " id stops { ...StationNestedFields } } }"
        ),
        "variables": {"from": TOKYO_GROUP, "to": SHINJUKU_GROUP},
    },
    {
        "id": "estimateArrivalTimes",
        "note": "東京 → 新宿（中央線快速）の到着時刻推定",
        "operation": (
            "query EstimateArrivalTimes($from: Int!, $to: Int!) {"
            " estimateArrivalTimes(fromStationId: $from, toStationId: $to) {"
            " routes { id stops { stationId stationGroupId cumulativeMinutes stopsHere"
            " departureCumulativeMinutes } } } }"
        ),
        "variables": {"from": CHUO_TOKYO, "to": CHUO_SHINJUKU},
    },
    {
        "id": "trainRoute",
        "note": "東京 → 新宿。系統長ではなく要求区間に比例するはずの区間",
        "operation": (
            "query TrainRoute($from: Int!, $to: Int!, $lineGroupId: Int) {"
            " trainRoute(fromStationId: $from, toStationId: $to, lineGroupId: $lineGroupId) {"
            " segments { stops distanceFromPrevious maxSpeed maxAcceleration maxDeceleration"
            " station { ...StationNestedFields } } } }"
        ),
        "variables": {"from": CHUO_TOKYO, "to": CHUO_SHINJUKU, "lineGroupId": CHUO_RAPID_GROUP},
    },
    {
        "id": "trainRoute:long",
        "note": "東海道本線を端から端まで。trainRoute の上限側",
        "operation": (
            "query TrainRouteLong($from: Int!, $to: Int!, $lineGroupId: Int) {"
            " trainRoute(fromStationId: $from, toStationId: $to, lineGroupId: $lineGroupId) {"
            " segments { stops distanceFromPrevious maxSpeed maxAcceleration maxDeceleration"
            " station { ...StationNestedFields } } } }"
        ),
        "variables": {
            "from": TOKAIDO_TOKYO,
            "to": TOKAIDO_FAR,
            "lineGroupId": TOKAIDO_LONG_GROUP,
        },
    },
]

# ---------------------------------------------------------------------------
# クエリ組み立て
# ---------------------------------------------------------------------------


def build_document(operation: str, fragments: dict[str, str]) -> str:
    """operation が参照するフラグメントだけを再帰的に集めて連結する。

    未使用のフラグメント定義はバリデーションエラーになるため、使うものだけを付ける。
    """
    used: list[str] = []
    seen: set[str] = set()
    pending = _spread_names(operation)
    while pending:
        name = pending.pop(0)
        if name in seen:
            continue
        seen.add(name)
        try:
            body = fragments[name]
        except KeyError:  # pragma: no cover - 定義ミスの早期検知用
            raise SystemExit(f"未定義のフラグメント: {name}")
        used.append(body.strip())
        pending.extend(_spread_names(body))
    return "\n".join([operation.strip(), *used]) + "\n"


def _spread_names(text: str) -> list[str]:
    names: list[str] = []
    index = 0
    while True:
        index = text.find("...", index)
        if index < 0:
            return names
        index += 3
        end = index
        while end < len(text) and (text[end].isalnum() or text[end] == "_"):
            end += 1
        if end > index:
            names.append(text[index:end])


def filter_cases(cases: list["Case"], expression: str) -> list["Case"]:
    """--only の絞り込み。完全一致があればそれを優先し、無ければ前方一致で拾う。

    `station` を指定して `stationsNearby` まで付いてくると意図と食い違うため、
    完全一致するトークンは前方一致に広げない。
    """
    selected: list[Case] = []
    known = {case.id for case in cases}
    for token in (x.strip() for x in expression.split(",")):
        if not token:
            continue
        if token in known:
            matched = [c for c in cases if c.id == token]
        else:
            matched = [c for c in cases if c.id.startswith(token)]
        for case in matched:
            if case not in selected:
                selected.append(case)
    return selected


@dataclass
class Case:
    id: str
    note: str
    document: str
    variables: dict[str, Any]


def load_cases(profile: str, path: str | None) -> list[Case]:
    raw = BUILTIN_CASES
    if path:
        with open(path, encoding="utf-8") as handle:
            raw = json.load(handle)
    fragments = PROFILES[profile]
    return [
        Case(
            id=item["id"],
            note=item.get("note", ""),
            document=build_document(item["operation"], fragments),
            variables=item.get("variables", {}),
        )
        for item in raw
    ]


# ---------------------------------------------------------------------------
# HTTP クライアント
# ---------------------------------------------------------------------------


@dataclass
class Sample:
    ok: bool
    ttfb_ms: float = 0.0
    total_ms: float = 0.0
    status: int = 0
    wire_bytes: int = 0
    body_bytes: int = 0
    error: str = ""
    digest: str = ""
    headers: dict[str, str] = field(default_factory=dict)


class Endpoint:
    """1 エンドポイントぶんの持続接続クライアント。

    HTTPS_PROXY があれば CONNECT トンネルを張る。接続確立は計測区間の外で行い、
    サンプルにはリクエスト送信〜本文読み切りだけが含まれるようにする。
    """

    def __init__(self, name: str, url: str, timeout: float, keepalive: bool, compression: str):
        self.name = name
        self.url = url.rstrip("/")
        parsed = urllib.parse.urlsplit(self.url)
        if parsed.scheme not in ("http", "https"):
            raise SystemExit(f"対応していない URL です: {url}")
        self.scheme = parsed.scheme
        self.host = parsed.hostname or ""
        self.port = parsed.port or (443 if parsed.scheme == "https" else 80)
        self.path = parsed.path or "/"
        self.timeout = timeout
        self.keepalive = keepalive
        self.compression = compression
        self._local = threading.local()
        self._proxy = self._resolve_proxy()

    def _resolve_proxy(self) -> tuple[str, int] | None:
        key = "https_proxy" if self.scheme == "https" else "http_proxy"
        value = os.environ.get(key) or os.environ.get(key.upper())
        if not value:
            return None
        no_proxy = os.environ.get("no_proxy") or os.environ.get("NO_PROXY") or ""
        for entry in no_proxy.split(","):
            entry = entry.strip().lstrip("*").lstrip(".")
            if entry and (self.host == entry or self.host.endswith("." + entry)):
                return None
        parsed = urllib.parse.urlsplit(value if "://" in value else "http://" + value)
        return (parsed.hostname or "", parsed.port or 80)

    def _ssl_context(self) -> ssl.SSLContext:
        cafile = (
            os.environ.get("SSL_CERT_FILE")
            or os.environ.get("REQUESTS_CA_BUNDLE")
            or os.environ.get("CURL_CA_BUNDLE")
        )
        return ssl.create_default_context(cafile=cafile if cafile and os.path.exists(cafile) else None)

    def connect(self) -> http.client.HTTPConnection:
        conn = getattr(self._local, "conn", None)
        if conn is not None:
            return conn
        if self.scheme == "https":
            if self._proxy:
                conn = http.client.HTTPSConnection(
                    self._proxy[0], self._proxy[1], timeout=self.timeout, context=self._ssl_context()
                )
                conn.set_tunnel(self.host, self.port)
            else:
                conn = http.client.HTTPSConnection(
                    self.host, self.port, timeout=self.timeout, context=self._ssl_context()
                )
        else:
            if self._proxy:
                conn = http.client.HTTPConnection(self._proxy[0], self._proxy[1], timeout=self.timeout)
                conn.set_tunnel(self.host, self.port)
            else:
                conn = http.client.HTTPConnection(self.host, self.port, timeout=self.timeout)
        conn.connect()
        self._local.conn = conn
        return conn

    def close(self) -> None:
        conn = getattr(self._local, "conn", None)
        if conn is not None:
            try:
                conn.close()
            finally:
                self._local.conn = None

    def request(self, case: Case) -> Sample:
        payload = json.dumps(
            {"query": case.document, "variables": case.variables, "operationName": None},
            ensure_ascii=False,
        ).encode("utf-8")
        headers = {
            "content-type": "application/json",
            "accept": "application/json",
            "accept-encoding": "gzip, deflate" if self.compression == "gzip" else "identity",
            "user-agent": USER_AGENT,
            "connection": "keep-alive" if self.keepalive else "close",
            "content-length": str(len(payload)),
        }
        try:
            conn = self.connect()
        except Exception as exc:  # 接続自体の失敗
            self.close()
            return Sample(ok=False, error=f"connect: {exc}")

        started = time.perf_counter()
        try:
            conn.request("POST", self.path, body=payload, headers=headers)
            response = conn.getresponse()
            ttfb = time.perf_counter() - started
            raw = response.read()
            total = time.perf_counter() - started
        except Exception as exc:
            self.close()
            return Sample(ok=False, error=f"{type(exc).__name__}: {exc}")

        if not self.keepalive or response.will_close:
            self.close()

        body = _decode(raw, response.headers.get("content-encoding", ""))
        sample = Sample(
            ok=False,
            ttfb_ms=ttfb * 1000.0,
            total_ms=total * 1000.0,
            status=response.status,
            wire_bytes=len(raw),
            body_bytes=len(body),
            headers={
                k: v
                for k, v in response.getheaders()
                if k.lower() in ("cf-ray", "cf-cache-status", "server-timing", "content-encoding")
            },
        )
        if response.status != 200:
            sample.error = f"HTTP {response.status}: {body[:180].decode('utf-8', 'replace')}"
            return sample
        try:
            parsed = json.loads(body)
        except json.JSONDecodeError as exc:
            sample.error = f"JSON 解析失敗: {exc}"
            return sample
        if parsed.get("errors"):
            first = parsed["errors"][0]
            sample.error = "GraphQL: " + str(first.get("message", first))[:180]
            return sample
        data = parsed.get("data")
        if data is None:
            sample.error = "data が null"
            return sample
        sample.ok = True
        sample.digest = hashlib.sha256(
            json.dumps(data, sort_keys=True, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
        ).hexdigest()
        return sample


def _decode(raw: bytes, encoding: str) -> bytes:
    encoding = encoding.lower()
    try:
        if encoding == "gzip":
            return gzip.decompress(raw)
        if encoding == "deflate":
            return zlib.decompress(raw)
    except (OSError, zlib.error):
        return raw
    return raw


# ---------------------------------------------------------------------------
# 集計
# ---------------------------------------------------------------------------


def percentile(values: list[float], pct: float) -> float:
    """最近傍順位法。サンプル数が少ない実測でも解釈がぶれない。"""
    if not values:
        return float("nan")
    ordered = sorted(values)
    rank = max(1, min(len(ordered), int(-(-pct / 100.0 * len(ordered) // 1))))
    return ordered[rank - 1]


@dataclass
class Result:
    case_id: str
    endpoint: str
    samples: list[Sample] = field(default_factory=list)
    wall_seconds: float = 0.0

    @property
    def ok_samples(self) -> list[Sample]:
        return [s for s in self.samples if s.ok]

    @property
    def totals(self) -> list[float]:
        return [s.total_ms for s in self.ok_samples]

    @property
    def error_count(self) -> int:
        return len(self.samples) - len(self.ok_samples)

    @property
    def first_error(self) -> str:
        for sample in self.samples:
            if not sample.ok:
                return sample.error
        return ""

    @property
    def digest(self) -> str:
        return self.ok_samples[0].digest if self.ok_samples else ""

    def stats(self) -> dict[str, float]:
        values = self.totals
        if not values:
            return {}
        return {
            "min": min(values),
            "p50": percentile(values, 50),
            "p90": percentile(values, 90),
            "p95": percentile(values, 95),
            "p99": percentile(values, 99),
            "max": max(values),
            "mean": statistics.fmean(values),
            "stdev": statistics.stdev(values) if len(values) > 1 else 0.0,
            "ttfb_p50": percentile([s.ttfb_ms for s in self.ok_samples], 50),
            "wire_bytes": statistics.fmean([s.wire_bytes for s in self.ok_samples]),
            "body_bytes": statistics.fmean([s.body_bytes for s in self.ok_samples]),
            "rps": (len(values) / self.wall_seconds) if self.wall_seconds > 0 else 0.0,
        }


# ---------------------------------------------------------------------------
# 実行
# ---------------------------------------------------------------------------


def run_sequential(
    cases: list[Case], endpoints: list[Endpoint], iterations: int, warmup: int, sleep: float, verbose: bool
) -> dict[tuple[str, str], Result]:
    results: dict[tuple[str, str], Result] = {
        (case.id, ep.name): Result(case.id, ep.name) for case in cases for ep in endpoints
    }
    for case in cases:
        if verbose:
            print(f"  {case.id} ...", file=sys.stderr, flush=True)
        for endpoint in endpoints:
            for _ in range(warmup):
                endpoint.request(case)
        started = time.perf_counter()
        for i in range(iterations):
            # 時間帯による偏りを打ち消すため、反復ごとにエンドポイントの順序を入れ替える。
            order = endpoints if i % 2 == 0 else list(reversed(endpoints))
            for endpoint in order:
                results[(case.id, endpoint.name)].samples.append(endpoint.request(case))
                if sleep:
                    time.sleep(sleep)
        elapsed = time.perf_counter() - started
        for endpoint in endpoints:
            # 交互実行なので RPS は「もう一方も動かしながらの実効値」。
            results[(case.id, endpoint.name)].wall_seconds = elapsed
    for endpoint in endpoints:
        endpoint.close()
    return results


def run_concurrent(
    cases: list[Case],
    endpoints: list[Endpoint],
    iterations: int,
    warmup: int,
    concurrency: int,
    verbose: bool,
) -> dict[tuple[str, str], Result]:
    results: dict[tuple[str, str], Result] = {
        (case.id, ep.name): Result(case.id, ep.name) for case in cases for ep in endpoints
    }
    for case in cases:
        if verbose:
            print(f"  {case.id} ...", file=sys.stderr, flush=True)
        for endpoint in endpoints:
            for _ in range(warmup):
                endpoint.request(case)
            endpoint.close()

            lock = threading.Lock()
            collected: list[Sample] = []
            per_thread = [iterations // concurrency] * concurrency
            for i in range(iterations % concurrency):
                per_thread[i] += 1

            def worker(count: int) -> None:
                local: list[Sample] = []
                for _ in range(count):
                    local.append(endpoint.request(case))
                endpoint.close()
                with lock:
                    collected.extend(local)

            threads = [threading.Thread(target=worker, args=(n,)) for n in per_thread if n > 0]
            started = time.perf_counter()
            for thread in threads:
                thread.start()
            for thread in threads:
                thread.join()
            elapsed = time.perf_counter() - started
            result = results[(case.id, endpoint.name)]
            result.samples = collected
            result.wall_seconds = elapsed
    return results


# ---------------------------------------------------------------------------
# レポート
# ---------------------------------------------------------------------------


def format_ms(value: float | None) -> str:
    if value is None:
        return "-"
    return f"{value:,.1f}"


def format_bytes(value: float) -> str:
    if value >= 1024 * 1024:
        return f"{value / 1024 / 1024:.2f} MiB"
    if value >= 1024:
        return f"{value / 1024:.1f} KiB"
    return f"{value:.0f} B"


def render_markdown(
    cases: list[Case],
    endpoints: list[Endpoint],
    results: dict[tuple[str, str], Result],
    meta: dict[str, Any],
) -> str:
    base, other = endpoints[0], endpoints[1] if len(endpoints) > 1 else None
    lines: list[str] = []
    lines.append("# GraphQL ベンチマーク結果")
    lines.append("")
    lines.append(f"- 実行日時: {meta['timestamp']}")
    for endpoint in endpoints:
        lines.append(f"- {endpoint.name}: `{endpoint.url}`")
    lines.append(
        f"- 反復: {meta['iterations']} 回/クエリ・エンドポイント"
        f"（ウォームアップ {meta['warmup']} 回、プロファイル `{meta['profile']}`、"
        f"並列 {meta['concurrency']}、圧縮 `{meta['compression']}`）"
    )
    lines.append(f"- 計測地点: クライアント側の往復時間（{meta['client']}）")
    lines.append("")

    if other is not None:
        lines.append(f"## サマリ（{base.name} vs {other.name}）")
        lines.append("")
        lines.append(
            f"| クエリ | {base.name} p50 | {other.name} p50 | 差 | "
            f"{base.name} p95 | {other.name} p95 | 応答量 | 一致 | エラー |"
        )
        lines.append("| --- | ---: | ---: | ---: | ---: | ---: | ---: | :--: | ---: |")
        for case in cases:
            a = results[(case.id, base.name)]
            b = results[(case.id, other.name)]
            sa, sb = a.stats(), b.stats()
            if sa and sb:
                delta = sb["p50"] - sa["p50"]
                ratio = sb["p50"] / sa["p50"] if sa["p50"] else float("nan")
                delta_text = f"{delta:+,.1f} ({ratio:.2f}×)"
            else:
                delta_text = "-"
            digest_text = "-"
            if a.digest and b.digest:
                digest_text = "✅" if a.digest == b.digest else "⚠️"
            errors = a.error_count + b.error_count
            lines.append(
                f"| `{case.id}` | {format_ms(sa.get('p50')) if sa else '-'} "
                f"| {format_ms(sb.get('p50')) if sb else '-'} | {delta_text} "
                f"| {format_ms(sa.get('p95')) if sa else '-'} "
                f"| {format_ms(sb.get('p95')) if sb else '-'} "
                f"| {format_bytes(sa['body_bytes']) if sa else '-'} "
                f"| {digest_text} | {errors if errors else ''} |"
            )
        lines.append("")
        lines.append("単位はミリ秒。「差」は本番を基準にしたステージングの差分と倍率。")
        lines.append("「一致」は両環境の `data` を正規化した SHA-256 の突き合わせ結果。")
        lines.append("")

    lines.append("## エンドポイント別の詳細")
    lines.append("")
    for endpoint in endpoints:
        lines.append(f"### {endpoint.name} — `{endpoint.url}`")
        lines.append("")
        lines.append("| クエリ | min | p50 | p90 | p95 | p99 | max | 平均 | 標準偏差 | TTFB p50 | 転送量 | 実効 RPS | 失敗 |")
        lines.append("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |")
        for case in cases:
            result = results[(case.id, endpoint.name)]
            stats = result.stats()
            if not stats:
                lines.append(
                    f"| `{case.id}` | - | - | - | - | - | - | - | - | - | - | - | "
                    f"{result.error_count} |"
                )
                continue
            lines.append(
                "| `{id}` | {min} | {p50} | {p90} | {p95} | {p99} | {max} | {mean} | {stdev} "
                "| {ttfb} | {wire} | {rps:.1f} | {err} |".format(
                    id=case.id,
                    min=format_ms(stats["min"]),
                    p50=format_ms(stats["p50"]),
                    p90=format_ms(stats["p90"]),
                    p95=format_ms(stats["p95"]),
                    p99=format_ms(stats["p99"]),
                    max=format_ms(stats["max"]),
                    mean=format_ms(stats["mean"]),
                    stdev=format_ms(stats["stdev"]),
                    ttfb=format_ms(stats["ttfb_p50"]),
                    wire=format_bytes(stats["wire_bytes"]),
                    rps=stats["rps"],
                    err=result.error_count or "",
                )
            )
        lines.append("")

    failures = [
        (case.id, ep.name, results[(case.id, ep.name)].first_error)
        for case in cases
        for ep in endpoints
        if results[(case.id, ep.name)].error_count
    ]
    if failures:
        lines.append("## 失敗したリクエスト")
        lines.append("")
        lines.append("| クエリ | エンドポイント | 最初のエラー |")
        lines.append("| --- | --- | --- |")
        for case_id, ep_name, message in failures:
            escaped = message.replace("|", "\\|")
            lines.append(f"| `{case_id}` | {ep_name} | {escaped} |")
        lines.append("")

    mismatches = []
    if other is not None:
        for case in cases:
            a = results[(case.id, base.name)]
            b = results[(case.id, other.name)]
            if a.digest and b.digest and a.digest != b.digest:
                mismatches.append(case.id)
    if mismatches:
        lines.append("## 応答内容が一致しなかったクエリ")
        lines.append("")
        lines.append(
            "以下は両環境で `data` が異なった。デプロイ時期やデータセットの差である可能性が高い。"
        )
        lines.append("")
        for case_id in mismatches:
            lines.append(f"- `{case_id}`")
        lines.append("")

    lines.append("## クエリ定義")
    lines.append("")
    lines.append("| クエリ | 説明 | 変数 |")
    lines.append("| --- | --- | --- |")
    for case in cases:
        variables = json.dumps(case.variables, ensure_ascii=False)
        lines.append(f"| `{case.id}` | {case.note} | `{variables}` |")
    lines.append("")
    return "\n".join(lines)


def to_json(
    cases: list[Case],
    endpoints: list[Endpoint],
    results: dict[tuple[str, str], Result],
    meta: dict[str, Any],
    include_samples: bool,
) -> dict[str, Any]:
    payload: dict[str, Any] = {
        "meta": meta,
        "endpoints": [{"name": e.name, "url": e.url} for e in endpoints],
        "cases": [],
    }
    for case in cases:
        entry: dict[str, Any] = {
            "id": case.id,
            "note": case.note,
            "variables": case.variables,
            "document": case.document,
            "endpoints": {},
        }
        for endpoint in endpoints:
            result = results[(case.id, endpoint.name)]
            record: dict[str, Any] = {
                "stats": result.stats(),
                "errors": result.error_count,
                "first_error": result.first_error,
                "digest": result.digest,
                "wall_seconds": result.wall_seconds,
                "headers": result.ok_samples[0].headers if result.ok_samples else {},
            }
            if include_samples:
                record["samples"] = [
                    {
                        "ok": s.ok,
                        "status": s.status,
                        "ttfb_ms": s.ttfb_ms,
                        "total_ms": s.total_ms,
                        "wire_bytes": s.wire_bytes,
                        "body_bytes": s.body_bytes,
                        "error": s.error,
                    }
                    for s in result.samples
                ]
            entry["endpoints"][endpoint.name] = record
        payload["cases"].append(entry)
    return payload


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="本番 / ステージングの GraphQL エンドポイントをベンチマークする"
    )
    parser.add_argument("--production", default=DEFAULT_PRODUCTION, help="本番の URL")
    parser.add_argument("--staging", default=DEFAULT_STAGING, help="ステージングの URL")
    parser.add_argument(
        "--endpoint",
        action="append",
        metavar="NAME=URL",
        help="計測先を明示指定する（複数可）。指定すると --production / --staging は無視される",
    )
    parser.add_argument("-n", "--iterations", type=int, default=20, help="1 クエリあたりの計測回数")
    parser.add_argument("-w", "--warmup", type=int, default=3, help="計測に含めないウォームアップ回数")
    parser.add_argument("-c", "--concurrency", type=int, default=1, help="並列数。2 以上でスループット計測")
    parser.add_argument("--profile", choices=sorted(PROFILES), default="full", help="選択セットの広さ")
    parser.add_argument("--only", help="実行するクエリ ID をカンマ区切りで指定（前方一致）")
    parser.add_argument("--cases", help="ケース定義 JSON のパス（組み込み定義を置き換える）")
    parser.add_argument("--timeout", type=float, default=30.0, help="1 リクエストのタイムアウト秒")
    parser.add_argument("--sleep", type=float, default=0.0, help="リクエスト間の待機秒")
    parser.add_argument("--compression", choices=("gzip", "identity"), default="gzip")
    parser.add_argument("--no-keepalive", action="store_true", help="毎回接続を張り直す")
    parser.add_argument("--json", dest="json_out", help="結果 JSON の出力先")
    parser.add_argument("--markdown", dest="md_out", help="結果 Markdown の出力先")
    parser.add_argument("--raw-samples", action="store_true", help="JSON に全サンプルを含める")
    parser.add_argument("--list", action="store_true", help="ケース一覧を表示して終了")
    parser.add_argument("--print-query", metavar="ID", help="指定ケースのクエリ本文を表示して終了")
    parser.add_argument("-q", "--quiet", action="store_true", help="進捗表示を抑制する")
    args = parser.parse_args(argv)

    cases = load_cases(args.profile, args.cases)
    if args.only:
        cases = filter_cases(cases, args.only)
        if not cases:
            parser.error("--only に一致するケースがありません")

    if args.list:
        for case in cases:
            print(f"{case.id}\t{case.note}")
        return 0
    if args.print_query:
        for case in cases:
            if case.id == args.print_query:
                print(case.document)
                print("# variables: " + json.dumps(case.variables, ensure_ascii=False))
                return 0
        parser.error(f"ケースが見つかりません: {args.print_query}")

    if args.iterations < 1:
        parser.error("--iterations は 1 以上を指定してください")
    if args.concurrency < 1:
        parser.error("--concurrency は 1 以上を指定してください")

    specs: list[tuple[str, str]]
    if args.endpoint:
        specs = []
        for item in args.endpoint:
            if "=" not in item:
                parser.error(f"--endpoint は NAME=URL 形式です: {item}")
            name, url = item.split("=", 1)
            specs.append((name, url))
    else:
        specs = [("production", args.production), ("staging", args.staging)]

    endpoints = [
        Endpoint(name, url, args.timeout, not args.no_keepalive, args.compression)
        for name, url in specs
    ]

    meta = {
        "timestamp": time.strftime("%Y-%m-%d %H:%M:%S %z"),
        "iterations": args.iterations,
        "warmup": args.warmup,
        "concurrency": args.concurrency,
        "profile": args.profile,
        "compression": args.compression,
        "keepalive": not args.no_keepalive,
        "client": f"python {sys.version.split()[0]} on {sys.platform}",
        "proxy": os.environ.get("HTTPS_PROXY") or os.environ.get("https_proxy") or "",
    }

    if not args.quiet:
        print(
            f"{len(cases)} クエリ × {len(endpoints)} エンドポイント × {args.iterations} 回を計測します",
            file=sys.stderr,
        )
    if args.concurrency > 1:
        results = run_concurrent(
            cases, endpoints, args.iterations, args.warmup, args.concurrency, not args.quiet
        )
    else:
        results = run_sequential(
            cases, endpoints, args.iterations, args.warmup, args.sleep, not args.quiet
        )

    markdown = render_markdown(cases, endpoints, results, meta)
    if args.md_out:
        with open(args.md_out, "w", encoding="utf-8") as handle:
            handle.write(markdown + "\n")
        if not args.quiet:
            print(f"Markdown を書き出しました: {args.md_out}", file=sys.stderr)
    else:
        print(markdown)
    if args.json_out:
        with open(args.json_out, "w", encoding="utf-8") as handle:
            json.dump(
                to_json(cases, endpoints, results, meta, args.raw_samples),
                handle,
                ensure_ascii=False,
                indent=2,
            )
            handle.write("\n")
        if not args.quiet:
            print(f"JSON を書き出しました: {args.json_out}", file=sys.stderr)

    failed = sum(results[(c.id, e.name)].error_count for c in cases for e in endpoints)
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
