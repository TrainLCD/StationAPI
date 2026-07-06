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

## compute_speed_table.py

`stationapi/src/domain/speed_table.rs` の自動生成ブロック(路線 × 列車種別ごとの
実効最高速度の較正テーブル)を、公開 GTFS 時刻表から再計算します。
到着時間推定(`arrival_estimation.rs`)の運動学モデルを Python で再現し、
実ダイヤの所要時間を再現する実効最高速度を二分探索でフィッティングして、
一般則(路線種別の基本速度 × 種別倍率)から ±10% 以上乖離した路線だけを出力します。

### 使い方

```bash
# フィードを取得して較正結果を表示する(ファイルは書き換えない)
python3 scripts/compute_speed_table.py --validate

# speed_table.rs の自動生成ブロックを書き換える
python3 scripts/compute_speed_table.py --apply
```

認証が必要なフィード(公共交通オープンデータセンターの大半)を使うには、
[developer.odpt.org](https://developer.odpt.org/) で発行した無料のアクセストークンを
環境変数 `ODPT_ACCESS_TOKEN` に設定します。未設定の場合、該当フィードはスキップされます。
取得した GTFS は `scripts/.gtfs_cache/` にキャッシュされます(Git 管理対象外)。

### データソース・ライセンス

GTFS 時刻表は[公共交通オープンデータセンター](https://ckan.odpt.org/)から取得しています。

| フィード | 提供事業者 | ライセンス | トークン |
| ---- | ---- | ---- | ---- |
| 都営地下鉄(浅草・三田・新宿・大江戸、都電荒川線・日暮里舎人ライナー同梱) | 東京都交通局 | [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/)(要出典明示) | 不要 |
| 函館市電 | 函館市企業局交通部 | [GTFS-RUL (ODPT)](https://gtfs-jp.org/GTFS-RUL(ODPT).pdf) | 不要 |
| 京都市営地下鉄 | 京都市交通局 | [公共交通オープンデータ基本ライセンス](https://developer.odpt.org/terms)(要出典明示) | 要 |
| 横浜市営地下鉄 | 横浜市交通局 | 同上 | 要 |
| 東京メトロ | 東京地下鉄 | 同上 | 要 |
| つくばエクスプレス | 首都圏新都市鉄道 | 同上 | 要 |
| 多摩都市モノレール | 多摩都市モノレール | 同上 | 要 |
| りんかい線 | 東京臨海高速鉄道 | 同上 | 要 |

較正値(派生データ)を含むサービスを提供する場合は、リポジトリ直下 README の
「Data Sources」に記載の出典表示をアプリ側のクレジットにも反映してください。
なお、京王電鉄・相模鉄道・東武鉄道の鉄道 GTFS も公共交通オープンデータセンターに
存在しますが、「公共交通オープンデータチャレンジ限定ライセンス」
(`api-challenge.odpt.org` 配信)で本番利用できないため、意図的に対象へ含めていません。

`--apply` はその実行で較正できた路線のエントリだけを更新し、トークン未設定などで
スキップしたフィード由来の既存エントリは保持します(トークンなし実行で京都・横浜の
エントリが消えることはありません)。
