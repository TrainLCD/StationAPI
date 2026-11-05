"""Usage: python3 audit_osaka_metro.py REPO STATION_GEOJSON [BEFORE_CSV]."""
import csv
import json
import math
import sys
from pathlib import Path

REPO = Path(sys.argv[1])
GEOJSON = Path(sys.argv[2])
BEFORE = Path(sys.argv[3]) if len(sys.argv) > 3 else None

def read_csv(path):
    with path.open(encoding="utf-8-sig", newline="") as stream:
        return list(csv.DictReader(stream))

names = dict(zip(
    ("99618", "99619", "99620", "99621", "99622", "99623", "99624", "99625", "99652"),
    ("御堂筋線", "谷町線", "四つ橋線", "中央線", "千日前線", "堺筋線", "長堀鶴見緑地線", "南港ポートタウン線", "今里筋線"),
))
aliases = {"なんば": "難波", "なかもず": "中百舌鳥", "あびこ": "我孫子"}
lines = {r["line_cd"] for r in read_csv(REPO / "data/2!lines.csv") if r["company_cd"] == "249"}
assert lines == set(names), "Unexpected operator lines; review mapping"
all_rows = read_csv(REPO / "data/3!stations.csv")
rows = [r for r in all_rows if r["line_cd"] in lines]
features = [f for f in json.loads(GEOJSON.read_text(encoding="utf-8"))["features"]
            if f["properties"]["N02_004"] == "大阪市高速電気軌道"]

def xy(point, origin):
    return ((point[0] - origin[0]) * 111195 * math.cos(math.radians(origin[1])),
            (point[1] - origin[1]) * 111195)

def distance(point, coordinates):
    distances = []
    for aa, bb in zip(coordinates, coordinates[1:]):
        a, b = xy(aa, point), xy(bb, point)
        dx, dy = b[0] - a[0], b[1] - a[1]
        denom = dx * dx + dy * dy
        t = max(0, min(1, -(a[0] * dx + a[1] * dy) / denom)) if denom else 0
        distances.append(math.hypot(a[0] + t * dx, a[1] + t * dy))
    return min(distances)

assert len(rows) == 134
assert len({r["station_cd"] for r in rows}) == 134
findings = []
for row in rows:
    lon, lat = float(row["lon"]), float(row["lat"])
    assert math.isfinite(lon) and math.isfinite(lat)
    assert 135 < lon < 136 and 34 < lat < 35
    candidates = [f for f in features
                  if names[row["line_cd"]] in f["properties"]["N02_003"]
                  and aliases.get(row["station_name"], row["station_name"]) == f["properties"]["N02_005"]]
    assert len(candidates) == 1, row["station_cd"]
    d = distance((lon, lat), candidates[0]["geometry"]["coordinates"])
    if d > 50:
        findings.append((row["station_cd"], row["station_name"], round(d, 3)))

if BEFORE:
    old_rows = read_csv(BEFORE)
    assert len(old_rows) == len(all_rows)
    changed = []
    for old, new in zip(old_rows, all_rows):
        assert old.keys() == new.keys()
        if old != new:
            assert old["station_cd"] == new["station_cd"]
            assert new["line_cd"] in lines
            assert all(old[k] == new[k] for k in old if k not in ("lon", "lat"))
            changed.append(new["station_cd"])
    assert len(changed) == 18, changed
    print("CSV change scope: 18 rows; only lon/lat changed; row order preserved")

print(json.dumps({"checked": len(rows), "over_50m": findings}, ensure_ascii=False, indent=2))
assert not findings, "Coordinates remain more than 50m from their line-specific N02 station shapes"
