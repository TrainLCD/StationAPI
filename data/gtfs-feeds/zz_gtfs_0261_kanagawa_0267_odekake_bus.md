# 横浜市旭区今宿地区「おでかけバス」

StationAPIの路線・停留所・運行パターンは、横浜市の[GTFSデータ](https://www.city.yokohama.lg.jp/kurashi/machizukuri-kankyo/kotsu/chiikikokyo/huyasu/odekake/torikumitiku/odekake_bus.html)（CC BY 4.0）を加工して作成しています。

- 管理番号: #1775 / #1789 の 267。
- 確認日: 2026-11-13（JST）。[取得先](https://www.city.yokohama.lg.jp/kurashi/machizukuri-kankyo/kotsu/chiikikokyo/huyasu/odekake/torikumitiku/odekake_bus.files/0050_20260226.zip)。
- 配布期間: 20260205〜20270204。版: `2025/11/04改正-見える化ver2.6e`。
- 取得ZIPのSHA-256: `5d4fbe78c06d1061e4edc4f427c0dc5e81e0c8956fa6a6ccb2849b5de0beecd0`。
- 原本: 4系統、13停留所レコード、21便、213停車レコード。
- 既存の登録フィードに同じ取得URLはなく、既存の東急・小田急・西東京フィードとは対象サービスが異なります。同名停留所の共有は路線データの重複とは区別します。
- ID用の接頭辞 `0261` は既存登録の後に配置するための値です。調査の通し番号は `0267` です。公開後にファイル名を変更しないでください。
- #1880 の都道府県判定を適用した実データ検証: 4系統、37路線別停留所、4運行パターン。生成された停留所の都道府県コードは神奈川県（14）を確認。
- ネイティブ実行した本番GraphQLスキーマで `stationsByName`、`linesByName`、`stationTrainTypes` の応答が空でなくエラーがないことを確認（2026-11-13）。全フィードを含むステージングへの配備確認は別途必要です。
