# 横浜市南区三春台・清水ケ丘地区「おおたループバス」

StationAPIの路線・停留所・運行パターンは、横浜市の[GTFSデータ](https://www.city.yokohama.lg.jp/kurashi/machizukuri-kankyo/kotsu/chiikikokyo/huyasu/odekake/torikumitiku/ootaloopbus.html)（CC BY 4.0）を加工して作成しています。

- 管理番号: #1775 / #1789 の 263。
- 確認日: 2026-11-13（JST）。[取得先](https://www.city.yokohama.lg.jp/kurashi/machizukuri-kankyo/kotsu/chiikikokyo/huyasu/odekake/torikumitiku/ootaloopbus.files/ootaloop.zip)。
- 配布期間: 20260603〜20270602。版: `2026/04/01改正-見える化ver2.6e`。
- 取得ZIPのSHA-256: `ece88efcf721bc8c344c93f32749c3e9b4f10e5a77ef04085522d8b0bc1bce17`。
- 原本: 3系統、17停留所レコード、21便、203停車レコード。
- 既存の登録フィードに同じ取得URLはなく、既存の東急・小田急・西東京フィードとは対象サービスが異なります。同名停留所の共有は路線データの重複とは区別します。
- ID用の接頭辞 `0261` は既存登録の後に配置するための値です。調査の通し番号は `0263` です。公開後にファイル名を変更しないでください。
- #1880 の都道府県判定を適用した実データ検証: 3系統、27路線別停留所、3運行パターン。生成された停留所の都道府県コードは神奈川県（14）を確認。
- ネイティブ実行した本番GraphQLスキーマで `stationsByName`、`linesByName`、`stationTrainTypes` の応答が空でなくエラーがないことを確認（2026-11-13）。全フィードを含むステージングへの配備確認は別途必要です。
