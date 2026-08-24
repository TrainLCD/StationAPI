#!/usr/bin/env python3
"""本番 (gql.trainlcd.app) とステージング (gql-stg.trainlcd.app) の GraphQL クエリ性能を比較する。

クライアント側の応答時間だけでなく、Cloudflare Worker の CPU Time も測る。
CPU Time は `wrangler tail --format json` が 1 リクエストごとに吐く `cpuTime` /
`wallTime` (いずれもミリ秒の整数) から取り、リクエストとイベントは `cf-ray` で
突き合わせる。tail 側には `--header` フィルタを渡すので、本番に実ユーザーの
トラフィックが流れていてもこの実行のリクエストだけが降ってくる。

    python3 .claude/skills/benchmark-gql/bench.py                 # 全ケース、既定 15 反復
    python3 .claude/skills/benchmark-gql/bench.py --repeat 30
    python3 .claude/skills/benchmark-gql/bench.py --only station,trainRoute_long
    python3 .claude/skills/benchmark-gql/bench.py --no-cpu        # tail を使わず応答時間だけ

依存は Python 3 標準ライブラリのみ。wrangler は npx 経由で呼ぶ。
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import re
import signal
import statistics
import subprocess
import sys
import time
from datetime import datetime, timezone, timedelta
from http.client import HTTPSConnection
from pathlib import Path

SKILL_DIR = Path(__file__).resolve().parent
REPO_ROOT = SKILL_DIR.parents[2]
DEFAULT_OUT_DIR = REPO_ROOT / "benchmarks"
JST = timezone(timedelta(hours=9))

# 比較対象。script は wrangler tail に渡す Worker 名 (wrangler.jsonc の name)。
TARGETS = [
    {"key": "production", "label": "本番", "origin": "https://gql.trainlcd.app", "script": "stationapi"},
    {"key": "staging", "label": "ステージング", "origin": "https://gql-stg.trainlcd.app", "script": "stationapi-stg"},
]

USER_AGENT = "stationapi-bench/1.0 (+https://github.com/TrainLCD/StationAPI)"
BENCH_HEADER = "x-stationapi-bench"

# コールドスタート (WASM 実体化 + 索引構築) の判定。実測では 250〜450 ms かかる。
# 固定閾値だと「もともと重いクエリ」を巻き込む (本番の trainRoute は定常で 850 ms 出る)
# ので、同じケース・同じ環境の中央値からどれだけ跳ねたかで見る。コールドは少数派なので
# 中央値は定常側に残り、両方の条件を満たした標本だけが外れる。
COLD_START_FACTOR = 2.5      # 中央値の何倍以上か
COLD_START_MARGIN_MS = 150   # かつ中央値から何 ms 以上離れているか


# --------------------------------------------------------------------------- クエリ定義


def wrangler_argv() -> list[str]:
    """wrangler の起動コマンド。

    AGENTS.md の方針どおり版は Makefile の WRANGLER_VERSION を唯一の出どころにする。
    ここに版を書くと「Makefile / 2 つの deploy workflow / composite action」の 4 箇所に
    5 箇所目が増えてずれるので、実行時に読み取る。
    """
    makefile = REPO_ROOT / "Makefile"
    version = None
    if makefile.exists():
        for line in makefile.read_text(encoding="utf-8").splitlines():
            if line.startswith("WRANGLER_VERSION"):
                version = line.split(":=", 1)[-1].strip()
                break
    pkg = f"wrangler@{version}" if version else "wrangler"
    return ["npx", "--yes", pkg]


_NAME = re.compile(r"[A-Za-z_]\w*")


def root_query_fields(document: str) -> set[str]:
    """オペレーション本文の深さ 1 — つまり Query の直下で選ぶフィールド名を返す。

    本文全体を正規表現で舐めると、ネストした同名フィールドまで拾ってしまう。
    たとえば `Station.lines(transportType: Rail)` が Query の `lines` を
    覆ったことになり、`lines` のケースを足し忘れても警告が出なくなる。
    深さで切れば取り違えは起きない。
    """
    fields: set[str] = set()
    depth = paren = 0
    i, n = 0, len(document)
    while i < n:
        ch = document[i]
        if document.startswith('"""', i):   # ブロック文字列。
            # 単純に " 単位で食うと、中に " が 1 つあるだけで境界がずれ、
            # 続く # や括弧が本文として解釈されてしまう。丸ごと 1 トークンで飛ばす。
            j = i + 3
            while j < n:
                if document[j] == "\\":
                    j += 2                  # \""" は終端ではない
                    continue
                if document.startswith('"""', j):
                    break
                j += 1
            i = n if j >= n else j + 3
            continue
        if ch == '"':                       # 引数の文字列。中の括弧を数えない
            i += 1
            while i < n and document[i] != '"':
                i += 2 if document[i] == "\\" else 1
            i += 1
            continue
        if ch == "#":                       # 行コメント。
            # 深さ 1 の外でも捨てる。コメントの散文に括弧が 1 つ混じるだけで
            # 深さの追跡がずれ、フィールドの集合ごと壊れるため。
            newline = document.find("\n", i + 1)
            i = n if newline == -1 else newline + 1
            continue
        if ch == "(":
            paren += 1
        elif ch == ")":
            paren = max(paren - 1, 0)
        elif paren:
            pass                            # 引数の中はフィールドではない
        elif ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth <= 0:
                break                       # オペレーション本文の終わり
        elif depth == 1:
            if ch == "@":                   # ディレクティブ名はフィールドではない
                directive = _NAME.match(document, i + 1)
                i = directive.end() if directive else i + 1
                continue
            if document.startswith("...", i):
                i += 3                      # フラグメントスプレッドは Query フィールドではない
                continue
            m = _NAME.match(document, i)
            if m:
                j = m.end()
                while j < n and document[j] in " \t\r\n":
                    j += 1
                if j < n and document[j] == ":":
                    i = j + 1               # エイリアス。実フィールド名は次のトークン
                    continue
                fields.add(m.group(0))
                i = m.end()
                continue
        i += 1
    return fields


def uncovered_query_fields(cases: list[dict]) -> list[str]:
    """schema/public.graphql の Query フィールドのうち、どのケースも叩かないものを返す。

    Query にフィールドが増えたのにケースを足し忘れると、そのクエリだけ性能が
    見えないまま貯まり続ける。絞り込み前の全ケースで判定する。
    """
    schema = REPO_ROOT / "schema" / "public.graphql"
    if not schema.exists():
        return []
    body = re.search(r"type Query \{(.*?)\n\}", schema.read_text(encoding="utf-8"), re.S)
    if not body:
        return []
    fields = [m.group(1) for m in re.finditer(r"^\s*(\w+)\s*[(:]", body.group(1), re.M)]
    covered: set[str] = set()
    for case in cases:
        covered |= root_query_fields(case.get("query", ""))
    return [f for f in fields if f not in covered]


def load_cases(path: Path, only: list[str] | None, skip_baseline: bool) -> tuple[list[dict], list[str]]:
    doc = json.loads(path.read_text(encoding="utf-8"))
    fragments = doc["fragments"]
    cases = doc["cases"]
    uncovered = uncovered_query_fields(cases)
    # 未知判定は絞り込み前の名前で行う。--skip-baseline で落ちたケースは
    # 「存在しない」のではなく「今回対象外」なので、--only に書かれても未知ではない。
    known = {c["name"] for c in cases}
    if skip_baseline:
        cases = [c for c in cases if c.get("weight") != "baseline"]
    if only:
        wanted = set(only)
        unknown = wanted - known
        if unknown:
            sys.exit(f"未知のケース: {', '.join(sorted(unknown))}")
        cases = [c for c in cases if c["name"] in wanted]
    for case in cases:
        if case.get("kind") == "http":
            continue
        body = case["query"]
        for name in case.get("uses", []):
            if name not in fragments:
                sys.exit(f"{case['name']}: 未定義のフラグメント {name}")
            body += "\n" + fragments[name]
        case["_document"] = body
    return cases, uncovered


# --------------------------------------------------------------------------- HTTP


class Client:
    """接続を使い回す最小の HTTPS クライアント。

    urllib は毎回接続を張り直すので、TLS ハンドシェイクが測定値に混ざる。
    keep-alive を保った 1 本の接続で測るほうがサーバー側の差が見えやすい。
    """

    def __init__(self, origin: str, run_id: str, timeout: float):
        assert origin.startswith("https://")
        self.host = origin[len("https://"):]
        self.run_id = run_id
        self.timeout = timeout
        self.conn: HTTPSConnection | None = None

    def _connect(self) -> HTTPSConnection:
        if self.conn is None:
            self.conn = HTTPSConnection(self.host, timeout=self.timeout)
        return self.conn

    def close(self) -> None:
        if self.conn is not None:
            self.conn.close()
            self.conn = None

    def request(self, method: str, path: str, payload: dict | None) -> dict:
        body = json.dumps(payload).encode("utf-8") if payload is not None else None
        headers = {
            "user-agent": USER_AGENT,
            "accept": "application/json",
            BENCH_HEADER: self.run_id,
        }
        if body is not None:
            headers["content-type"] = "application/json"
        for attempt in (0, 1):
            conn = self._connect()
            try:
                started = time.perf_counter()
                conn.request(method, path, body=body, headers=headers)
                resp = conn.getresponse()
                data = resp.read()
                elapsed = (time.perf_counter() - started) * 1000
            except Exception:
                self.close()
                if attempt == 0:
                    continue
                raise
            ray = resp.getheader("cf-ray") or ""
            return {
                "status": resp.status,
                "ray": ray.split("-")[0],
                "colo": ray.split("-")[1] if "-" in ray else "",
                "bytes": len(data),
                "client_ms": elapsed,
                "body": data,
            }
        raise RuntimeError("unreachable")


def graphql_errors(raw: bytes) -> list | None:
    try:
        doc = json.loads(raw)
    except json.JSONDecodeError:
        return [{"message": "レスポンスが JSON ではありません"}]
    return doc.get("errors")


# --------------------------------------------------------------------------- wrangler tail


class Tail:
    """`wrangler tail` を JSON 形式で回し、cf-ray -> {cpuTime, wallTime} の表を作る。"""

    def __init__(self, script: str, run_id: str, log_dir: Path):
        self.script = script
        self.run_id = run_id
        self.out_path = log_dir / f"tail-{script}.jsonl"
        self.err_path = log_dir / f"tail-{script}.err"
        self.proc: subprocess.Popen | None = None
        self._out = None
        self._err = None

    def start(self) -> None:
        self._out = self.out_path.open("w", encoding="utf-8")
        self._err = self.err_path.open("w", encoding="utf-8")
        self.proc = subprocess.Popen(
            wrangler_argv() + ["tail", self.script, "--format", "json",
                               "--header", f"{BENCH_HEADER}:{self.run_id}"],
            stdout=self._out, stderr=self._err, cwd=str(REPO_ROOT),
            start_new_session=True,
        )

    def stop(self) -> None:
        """何度呼んでも安全。異常終了時の後始末からも呼ばれる。"""
        if self.proc is not None and self.proc.poll() is None:
            try:
                os.killpg(os.getpgid(self.proc.pid), signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                self.proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                os.killpg(os.getpgid(self.proc.pid), signal.SIGKILL)
        for handle in (self._out, self._err):
            if handle is not None and not handle.closed:
                handle.close()

    def died(self) -> str | None:
        """tail が落ちていれば stderr を返す。"""
        if self.proc is not None and self.proc.poll() is not None:
            return self.err_path.read_text(encoding="utf-8", errors="replace").strip()
        return None

    def events(self) -> dict[str, dict]:
        """cf-ray をキーにしたイベント表。wrangler は整形済み JSON を連結して吐く。"""
        if not self.out_path.exists():
            return {}
        text = self.out_path.read_text(encoding="utf-8", errors="replace")
        decoder = json.JSONDecoder()
        found: dict[str, dict] = {}
        i = 0
        while i < len(text):
            while i < len(text) and text[i] in " \r\n\t":
                i += 1
            if i >= len(text):
                break
            try:
                obj, i = decoder.raw_decode(text, i)
            except json.JSONDecodeError:
                break  # 途中で切れた最後の 1 件
            if not isinstance(obj, dict):
                continue
            # fetch 以外のイベント (cron など) では event が null になりうる
            headers = ((obj.get("event") or {}).get("request") or {}).get("headers") or {}
            ray = (headers.get("cf-ray") or "").split("-")[0]
            if ray:
                found[ray] = {
                    "cpu_ms": obj.get("cpuTime"),
                    "wall_ms": obj.get("wallTime"),
                    "outcome": obj.get("outcome"),
                    "version_id": (obj.get("scriptVersion") or {}).get("id"),
                }
        return found


def wait_for_tail(tails: dict[str, Tail], clients: dict[str, Client], timeout: float) -> bool:
    """tail が実際にイベントを運んでくるまで待つ。

    `--format json` の wrangler は接続完了を何も出力しないので、ヘッダ付きの
    捨てリクエストを撃ち、その cf-ray が降ってくるのを接続完了の合図にする。
    ここで待たないと最初の数ケースの CPU Time だけが欠測になる。
    """
    deadline = time.time() + timeout
    pending = {key: None for key in tails}
    while time.time() < deadline:
        for key, tail in tails.items():
            err = tail.died()
            if err:
                print(f"  ! wrangler tail ({tail.script}) が終了しました:\n{err}", file=sys.stderr)
                return False
        for key in list(pending):
            if pending[key] is None:
                try:
                    pending[key] = clients[key].request("GET", "/__ping", None)["ray"]
                except Exception:
                    pending[key] = None
        time.sleep(2.0)
        ready = True
        for key, tail in tails.items():
            ray = pending.get(key)
            if not ray or ray not in tail.events():
                ready = False
                pending[key] = None
        if ready:
            return True
    return False


# --------------------------------------------------------------------------- 計測


def percentile(values: list[float], q: float) -> float:
    """線形補間つきパーセンタイル (statistics.quantiles は n<2 で落ちるため自前)。"""
    if not values:
        return float("nan")
    ordered = sorted(values)
    if len(ordered) == 1:
        return ordered[0]
    pos = (len(ordered) - 1) * q
    low = int(pos)
    high = min(low + 1, len(ordered) - 1)
    return ordered[low] + (ordered[high] - ordered[low]) * (pos - low)


def measure(cases: list[dict], repeat: int, warmup: int, timeout: float,
            run_id: str, use_cpu: bool, log_dir: Path, pause: float) -> tuple[list[dict], dict]:
    clients = {t["key"]: Client(t["origin"], run_id, timeout) for t in TARGETS}
    tails: dict[str, Tail] = {}
    meta: dict = {"cpu_time_available": False, "tail_note": None}

    if use_cpu:
        for t in TARGETS:
            tails[t["key"]] = Tail(t["script"], run_id, log_dir)
        for tail in tails.values():
            tail.start()
        print("wrangler tail の接続を待っています ...", file=sys.stderr)
        if wait_for_tail(tails, clients, timeout=90):
            meta["cpu_time_available"] = True
            print("  tail 接続完了。CPU Time を収集します。", file=sys.stderr)
        else:
            meta["tail_note"] = "wrangler tail に接続できなかったため CPU Time は欠測です。`npx wrangler whoami` で workers_tail (read) を確認してください。"
            print(f"  ! {meta['tail_note']}", file=sys.stderr)
            for tail in tails.values():
                tail.stop()
            tails = {}

    samples: list[dict] = []
    try:
        # ---- ウォームアップ。全ケースを先に一巡させてから計測に入る。
        # ケースごとに「暖めて即計測」だと、最初のほうのケースだけ最適化前の
        # アイソレートを測ることになる。実測でも、直前に叩いていなかった環境の
        # CPU Time が 1 回目の計測でだけ 4〜5 割高く出た。両環境ともすべての
        # クエリ形状を一巡させてから測れば、その非対称が消える。
        # 応答内容の妥当性もここで見る (壊れたケースは計測前に落とす)。
        print(f"ウォームアップ ({warmup} 巡) ...", file=sys.stderr)
        for _ in range(warmup):
            for case in cases:
                method = case.get("method", "POST")
                path = case.get("path", "/")
                payload = None
                if case.get("kind") != "http":
                    payload = {"query": case["_document"], "variables": case.get("variables", {})}
                for target in TARGETS:
                    try:
                        res = clients[target["key"]].request(method, path, payload)
                    except Exception as exc:
                        sys.exit(f"{case['name']} / {target['key']}: リクエスト失敗 {exc}")
                    if res["status"] != 200:
                        sys.exit(f"{case['name']} / {target['key']}: HTTP {res['status']}")
                    if payload is not None:
                        errs = graphql_errors(res["body"])
                        if errs:
                            sys.exit(f"{case['name']} / {target['key']}: GraphQL エラー "
                                     f"{json.dumps(errs, ensure_ascii=False)[:400]}")

        # ---- 本計測
        # 外側が反復、内側がケース。1 ケースを続けて 15 回叩くのではなく、
        # 全ケースを 1 巡することを 15 回繰り返す。ケースごとにまとめて叩くと、
        # そのケースの標本が実行時間のごく一部の窓に集中し、その窓でたまたま
        # 片方の環境が遅い状態にあると、そのケースだけ差が出たように見える
        # (実測で、同じクエリのステージング平均が実行間で 89ms と 52ms に割れた)。
        # 各ケースの標本を実行全体へばらすことで、そういう一過性の状態が
        # 特定のケースへ偏らなくなる。
        # 環境の順序も毎回入れ替え、回線の変動が片方に寄らないようにする。
        prepared = []
        for case in cases:
            payload = None
            if case.get("kind") != "http":
                payload = {"query": case["_document"], "variables": case.get("variables", {})}
            prepared.append((case, case.get("method", "POST"), case.get("path", "/"), payload))

        for i in range(repeat):
            for j, (case, method, path, payload) in enumerate(prepared):
                order = TARGETS if (i + j) % 2 == 0 else list(reversed(TARGETS))
                for target in order:
                    res = clients[target["key"]].request(method, path, payload)
                    samples.append({
                        "case": case["name"],
                        "target": target["key"],
                        "iteration": i,
                        "status": res["status"],
                        "ray": res["ray"],
                        "colo": res["colo"],
                        "bytes": res["bytes"],
                        "client_ms": res["client_ms"],
                    })
                    if pause:
                        time.sleep(pause)
            print(f"  反復 {i + 1:3d}/{repeat}  ({len(samples)} samples)", file=sys.stderr)
    except BaseException:
        # ウォームアップの検証失敗 (sys.exit) や Ctrl-C で wrangler tail を残さない
        for tail in tails.values():
            tail.stop()
        raise
    finally:
        for client in clients.values():
            client.close()

    if tails:
        # tail は数秒遅れて届く。最後のリクエスト分を取りこぼさないよう待つ。
        print("tail の残りを待っています ...", file=sys.stderr)
        time.sleep(12)
        events: dict[str, dict] = {}
        for key, tail in tails.items():
            tail.stop()
            for ray, ev in tail.events().items():
                ev["target"] = key
                events[ray] = ev
        matched = 0
        versions: dict[str, set] = {t["key"]: set() for t in TARGETS}
        for s in samples:
            ev = events.get(s["ray"])
            if ev and ev["target"] == s["target"]:
                s["cpu_ms"] = ev["cpu_ms"]
                s["worker_wall_ms"] = ev["wall_ms"]
                s["outcome"] = ev["outcome"]
                matched += 1
                if ev.get("version_id"):
                    versions[s["target"]].add(ev["version_id"])
        meta["tail_matched"] = matched
        meta["tail_total"] = len(samples)
        meta["versions"] = {k: sorted(v) for k, v in versions.items()}
        print(f"  CPU Time 突き合わせ: {matched}/{len(samples)}", file=sys.stderr)
        if matched == 0:
            meta["cpu_time_available"] = False
            meta["tail_note"] = "tail イベントを 1 件も突き合わせられませんでした。"

    return samples, meta


# --------------------------------------------------------------------------- 集計


def summarize(samples: list[dict], cases: list[dict]) -> list[dict]:
    by_case = {c["name"]: c for c in cases}
    rows = []
    for name in [c["name"] for c in cases]:
        row = {"case": name, "weight": by_case[name].get("weight", ""),
               "note": by_case[name].get("note", ""), "targets": {}}
        for target in TARGETS:
            sel = [s for s in samples if s["case"] == name and s["target"] == target["key"]]
            if not sel:
                continue
            client = [s["client_ms"] for s in sel]
            cpu_all = [s["cpu_ms"] for s in sel if s.get("cpu_ms") is not None]
            cutoff = cold_cutoff(cpu_all)
            cold = [v for v in cpu_all if v > cutoff]
            cpu = [v for v in cpu_all if v <= cutoff]
            wall = [s["worker_wall_ms"] for s in sel
                    if s.get("worker_wall_ms") is not None
                    and s.get("cpu_ms") is not None and s["cpu_ms"] <= cutoff]
            row["targets"][target["key"]] = {
                "n": len(sel),
                "bytes": statistics.median([s["bytes"] for s in sel]),
                "client_min": min(client),
                "client_p50": percentile(client, 0.5),
                "client_mean": statistics.fmean(client),
                "client_p95": percentile(client, 0.95),
                "cpu_n": len(cpu),
                "cpu_mean": statistics.fmean(cpu) if cpu else None,
                "cpu_p50": percentile(cpu, 0.5) if cpu else None,
                "cpu_p95": percentile(cpu, 0.95) if cpu else None,
                "cpu_sd": statistics.pstdev(cpu) if len(cpu) > 1 else None,
                "cpu_max": max(cpu) if cpu else None,
                "worker_wall_mean": statistics.fmean(wall) if wall else None,
                "cold_n": len(cold),
                "cold_max": max(cold) if cold else None,
                "cold_cutoff": cutoff if cutoff != float("inf") else None,
            }
        rows.append(row)
    return rows


def cold_cutoff(values: list[float]) -> float:
    """この値を超えた標本をコールドスタートとみなす、という境目を返す。"""
    if not values:
        return float("inf")
    med = statistics.median(values)
    return max(med * COLD_START_FACTOR, med + COLD_START_MARGIN_MS)


def ratio(prod, stg):
    """本番を基準にしたステージングの比。1 未満ならステージングが速い。"""
    if prod in (None, 0) or stg is None:
        return None
    return stg / prod


# --------------------------------------------------------------------------- 出力


def fmt(value, digits=2, unit=""):
    if value is None:
        return "—"
    if isinstance(value, float) and value != value:
        return "—"
    return f"{value:.{digits}f}{unit}"


def fmt_delta(r):
    if r is None:
        return "—"
    pct = (r - 1) * 100
    sign = "+" if pct >= 0 else ""
    return f"{sign}{pct:.1f}%"


# cpuTime はミリ秒の整数で届くので、両環境とも平均 1 ms を切る帯では
# 比を出しても丸め誤差を読んでいるだけになる。その場合は判定を保留する。
CPU_RESOLUTION_MS = 1.0

# 比だけで判定すると、1 ms が 2 ms になっただけで「+100%」になってしまう。
# cpuTime は整数なので各標本に最大 ±0.5 ms の丸めが乗る。平均の差がこの値を
# 下回るケースは、割合がいくら大きくても「同等」に倒す。
CPU_MIN_DIFF_MS = 1.5

# 応答時間から ping ぶんを引いた「サーバ分」は、回線のゆらぎと同じ桁まで小さくなると
# 比を出しても意味がない。両環境ともこの値を下回ったら判定を保留する。
CLIENT_NOISE_MS = 2.0


def verdict(r, threshold=0.10, abs_diff=None, min_diff=0.0, small_label="微差"):
    """比で判定し、その差が絶対値で小さすぎるときだけ判定を降ろす。

    絶対差を先に見ると、43 ms 対 42 ms のような「比でも絶対値でも僅差」まで
    特別扱いされてしまう。そこは素直に「同等」でよい。小さい絶対差が問題になるのは、
    1 ms 対 2 ms のように比だけが大きく見えるときだけ。
    """
    if r is None:
        return "—"
    if 1 - threshold < r < 1 + threshold:
        return "同等"
    if abs_diff is not None and abs(abs_diff) < min_diff:
        return small_label
    return "stg 速い" if r <= 1 - threshold else "stg 遅い"


def below_resolution(*means) -> bool:
    values = [m for m in means if m is not None]
    return bool(values) and max(values) < CPU_RESOLUTION_MS


def render_cpu_table(a, rows) -> None:
    """CPU Time の表と、その読み方の注記を書き出す。"""
    prod_key, stg_key = TARGETS[0]["key"], TARGETS[1]["key"]
    a("| クエリ | 重み | 本番 平均 | 本番 p95 | stg 平均 | stg p95 | 差 (stg/本番) | 判定 |")
    a("| --- | --- | ---: | ---: | ---: | ---: | ---: | --- |")
    for row in rows:
        p = row["targets"].get(prod_key, {})
        s = row["targets"].get(stg_key, {})
        r = ratio(p.get("cpu_mean"), s.get("cpu_mean"))
        if below_resolution(p.get("cpu_mean"), s.get("cpu_mean")):
            delta, call = "—", "分解能未満"
        else:
            diff = None
            if p.get("cpu_mean") is not None and s.get("cpu_mean") is not None:
                diff = s["cpu_mean"] - p["cpu_mean"]
            delta = fmt_delta(r)
            call = verdict(r, abs_diff=diff, min_diff=CPU_MIN_DIFF_MS)
        a(f"| `{row['case']}` | {row['weight']} | {fmt(p.get('cpu_mean'))} | {fmt(p.get('cpu_p95'))} "
          f"| {fmt(s.get('cpu_mean'))} | {fmt(s.get('cpu_p95'))} | {delta} | {call} |")
    a("")
    a(f"単位はミリ秒。「微差」は割合こそ大きいものの平均の差が {CPU_MIN_DIFF_MS} ms 未満で、"
      "cpuTime の整数丸め (標本あたり最大 ±0.5 ms) と区別がつかないケース。")
    # この Worker は外部 I/O を持たないので wall time は CPU Time とほぼ一致するはず。
    # 乖離が出たら「CPU を使っていない待ち時間」が混ざったということなので、そこだけ報告する。
    gaps = []
    for row in rows:
        for key, stat in row["targets"].items():
            if stat.get("cpu_mean") is not None and stat.get("worker_wall_mean") is not None:
                gaps.append((stat["worker_wall_mean"] - stat["cpu_mean"], row["case"], key))
    if gaps:
        gap, case, key = max(gaps)
        label = next(t["label"] for t in TARGETS if t["key"] == key)
        a("")
        a(f"Worker の wall time と CPU Time の差は最大 {gap:.2f} ms "
          f"(`{case}` / {label})。この Worker は外部 I/O を持たないので両者はほぼ一致し、"
          "大きく開いたときは CPU を使っていない待ちが混ざったことを意味する。")


def render_markdown(rows, samples, meta, args, run_id, started, finished) -> str:
    prod_key, stg_key = TARGETS[0]["key"], TARGETS[1]["key"]
    out = []
    a = out.append

    a(f"# GraphQL ベンチマーク {started.strftime('%Y-%m-%d %H:%M')} JST")
    a("")
    a(f"本番 `{TARGETS[0]['origin']}` とステージング `{TARGETS[1]['origin']}` の同一クエリを "
      f"交互に叩き、クライアント応答時間と Cloudflare Worker の CPU Time を比較した。")
    a("")
    a("## 実行条件")
    a("")
    a("| 項目 | 値 |")
    a("| --- | --- |")
    a(f"| 実行 ID | `{run_id}` |")
    a(f"| 開始 / 終了 | {started.strftime('%Y-%m-%d %H:%M:%S')} / {finished.strftime('%H:%M:%S')} JST |")
    a(f"| 反復数 | {args.repeat} (計測前に全ケースを {args.warmup} 巡して破棄) |")
    a(f"| ケース数 | {len(rows)} |")
    a(f"| 総リクエスト数 | {len(samples)} |")
    a(f"| CPU Time 取得 | {'wrangler tail (cf-ray 突き合わせ ' + str(meta.get('tail_matched', 0)) + '/' + str(meta.get('tail_total', 0)) + ')' if meta.get('cpu_time_available') else '欠測'} |")
    a(f"| 計測元 | {platform.node()} / Python {platform.python_version()} |")
    colos = sorted({s.get("colo") for s in samples if s.get("colo")})
    if colos:
        a(f"| コロ | {', '.join(colos)} |")
    versions = meta.get("versions") or {}
    for target in TARGETS:
        ids = versions.get(target["key"]) or []
        if ids:
            a(f"| {target['label']} Worker バージョン | {', '.join(f'`{v}`' for v in ids)} |")
    a("")
    if meta.get("uncovered_query_fields"):
        a("> [!WARNING]")
        a(f"> ベンチマークのケースが無い Query フィールドがある: "
          f"{', '.join('`' + f + '`' for f in meta['uncovered_query_fields'])}")
        a("")
    if meta.get("tail_note"):
        a(f"> [!WARNING]")
        a(f"> {meta['tail_note']}")
        a("")

    has_cpu = any(stat.get("cpu_mean") is not None
                  for row in rows for stat in row["targets"].values())

    a("## CPU Time (Cloudflare Worker)")
    a("")
    if not has_cpu:
        a("この実行では CPU Time を集めていない (`--no-cpu`、または `wrangler tail` に"
          "接続できなかった)。実装差の判定に使えるのはこの指標だけなので、"
          "下のクライアント応答時間は参考値として読むこと。")
    else:
        a("Worker が 1 リクエストの処理に使った CPU 時間。ネットワークとコロの当たり外れを含まないので、"
          "実装起因の差はここに出る。`wrangler tail` が返す値はミリ秒の整数。"
          "中央値から大きく跳ねた標本はコールドスタートとみなし、この表から外して次節に分離した。")
        a("")
        render_cpu_table(a, rows)
    a("")

    a("## クライアント応答時間")
    a("")
    a("同一の keep-alive 接続で測った往復時間。ネットワークと Cloudflare のエッジ処理、"
      "レスポンス転送を含むので、CPU Time との差がそれらの取り分になる。")
    a("")
    # __ping は「何もしない」1 往復なので、その時間を引けば回線ぶんを落とせる。
    # 本番とステージングは別ドメインで経路も別なので、環境ごとに基準線を持つ。
    # p50 だと回線のゆらぎが基準線に乗って補正後の値が潰れる (0 に張り付く) ため、
    # 基準線も対象も最小値を使う。最小値は往復時間の下限、つまり最もノイズの少ない推定。
    base = {}
    for row in rows:
        if row["case"] == "ping":
            for key, stat in row["targets"].items():
                base[key] = stat.get("client_min")

    def net_corrected(stat, key):
        v, b = stat.get("client_min"), base.get(key)
        return None if v is None or b is None else max(v - b, 0.0)

    header = "| クエリ | 応答サイズ | 本番 p50 | 本番 p95 | stg p50 | stg p95 | 差 (stg/本番) | 判定 |"
    divider = "| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |"
    if base:
        header = ("| クエリ | 応答サイズ | 本番 p50 | 本番 p95 | 本番 サーバ分 | "
                  "stg p50 | stg p95 | stg サーバ分 | 差 (サーバ分同士) | 判定 |")
        divider = "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |"
    a(header)
    a(divider)
    for row in rows:
        p = row["targets"].get(prod_key, {})
        s = row["targets"].get(stg_key, {})
        size = p.get("bytes") or s.get("bytes") or 0
        if base:
            if row["case"] == "ping":
                pc = sc = None      # 基準線そのものなので定義上ゼロ。空欄にする
            else:
                pc, sc = net_corrected(p, prod_key), net_corrected(s, stg_key)
            r = ratio(pc, sc)
            if pc is None or sc is None:
                delta, call = "—", "—"
            elif max(pc, sc) < CLIENT_NOISE_MS:
                # 引き算の結果が回線のゆらぎ以下。0 に張り付いて -100% と出るのを防ぐ
                delta, call = "—", "誤差内"
            else:
                delta = fmt_delta(r)
                call = verdict(r, abs_diff=sc - pc, min_diff=CLIENT_NOISE_MS,
                               small_label="誤差内")
            a(f"| `{row['case']}` | {size/1024:.1f} KiB "
              f"| {fmt(p.get('client_p50'), 1)} | {fmt(p.get('client_p95'), 1)} | {fmt(pc, 1)} "
              f"| {fmt(s.get('client_p50'), 1)} | {fmt(s.get('client_p95'), 1)} | {fmt(sc, 1)} "
              f"| {delta} | {call} |")
        else:
            r = ratio(p.get("client_p50"), s.get("client_p50"))
            a(f"| `{row['case']}` | {size/1024:.1f} KiB | {fmt(p.get('client_p50'), 1)} | {fmt(p.get('client_p95'), 1)} "
              f"| {fmt(s.get('client_p50'), 1)} | {fmt(s.get('client_p95'), 1)} | {fmt_delta(r)} | {verdict(r)} |")
    a("")
    a("単位はミリ秒。")
    if base:
        a("")
        a(f"`サーバ分` 列は、そのクエリの最小往復時間から同じ環境の `ping` の最小往復時間 "
          f"(本番 {fmt(base.get(prod_key), 1)} ms / ステージング {fmt(base.get(stg_key), 1)} ms) を引いた値。"
          "計測ホストからエッジまでの往復は環境ごとにほぼ一定なので、これを落とすと"
          "レスポンスの生成と転送にかかった分だけが残る。回線のゆらぎを避けるため"
          "中央値ではなく最小値どうしを引いている。"
          f"引いた結果が両環境とも {CLIENT_NOISE_MS:.0f} ms を下回るケースは、"
          "回線のゆらぎと区別がつかないので判定を「誤差内」にしている。")
    a("")

    cold_rows = [(row["case"], t, row["targets"][t].get("cold_n"),
                  row["targets"][t].get("cold_max"), row["targets"][t].get("cold_cutoff"))
                 for row in rows for t in row["targets"] if row["targets"][t].get("cold_n")]
    a("## コールドスタート")
    a("")
    if cold_rows:
        a("同じケース・同じ環境の CPU Time 中央値から大きく跳ねた標本。"
          "WASM の実体化と索引構築のコストで、定常性能とは分けて読む。"
          f"判定は「中央値の {COLD_START_FACTOR} 倍以上、かつ中央値 +{COLD_START_MARGIN_MS} ms 以上」。")
        a("")
        a("| クエリ | 対象 | 件数 | 判定境界 | 最大 CPU |")
        a("| --- | --- | ---: | ---: | ---: |")
        for case, target, n, mx, cutoff in cold_rows:
            label = next(t["label"] for t in TARGETS if t["key"] == target)
            a(f"| `{case}` | {label} | {n} | {fmt(cutoff, 0)} ms | {fmt(mx, 0)} ms |")
    else:
        a("この実行では中央値から跳ねた CPU Time の標本は出なかった。"
          "ウォームアップ後は暖まったアイソレートに当たり続けたということ。")
    a("")

    a("## ケース一覧")
    a("")
    a("| クエリ | 内容 |")
    a("| --- | --- |")
    for row in rows:
        a(f"| `{row['case']}` | {row['note']} |")
    a("")

    a("## 所見")
    a("")
    a("<!-- 実行者が記入する。差が出たクエリについて、実装のどこが効いているかを書く。 -->")
    a("")
    a("## 測り方の限界")
    a("")
    a(f"- `wrangler tail` の `cpuTime` はミリ秒の整数なので、1〜2 ms のクエリは丸めの影響が大きい。"
      f"平均は反復数ぶん細かくなるが、1 反復の値は信用しない。両環境とも平均 {CPU_RESOLUTION_MS:.0f} ms 未満の"
      "ケースは判定を「分解能未満」として保留している。")
    a("- 本番とステージングは別の Worker であり、暖まり具合もリクエストの割り当て先マシンも独立している。"
      "コールドスタートの有無で 100 倍単位の差が出るため、判定は定常標本だけで行っている。"
      "コールドかどうかは固定閾値ではなく、同じケース内の中央値からの跳ね方で決めている"
      "(もともと数百 ms 使うクエリを巻き込まないため)。")
    a("- 応答時間には計測ホストから Cloudflare エッジまでの回線が乗る。"
      "本番とステージングのどちらを先に叩くかは 1 リクエストごとに入れ替えており、"
      "回線の変動が片方へ偏らないようにしてある。")
    a("- 標本は「全ケースを 1 巡」を反復数ぶん繰り返して集めており、各ケースの標本は"
      "実行時間全体へばらしてある。それでも実行をまたぐと平均は動く"
      "(実測で、同じクエリのステージング平均が別実行で 89 ms と 52 ms に割れたことがある)。"
      "**2 倍に満たない差は、もう一度まわして同じ向きに出るまで結論にしない。**")
    a("- データは両環境で同一 (`/__health` で確認できる)。差が出たら実装差とみなしてよい。")
    return "\n".join(out) + "\n"


def update_index(index_path: Path, run_id: str, started, rows, meta, args) -> None:
    prod_key, stg_key = TARGETS[0]["key"], TARGETS[1]["key"]
    # 中央比は「測れたケース全体がどちらへ寄ったか」なので、差の小さいケースも含める。
    # 最速 / 最遅は個別のクエリを名指しするので、丸め誤差と区別がつかないケースは外す。
    measured, notable = [], []
    for row in rows:
        if row["weight"] == "baseline":
            continue
        pm = row["targets"].get(prod_key, {}).get("cpu_mean")
        sm = row["targets"].get(stg_key, {}).get("cpu_mean")
        r = ratio(pm, sm)
        if r is None or below_resolution(pm, sm):
            continue
        measured.append((row["case"], r))
        if abs(sm - pm) >= CPU_MIN_DIFF_MS:
            notable.append((row["case"], r))
    if measured:
        median = statistics.median([r for _, r in measured])
        summary = f"CPU 中央比 {fmt_delta(median)} ({len(measured)} ケース)"
        if notable:
            best = min(notable, key=lambda x: x[1])
            worst = max(notable, key=lambda x: x[1])
            summary += (f" / 最速 `{best[0]}` {fmt_delta(best[1])}"
                        f" / 最遅 `{worst[0]}` {fmt_delta(worst[1])}")
    else:
        summary = "CPU Time 欠測"
    line = (f"| [{started.strftime('%Y-%m-%d %H:%M')}](./{run_id}.md) | {len(rows)} | "
            f"{args.repeat} | {summary} |")

    header = [
        "# 実行履歴",
        "",
        "`bench.py` が 1 実行につき 1 行追記する。差は本番を基準にしたステージングの CPU Time 平均比で、"
        "マイナスならステージングのほうが CPU を使っていない。",
        "",
        "| 実行 | ケース数 | 反復 | 要約 |",
        "| --- | ---: | ---: | --- |",
    ]
    if index_path.exists():
        lines = index_path.read_text(encoding="utf-8").rstrip("\n").split("\n")
    else:
        lines = list(header)
    # 同じ実行の行があれば差し替える (--rerender で集計をやり直したとき)
    anchor = f"](./{run_id}.md)"
    for i, existing in enumerate(lines):
        if anchor in existing:
            lines[i] = line
            break
    else:
        lines.append(line)
    index_path.write_text("\n".join(lines) + "\n", encoding="utf-8")


# --------------------------------------------------------------------------- 自己診断

# root_query_fields は「ケースの足し忘れ」を知らせるためだけの補助だが、壊れても
# 黙って警告が出なくなるだけなので気付けない。過去に見つかった取りこぼしを
# ここに固定しておく。`make test` は Rust 専用なので、Python 側はこれで代える。
_LEXER_CASES = [
    ("ルートフィールド",
     'query Q { station(id: 1) { name } }', {"station"}),
    ("ルート複数",
     'query Q { station(id: 1) { name } lines(lineIds: [1]) { id } }', {"station", "lines"}),
    ("ネストは拾わない",
     'query Q { trainRoute(fromStationId: 1, toStationId: 2) { segments { station { id } } } }',
     {"trainRoute"}),
    ("ネストが引数付きでも拾わない",
     'query Q { trainRoute(fromStationId: 1, toStationId: 2) { segments { station { lines(transportType: Rail) { id } } } } }',
     {"trainRoute"}),
    ("フラグメントスプレッドは無視",
     'query Q { station(id: 1) { ...StationCore } }', {"station"}),
    ("エイリアスは実フィールド名を採る",
     'query Q { a: stationsByName(name: "x", limit: 2) { id } }', {"stationsByName"}),
    ("引数の文字列にある括弧を数えない",
     'query Q { stationsByName(name: "新宿(西口)", limit: 2) { id } lines(lineIds: [1]) { id } }',
     {"stationsByName", "lines"}),
    ("引数の文字列にある # はコメントではない",
     'query Q { stationsByName(name: "#1 番線", limit: 2) { id } }', {"stationsByName"}),
    ("コメントの語を拾わない",
     'query Q {\n  # lines\n  station(id: 1) { name }\n}', {"station"}),
    ("コメント内の閉じ波括弧で深さを崩さない",
     'query Q {\n  station(id: 1) {\n    # closing } here\n    name\n  }\n  lines(lineIds: [1]) { id }\n}',
     {"station", "lines"}),
    ("コメント内の開き波括弧で深さを崩さない",
     'query Q {\n  # open { here\n  station(id: 1) { name }\n  lines(lineIds: [1]) { id }\n}',
     {"station", "lines"}),
    ("コメント内の閉じ括弧で深さを崩さない",
     'query Q {\n  station(id: 1) { name }  # a paren ) here\n  lines(lineIds: [1]) { id }\n}',
     {"station", "lines"}),
    ("ディレクティブ名はフィールドではない",
     'query Q { station(id: 1) @lines { name } }', {"station"}),
    ("引数付きディレクティブ",
     'query Q { station(id: 1) @include(if: $x) { name } lines(lineIds: [1]) { id } }',
     {"station", "lines"}),
    ("ブロック文字列を丸ごと飛ばす",
     'query Q { field(arg: """text # { }""") other }', {"field", "other"}),
    ("ブロック文字列の中に \" があっても崩れない",
     'query Q { field(arg: """text " # { }""") other }', {"field", "other"}),
    ("ブロック文字列の中の波括弧",
     'query Q { field(arg: """text " { }""") other }', {"field", "other"}),
]


def self_test(queries_path: Path) -> int:
    """レキサとカバレッジ判定の自己診断。`--self-test` で走る。"""
    failures = 0
    for label, document, expected in _LEXER_CASES:
        got = root_query_fields(document)
        ok = got == expected
        failures += not ok
        print(f"  {'ok  ' if ok else 'FAIL'}  {label}"
              + ("" if ok else f"\n        期待 {sorted(expected)} / 実際 {sorted(got)}"),
              file=sys.stderr)

    # カタログ側。全 Query フィールドを覆えているか、覆えなくなったら気付けるか。
    # スキーマが読めないと uncovered_query_fields は無条件に空を返すので、
    # 先に存在を確かめる。これが無いと以下 2 件が空振りで ok になる。
    schema = REPO_ROOT / "schema" / "public.graphql"
    if not schema.exists():
        print(f"  FAIL  {schema} が見つからない (カバレッジ判定を検証できない)", file=sys.stderr)
        print(f"全 {len(_LEXER_CASES) + 2} 件中 {len(_LEXER_CASES) - failures} 件 ok", file=sys.stderr)
        return 1

    cases = json.loads(queries_path.read_text(encoding="utf-8"))["cases"]
    uncovered = uncovered_query_fields(cases)
    ok = not uncovered
    failures += not ok
    print(f"  {'ok  ' if ok else 'FAIL'}  {queries_path.name} が Query を全て覆う"
          + ("" if ok else f" (未カバー: {uncovered})"), file=sys.stderr)

    named = next((c["name"] for c in cases if c.get("query")), None)
    if named:
        target = next(iter(root_query_fields(
            next(c["query"] for c in cases if c["name"] == named))), None)
        remaining = [c for c in cases if c["name"] != named]
        ok = target is not None and target in uncovered_query_fields(remaining)
        failures += not ok
        print(f"  {'ok  ' if ok else 'FAIL'}  ケースを外すと未カバーとして検出する"
              f" ({named} / {target})", file=sys.stderr)

    total = len(_LEXER_CASES) + 2
    print(f"全 {total} 件中 {total - failures} 件 ok", file=sys.stderr)
    return 1 if failures else 0


# --------------------------------------------------------------------------- main


def rerender(args) -> int:
    """生データからレポートを作り直す。

    集計や表の書き方を直したときに、本番へ投げ直さずにレポートを更新できる。
    生データには全リクエストの結果が入っているので、再計測する理由は無い。
    """
    # 出力先は入力パスから逆算するので、想定の配置でなければ止める。
    # 黙って parent.parent を取ると、raw/ 以外を渡されたとき無関係な場所へ書き出す。
    if args.rerender.parent.name != "raw":
        sys.exit(f"--rerender には <出力先>/raw/<実行 ID>.json を渡してください: {args.rerender}")
    out_dir = args.rerender.parent.parent

    raw = json.loads(args.rerender.read_text(encoding="utf-8"))
    run_id = raw["run_id"]
    started = datetime.fromisoformat(raw["started_at"])
    finished = datetime.fromisoformat(raw["finished_at"])
    saved = raw.get("args", {})

    class Saved:
        repeat = saved.get("repeat")
        warmup = saved.get("warmup")
    note = saved.get("note") or ""

    cases, _ = load_cases(Path(saved.get("queries") or (SKILL_DIR / "queries.json")), None, False)
    present = {s["case"] for s in raw["samples"]}
    cases = [c for c in cases if c["name"] in present]

    rows = summarize(raw["samples"], cases)
    markdown = render_markdown(rows, raw["samples"], raw.get("meta", {}), Saved,
                               run_id, started, finished)
    if note:
        markdown = markdown.replace("## 実行条件", f"> {note}\n\n## 実行条件", 1)

    report = out_dir / f"{run_id}.md"
    previous = report.read_text(encoding="utf-8") if report.exists() else ""
    # 手で書いた「所見」は上書きしない。集計を直しても書いた考察は残す。
    marker = "## 所見\n"
    if marker in previous and marker in markdown:
        head, _, tail = previous.partition(marker)
        kept, _, _ = tail.partition("\n## ")
        before, _, after = markdown.partition(marker)
        _, _, rest = after.partition("\n## ")
        markdown = before + marker + kept + "\n## " + rest
    report.write_text(markdown, encoding="utf-8")
    update_index(report.parent / "index.md", run_id, started, rows, raw.get("meta", {}), Saved)
    print(f"作り直し: {report}", file=sys.stderr)
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--repeat", type=int, default=15, help="計測反復数 (既定 15)")
    parser.add_argument("--warmup", type=int, default=3,
                        help="計測前に全ケースを何巡するか (既定 3)。この間の結果は捨てる")
    parser.add_argument("--only", default="", help="カンマ区切りのケース名だけを実行")
    parser.add_argument("--skip-baseline", action="store_true", help="__ping / __health を除く")
    parser.add_argument("--no-cpu", action="store_true", help="wrangler tail を使わず応答時間だけ測る")
    parser.add_argument("--timeout", type=float, default=60.0, help="HTTP タイムアウト秒")
    parser.add_argument("--pause", type=float, default=0.0, help="リクエスト間のスリープ秒")
    parser.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR, help="結果の出力先")
    parser.add_argument("--queries", type=Path, default=SKILL_DIR / "queries.json")
    parser.add_argument("--dry-run", action="store_true", help="ファイルを書かずに標準出力へ出す")
    parser.add_argument("--self-test", action="store_true",
                        help="レキサとカバレッジ判定の自己診断だけ走らせる (リクエストは送らない)")
    parser.add_argument("--rerender", type=Path, default=None,
                        help="benchmarks/raw/*.json からレポートを作り直す (リクエストは送らない)")
    parser.add_argument("--note", default="", help="レポート冒頭に添える一言")
    args = parser.parse_args()

    if args.self_test:
        return self_test(args.queries)

    if args.rerender:
        return rerender(args)

    only = [s.strip() for s in args.only.split(",") if s.strip()]
    cases, uncovered = load_cases(args.queries, only, args.skip_baseline)
    if not cases:
        sys.exit("実行するケースがありません")
    if uncovered:
        print(f"警告: ケースが無い Query フィールド: {', '.join(uncovered)}\n"
              f"      {args.queries} に追加してください。",
              file=sys.stderr)

    started = datetime.now(JST)
    run_id = started.strftime("%Y%m%d-%H%M%S")
    out_dir: Path = args.out_dir
    raw_dir = out_dir / "raw"
    log_dir = out_dir / ".logs"
    if not args.dry_run:
        raw_dir.mkdir(parents=True, exist_ok=True)
    log_dir.mkdir(parents=True, exist_ok=True)

    print(f"実行 ID {run_id} / {len(cases)} ケース × {args.repeat} 反復 × {len(TARGETS)} 環境", file=sys.stderr)
    samples, meta = measure(cases, args.repeat, args.warmup, args.timeout,
                            run_id, not args.no_cpu, log_dir, args.pause)
    finished = datetime.now(JST)
    rows = summarize(samples, cases)
    if args.note:
        meta["note"] = args.note
    meta["uncovered_query_fields"] = uncovered
    markdown = render_markdown(rows, samples, meta, args, run_id, started, finished)
    if args.note:
        markdown = markdown.replace("## 実行条件", f"> {args.note}\n\n## 実行条件", 1)

    if args.dry_run:
        print(markdown)
        return 0

    report = out_dir / f"{run_id}.md"
    report.write_text(markdown, encoding="utf-8")
    (raw_dir / f"{run_id}.json").write_text(json.dumps({
        "run_id": run_id,
        "started_at": started.isoformat(),
        "finished_at": finished.isoformat(),
        "args": {k: (str(v) if isinstance(v, Path) else v) for k, v in vars(args).items()},
        "targets": TARGETS,
        "meta": meta,
        "samples": samples,
        "summary": rows,
    }, ensure_ascii=False, indent=1), encoding="utf-8")
    update_index(out_dir / "index.md", run_id, started, rows, meta, args)
    print(f"\nレポート: {report}", file=sys.stderr)
    print(f"生データ: {raw_dir / (run_id + '.json')}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
