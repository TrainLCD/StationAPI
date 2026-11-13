# 大新東(レイディアントシティ横濱)

StationAPIの路線・停留所・運行パターンは、大新東の[GTFSデータ](https://gtfs-data.jp/search?target_feed=daishinto*Radiant-City)（CC0 1.0）を加工して作成しています。

- 管理番号: #1775 / #1789 の 270。
- 確認日: 2026-11-13（JST）。[取得先](https://api.gtfs-data.jp/v2/organizations/daishinto/feeds/Radiant-City/files/feed.zip)。
- 配布期間: 20260301〜20270228。版: `2026/03/01改正-見える化ver2.6e`。
- 取得ZIPのSHA-256: `5f8c645b2b76306c40ccfe7b19d9b273d425298ada7c7699c213dd3ce6ec0ffd`。
- 原本: 3系統、9停留所レコード、456便、2794停車レコード。
- 既存の登録フィードに同じ取得URLはなく、既存の東急・小田急・西東京フィードとは対象サービスが異なります。同名停留所の共有は路線データの重複とは区別します。
- ID用の接頭辞 `0261` は既存登録の後に配置するための値です。調査の通し番号は `0270` です。公開後にファイル名を変更しないでください。
- ライセンス根拠: [配布元メタデータ](https://api.gtfs-data.jp/v2/organizations/daishinto/feeds/Radiant-City)。
- #1880 の都道府県判定を適用した実データ検証: 3系統、19路線別停留所、3運行パターン。生成された停留所の都道府県コードは神奈川県（14）を確認。
- ネイティブ実行した本番GraphQLスキーマで `stationsByName`、`linesByName`、`stationTrainTypes` の応答が空でなくエラーがないことを確認（2026-11-13）。全フィードを含むステージングへの配備確認は別途必要です。
