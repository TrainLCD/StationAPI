# scripts/

データメンテナンス用のスクリプト置き場です。CI で実行されるスクリプト
（`.github/scripts/`）とは別物で、メンテナが手元で実行してデータを更新するために使います。

## compute_average_distance.py

`data/2!lines.csv` の `average_distance`（路線ごとの平均駅間距離・メートル）を、
OpenStreetMap の鉄道ルート関係(route relation)から得た**線路に沿った実距離**で再計算します。

従来の `average_distance` は「隣接駅の直線距離の平均 × 路線種別ごとの固定係数(1.05〜1.25)」
という推定値でした。本スクリプトは当該路線の線路ジオメトリだけを取り出し、隣接駅間を
線路に沿って経路探索することで、より実態に近い平均駅間距離を求めます。

### 仕組み

1. `data/3!stations.csv` から対象路線の稼働駅(`e_status=0`)を `e_sort` 順に取得。
2. Overpass API で駅群のbbox内の鉄道ルート関係を検索し、**全駅が線路から200m以内に収まる**
   関係を路線名一致＋カバレッジで1つ選ぶ（無関係な路線への誤マッチを防止）。
3. 採用した関係の構成 way だけで線路グラフを構築。
4. 各駅を半径70m以内の全グラフノードに対応付け（複線の上下線を拾うため）、隣接駅ペアごとに
   多始点・多終点ダイクストラで線路沿い距離を算出し、平均する。
5. 経路が取れない／妥当範囲外の区間は直線距離で代替。確信できる関係が無い路線は
   旧来式（直線×係数）でフォールバックする。

### 使い方

```bash
# 既知路線で較正・精度確認（CSVは書き換えない）
python3 scripts/compute_average_distance.py --validate

# 任意の line_cd を個別計算（CSVは書き換えない）
python3 scripts/compute_average_distance.py --lines 11302,1002

# 全路線を計算して data/2!lines.csv を書き換える
python3 scripts/compute_average_distance.py --apply
```

依存は Python 3 標準ライブラリのみ。取得した OSM データは `scripts/.osm_cache/`
にキャッシュされ、再実行が高速になります（このディレクトリは Git 管理対象外）。

### データソース・ライセンス

地理データは OpenStreetMap (© OpenStreetMap contributors) を Overpass API 経由で取得しています。
OSM データは [Open Database License (ODbL)](https://www.openstreetmap.org/copyright) の下で
提供されています。算出した距離値を再配布する際は出典表示にご留意ください。
