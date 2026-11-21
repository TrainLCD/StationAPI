# 横浜市泉区下和泉地区「Eバス」

StationAPIの路線・停留所・運行パターンは、天台観光の[GTFSデータ](https://www.city.yokohama.lg.jp/kurashi/machizukuri-kankyo/kotsu/chiikikokyo/huyasu/support/shimoizumi/e-bus.html)（CC BY 4.0）を加工して作成しています。

- 管理番号: #1775 / #1789 の 269。
- 確認日: 2026-11-13（JST）。[取得先](https://www.city.yokohama.lg.jp/kurashi/machizukuri-kankyo/kotsu/chiikikokyo/huyasu/support/shimoizumi/e-bus.files/0015_20260226.zip)。
- 配布期間: 20260226〜20270225。版: `2024/09/17改正-見える化ver2.6e`。
- 取得ZIPのSHA-256: `ae7412b7bf0872a700edae321986e16bd1233299249407981bfada8c50782a7a`。
- 原本: 4系統、14停留所レコード、41便、540停車レコード。
- 既存の登録フィードに同じ取得URLはなく、既存の東急・小田急・西東京フィードとは対象サービスが異なります。同名停留所の共有は路線データの重複とは区別します。
- ID用の接頭辞 `0261` は既存登録の後に配置するための値です。調査の通し番号は `0269` です。公開後にファイル名を変更しないでください。
- #1880 の都道府県判定を適用した実データ検証: 4系統、43路線別停留所、4運行パターン。生成された停留所の都道府県コードは神奈川県（14）を確認。
- ネイティブ実行した本番GraphQLスキーマで `stationsByName`、`linesByName`、`stationTrainTypes` の応答が空でなくエラーがないことを確認（2026-11-13）。全フィードを含むステージングへの配備確認は別途必要です。
