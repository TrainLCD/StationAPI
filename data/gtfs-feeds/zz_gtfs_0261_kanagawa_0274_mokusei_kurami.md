# 寒川町コミュニティバス「もくせい号」倉見大村ルート

StationAPIの路線・停留所・運行パターンは、寒川町の[GTFSデータ](https://gtfs-data.jp/search?target_feed=samukawatown*mokusei-go_kuramiomuraroute)（CC BY 4.0）を加工して作成しています。

- 管理番号: #1775 / #1789 の 274。
- 確認日: 2026-11-13（JST）。[取得先](https://api.gtfs-data.jp/v2/organizations/samukawatown/feeds/mokusei-go_kuramiomuraroute/files/feed.zip)。
- 配布期間: 20251201〜20261231。版: `2025v1-2025年12月23日 08時27分12秒-西沢ツールver8.31`。
- 取得ZIPのSHA-256: `dd145e50cb7306160600e8931583cb73222dddb5e7a414693c71ee2d7457f083`。
- 原本: 1系統、26停留所レコード、24便、552停車レコード。
- 既存の登録フィードに同じ取得URLはなく、既存の東急・小田急・西東京フィードとは対象サービスが異なります。同名停留所の共有は路線データの重複とは区別します。
- ID用の接頭辞 `0261` は既存登録の後に配置するための値です。調査の通し番号は `0274` です。公開後にファイル名を変更しないでください。
- ライセンス根拠: [配布元メタデータ](https://api.gtfs-data.jp/v2/organizations/samukawatown/feeds/mokusei-go_kuramiomuraroute)。
- #1880 の都道府県判定を適用した実データ検証: 1系統、26路線別停留所、2運行パターン。生成された停留所の都道府県コードは神奈川県（14）を確認。
- ネイティブ実行した本番GraphQLスキーマで `stationsByName`、`linesByName`、`stationTrainTypes` の応答が空でなくエラーがないことを確認（2026-11-13）。全フィードを含むステージングへの配備確認は別途必要です。
