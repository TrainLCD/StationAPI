# Limon Bus（神姫観光）

StationAPIの路線・停留所・運行パターンは、神姫観光の[Limon Bus GTFS](https://gtfs-data.jp/search?target_feed=shinkikanko*limonbus)（[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/deed.ja)）を加工して作成しています。

- 管理番号: #1775 / #1802 の 535。
- 確認日: 2026-11-14（JST）。[取得先](https://api.gtfs-data.jp/v2/organizations/shinkikanko/feeds/limonbus/files/feed.zip)。認証不要。
- [配布元メタデータ](https://api.gtfs-data.jp/v2/organizations/shinkikanko/feeds/limonbus)で CC BY 4.0、配布継続中、現行ファイルを確認。
- 配布期間: 20260401〜20270331。版: `2026/04/01改正-見える化ver2.6e`。
- 取得ZIPのSHA-256: `d99a283ad71fac5a4520e7f0ef3e7291766e1e8ed4b39343d6b349dd4f1a0249`。
- 原本: 4系統、11停留所レコード、4便、24停車レコード。大阪・京都・金沢・高山間と、高山・松本・河口湖・東京間の双方向。
- 事業者は `agency.txt` の神姫観光株式会社。既存登録に同事業者・同URLはなく、北海道高速バスなど既存フィードとも対象サービスが異なる。同名駅の停留所共有は重複路線と区別する。
- ID用接頭辞 `0262` は既存登録の後に配置するための値。調査番号は `0535`。公開後にファイル名を変更しないこと。
- 大阪・京都・石川・岐阜・山梨・東京・長野の停留所は、既存の行政区域境界と座標から都道府県を判定する。のりば番号と原本の停車順を維持する。
- 原本の主要5ファイルを `preprocessor/test_data/limonbus/` に保存し、ネットワーク不要で取り込み・都道府県・のりばの回帰検証を行う。
- 実データ検証: 4系統・24路線別停留所・4運行パターンを生成。鉄道正本と当該フィードを埋め込んだネイティブの本番GraphQLスキーマで、`stationsByName`（なんば2件・のりば5）、`linesByName`（limit: 10で4件）、各`line`、`stationTrainTypes`（双方向）を確認。認証付きの全フィードを含むステージング配備は別途確認が必要。
