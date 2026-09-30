#!/usr/bin/env python3
"""実際の所要時間 (travel_times/cases.csv) に対する trainRoute の Estimated (MobileApp の
GPX の生成が使う推定) の誤差を、動いている Worker に問い合わせて Markdown で出す。

CI の回帰テスト (src/travel_times.rs) は data/*.csv だけで動くので、生成データにしか
無い種別グループを飛ばし、線路の長さも持たない。本番と同じ生成データでの精度は、
`make data && make dev` で起動した Worker か、ステージングに向けてこれで測る。
推定の規則や較正を変える PR には、変更前と変更後のこのレポートを載せる。

使い方:
    python3 scripts/travel_time_report.py                 # http://127.0.0.1:8787/
    python3 scripts/travel_time_report.py --api <URL>
"""
from __future__ import annotations

import argparse
import csv
import json
import statistics
import sys
import urllib.request
from pathlib import Path

CASES = Path(__file__).resolve().parent.parent / "travel_times" / "cases.csv"
# 既定の Python-urllib は配信側で弾かれるので、bench.py と同じく名乗る
USER_AGENT = "stationapi-travel-time-report/1.0 (+https://github.com/TrainLCD/StationAPI)"
QUERY = """query TravelTimeReport($from: Int!, $to: Int!, $group: Int!) {
  trainRoute(fromStationId: $from, toStationId: $to, model: Estimated,
    legs: [{ lineGroupId: $group, fromStationId: $from, toStationId: $to }]) {
    segments { station { id } arrivalCumulativeMinutes departureCumulativeMinutes }
  }
}"""


def estimate(api: str, case: dict) -> float:
    end = int(case["slice_end_station_id"] or case["to_station_id"])
    body = json.dumps({
        "query": QUERY,
        "variables": {
            "from": int(case["from_station_id"]),
            "to": end,
            "group": int(case["line_group_id"]),
        },
    }).encode()
    req = urllib.request.Request(
        api, data=body, headers={"content-type": "application/json", "user-agent": USER_AGENT}
    )
    with urllib.request.urlopen(req, timeout=30) as res:
        data = json.load(res)
    if data.get("errors"):
        raise RuntimeError("; ".join(e.get("message", "") for e in data["errors"]))
    segments = data["data"]["trainRoute"]["segments"]
    target = int(case["to_station_id"])
    segment = next(s for s in segments[1:] if s["station"]["id"] == target)
    key = (
        "arrivalCumulativeMinutes"
        if case["measure"] == "arrival"
        else "departureCumulativeMinutes"
    )
    return float(segment[key])


def range_error(est: float, lo: float, hi: float) -> float:
    if est < lo:
        return (lo - est) / lo
    if est > hi:
        return (est - hi) / hi
    return 0.0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--api", default="http://127.0.0.1:8787/")
    args = parser.parse_args()

    with CASES.open(encoding="utf-8") as f:
        cases = list(csv.DictReader(f))

    rows, range_errors, typical_errors, failed = [], [], [], []
    for case in cases:
        lo, hi = float(case["real_min_minutes"]), float(case["real_max_minutes"])
        typical = float(case["real_typical_minutes"])
        try:
            est = estimate(args.api, case)
        except Exception as e:  # noqa: BLE001 - 1 件の失敗で全体を止めない
            failed.append(f"{case['label']}: {e}")
            continue
        t, r = (est - typical) / typical, range_error(est, lo, hi)
        typical_errors.append(abs(t))
        range_errors.append(r)
        rows.append(
            f"| {case['label']} | {typical:g}分 ({lo:g}〜{hi:g}分) | {est:.1f}分 "
            f"| {t * 100:+.1f}% | {r * 100:.1f}% |"
        )

    print(f"# 到着時間推定の誤差 ({args.api})\n")
    print("| 基準 | 実際の典型 (範囲) | 推定 | 典型からのずれ | 範囲からの外れ |")
    print("| --- | --- | --- | --- | --- |")
    print("\n".join(rows))
    if typical_errors:
        print(
            f"\n{len(typical_errors)} 件: 典型からのずれ (絶対値) の平均 "
            f"{statistics.mean(typical_errors) * 100:.2f}%、"
            f"範囲からの外れの平均 {statistics.mean(range_errors) * 100:.2f}%"
        )
    if failed:
        print("\n推定できなかった基準:\n")
        print("\n".join(f"- {line}" for line in failed))
    return 0 if typical_errors else 1


if __name__ == "__main__":
    sys.exit(main())
