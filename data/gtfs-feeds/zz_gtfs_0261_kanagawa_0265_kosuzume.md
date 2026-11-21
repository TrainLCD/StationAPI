# 横浜市地域交通サポート事業 戸塚区小雀地区「こすずめ号」

StationAPIの路線・停留所・運行パターンは、横浜市の[GTFSデータ](https://data.city.yokohama.lg.jp/dataset/toshi_kosuzume-bus)（CC BY 4.0）を加工して作成しています。

- 管理番号: #1775 / #1789 の 265。
- 確認日: 2026-11-13（JST）。[取得先](https://www.city.yokohama.lg.jp/kurashi/machizukuri-kankyo/kotsu/chiikikokyo/huyasu/odekake/torikumitiku/kosuzume-bus.files/0064_20260513.zip)。
- 配布期間: 20260513〜20270331。版: `2025/04/01改正-見える化ver2.6e`。
- 取得ZIPのSHA-256: `33c358e0dbecfba54a33821e8891a6775d994f61995e5db957b224eb7c0ecba6`。
- 原本: 2系統、24停留所レコード、11便、246停車レコード。
- 既存の登録フィードに同じ取得URLはなく、既存の東急・小田急・西東京フィードとは対象サービスが異なります。同名停留所の共有は路線データの重複とは区別します。
- ID用の接頭辞 `0261` は既存登録の後に配置するための値です。調査の通し番号は `0265` です。公開後にファイル名を変更しないでください。
- #1880 の都道府県判定を適用した実データ検証: 2系統、47路線別停留所、5運行パターン。生成された停留所の都道府県コードは神奈川県（14）を確認。
- ネイティブ実行した本番GraphQLスキーマで `stationsByName`、`linesByName`、`stationTrainTypes` の応答が空でなくエラーがないことを確認（2026-11-13）。全フィードを含むステージングへの配備確認は別途必要です。
