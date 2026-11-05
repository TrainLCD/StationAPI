# 名古屋鉄道の駅座標監査

確認日: 2026-11-06

## 対象と方法

`company_cd=20` の路線に属する現役駅（`e_status=0`）302行、駅名で276駅を確認した。
廃止駅8行は現役駅の位置と比較できないため対象外とした。

[国土数値情報・鉄道データ N02-25](https://nlftp.mlit.go.jp/ksj/gml/datalist/KsjTmplt-N02-2025.html)
の `N02-25_Station.geojson` から運営会社が名古屋鉄道の駅形状を抽出し、
同名駅の線分への最短距離を計算した。距離は緯度に応じて経度方向を補正した平面近似。
全302行で同名の駅形状が見つかり、293行は50m以内、9行（7駅）が50m超だった。
50mは候補抽出の基準であり、それ以内の座標の正確性を保証するものではない。

候補は名鉄公式駅ページの埋込地図の座標とも照合した。N02と公式地図には不一致があり、
ホーム中心・駅入口などの代表点の違いもあるため、N02の中心への一括置換は行わない。
修正した駅では公式地図の座標を採用し、CSVの経度・緯度だけを変更した。

## 修正

座標は（緯度, 経度）。

| 駅 | station_cd | 旧座標 | 修正座標 | 根拠 |
| --- | --- | --- | --- | --- |
| 柳津 | 3002003 | 35.364925, 136.737103 | 35.365125, 136.739523 | [公式駅地図](https://www.meitetsu.co.jp/train/station_info/line11/station/2712.html)、[230m移設の公式資料](https://www.meitetsu.co.jp/ir/reference/investment_plan/__icsFiles/afieldfile/2021/06/10/release080327_1.pdf) |
| 西ノ口 | 3000822 | 34.926759, 136.826630 | 34.924843, 136.826844 | [公式駅地図](https://www.meitetsu.co.jp/train/station_info/line07/station/2139.html) |
| 鳴海 | 3000127 | 35.079149, 136.950096 | 35.078482, 136.950704 | [公式駅地図](https://www.meitetsu.co.jp/train/station_info/line01/station/1148.html) |

## 据え置いた候補

| 駅 | N02駅形状までの距離 | 判断・一次資料 |
| --- | --- | --- |
| 名鉄名古屋（3行） | 約105m | [公式地図](https://www.meitetsu.co.jp/train/station_info/line01/station/1201.html) もN02より南にある。代表点の違いを排除できず、誤りと断定しない。 |
| 小牧原 | 約117m | [公式地図](https://www.meitetsu.co.jp/train/station_info/line14/station/3531.html) の35.300121,136.935857と既存値は約3m差。 |
| 猿投 | 約64m | [公式地図](https://www.meitetsu.co.jp/train/station_info/line05/station/1655.html) の35.122616,137.178739と既存値がほぼ一致。 |
| 町方 | 約57m | [公式地図](https://www.meitetsu.co.jp/train/station_info/line10/station/2617.html) の35.18902442412662,136.72763242049018と既存値は約9m差。 |

## 検証

- WSL Ubuntuで `cargo run -p data_validator` を実行し、`[VALID] No errors reported.` を確認した。
- CSVを変更前と列単位で比較し、差分が対象3行の `lon` / `lat` のみに限られ、行数・駅ID・行順・他列が不変であることを確認した。`git diff --check` も成功。
- データはビルド時に埋め込まれるため、公開にはデータ再生成・Worker再ビルド・再デプロイが必要。

## 関連する未修正項目

柳津の郵便番号・住所は `501-6111 / 岐阜県岐阜市柳津町宮東` のままで、
公式の `501-6105 / 岐阜市柳津町梅松一丁目1番地2` と異なる。
今回の変更対象は座標に限定したため、住所情報は別途修正が必要。
