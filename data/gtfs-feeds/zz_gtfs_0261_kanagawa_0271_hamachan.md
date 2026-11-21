# 横須賀市コミュニティバス「ハマちゃんバス」

StationAPIの路線・停留所・運行パターンは、横須賀市の[GTFSデータ](https://gtfs-data.jp/search?target_feed=yokosukacity*hamachanbus)（CC BY 4.0）を加工して作成しています。

- 管理番号: #1775 / #1789 の 271。
- 確認日: 2026-11-13（JST）。[取得先](https://api.gtfs-data.jp/v2/organizations/yokosukacity/feeds/hamachanbus/files/feed.zip)。
- 配布期間: 20230601〜20280331。版: `20230601`。
- 取得ZIPのSHA-256: `697e90b21415c25181cc58b3168c2751cce1eb6245f7800ccdadac3f8e6dee72`。
- 原本: 3系統、31停留所レコード、12便、222停車レコード。
- 既存の登録フィードに同じ取得URLはなく、既存の東急・小田急・西東京フィードとは対象サービスが異なります。同名停留所の共有は路線データの重複とは区別します。
- ID用の接頭辞 `0261` は既存登録の後に配置するための値です。調査の通し番号は `0271` です。公開後にファイル名を変更しないでください。
- ライセンス根拠: [配布元メタデータ](https://api.gtfs-data.jp/v2/organizations/yokosukacity/feeds/hamachanbus)。
- #1880 の都道府県判定を適用した実データ検証: 3系統、31路線別停留所、3運行パターン。生成された停留所の都道府県コードは神奈川県（14）を確認。
- ネイティブ実行した本番GraphQLスキーマで `stationsByName`、`linesByName`、`stationTrainTypes` の応答が空でなくエラーがないことを確認（2026-11-13）。全フィードを含むステージングへの配備確認は別途必要です。
