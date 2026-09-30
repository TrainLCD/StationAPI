# travel_times/

`trainRoute` の `Estimated` (MobileApp の GPX の生成が使う到着時間推定) の所要時間を、
実際の列車の所要時間と比べるための基準を置く場所です。`estimateArrivalTimes` と
`connectedRoutes` は元の較正のままなので、ここでは測りません。速度の較正テーブルや一般則は、
1 つの路線に合わせて変えると、同じ規則を使うほかの路線の推定も変わります。
変更の前後で全体の誤差を測り、局所的な合わせ込みで全体が崩れないようにします。

## ファイル

| パス | 内容 |
| --- | --- |
| `cases.csv` | 基準の一覧。1 行が 1 区間 |
| `baseline.csv` | 本番と同じ生成データ (`make data` で作る `generated/`) で出した推定の所要時間の記録 |

`cases.csv` の列は次のとおりです。

| 列 | 内容 |
| --- | --- |
| `label` | 基準の名前。一覧の中で重複させない |
| `line_group_id` | 推定に使う種別グループ (系統) |
| `from_station_id` / `to_station_id` | 区間の出発駅と到着駅 |
| `slice_end_station_id` | 推定する区間の終わり。`measure` が `departure` のときに、到着駅より先の駅を書く。空なら到着駅 |
| `measure` | `arrival` は出発駅の発車から到着駅の到着まで。`departure` は到着駅の発車までで、到着時刻を載せない時刻表の値に使う |
| `real_min_minutes` / `real_max_minutes` | 実際の所要時間の範囲 (分) |
| `real_typical_minutes` | 実際の典型的な所要時間 (分)。回帰の見張りはこの値からのずれで判定する |
| `source` | 値の出どころ |

## 基準の決め方

- 平日の日中に出発駅を出る列車の所要時間を使います。途中で待ち合わせる列車などで
  所要時間に幅があるときは、最小と最大をそのまま書きます。
- 典型的な所要時間には、同じ列車の所要時間の中央値を書きます。中央値が分からず
  範囲だけが分かっているときは範囲の中央を書き、`source` にそう書き添えます。
  範囲は外れ値の列車 1 本で広がるので、範囲に入っているかどうかだけでは、推定が
  典型的な値から離れたことを見逃します。
- 列車は、`line_group_id` の種別グループと同じ停車パターンのものに限ります。
  停車駅が違う列車の所要時間と比べると、推定の誤差ではない差が混ざります。
- 出どころは、公開 GTFS か、メンテナが確認した値に限ります。GTFS から求めた値を
  足すときは、そのフィードの出典をリポジトリ直下の README の「Data Sources」に
  書きます。

## 測る

### CI (回帰の見張り)

`cargo test -p stationapi-worker` の `travel_times` モジュール (`src/travel_times.rs`)
が、基準ごとに推定を出し、典型的な所要時間からのずれ (絶対値の割合) を求めます。
`baseline.csv` の記録と比べて、1 件でもずれが 1 ポイントより多く増えるか、平均が
悪くなると失敗します。範囲からの外れ (範囲内なら 0、外れたら近い端に対する割合)
も表に出しますが、判定には使いません。記録にある基準を
推定できなくなったとき (種別グループがデータから消えたときなど) も失敗します。

比べるのは、本番と同じ生成データ (`generated/`) で動くときだけです。`Estimated` は、
生成データにしか無い線路の長さや種別グループを使うので、`data/*.csv` だけでは本番の
推定を再現できません。CI では `build_worker.yml` が `generated/` を作ってから走らせ
ます。`data/*.csv` で動くとき (`ci.yml` のテストなど) は、表を出すだけにします。

推定を意図して変えたときは、`make data` のあとに記録を更新して、同じ PR に含めます。
`generated/` が無いと更新は止まります。

```bash
make data
TRAVEL_TIMES_UPDATE_BASELINE=1 cargo test -p stationapi-worker travel_times
```

### レポート (本番と同じデータでの精度)

動いている Worker に `trainRoute` (`model: Estimated`) を問い合わせ、全件の誤差を Markdown で
出します。`make data && make dev` で起動した Worker (既定) か、ステージングに
向けます。

```bash
make travel-time-report
make travel-time-report TRAVEL_TIME_API=https://gql-stg.trainlcd.app/
```

推定の規則や較正を変える PR には、変更前と変更後のレポートを載せます。
