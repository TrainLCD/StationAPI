# 寒川町コミュニティバス「もくせい号」南ルート

StationAPIの路線・停留所・運行パターンは、寒川町の[GTFSデータ](https://gtfs-data.jp/search?target_feed=samukawatown*mokusei-go_southroute)（CC BY 4.0）を加工して作成しています。

- 管理番号: #1775 / #1789 の 275。
- 確認日: 2026-11-13（JST）。[取得先](https://api.gtfs-data.jp/v2/organizations/samukawatown/feeds/mokusei-go_southroute/files/feed.zip)。
- 配布期間: 20251201〜20261231。版: `2025v1-2025年12月23日 09時19分53秒-西沢ツールver8.31`。
- 取得ZIPのSHA-256: `a929ea903ca86db1b2e8c54a5a9493acf94eff044672eff8e59537e7662ee1da`。
- 原本: 1系統、15停留所レコード、4便、64停車レコード。
- 既存の登録フィードに同じ取得URLはなく、既存の東急・小田急・西東京フィードとは対象サービスが異なります。同名停留所の共有は路線データの重複とは区別します。
- ID用の接頭辞 `0261` は既存登録の後に配置するための値です。調査の通し番号は `0275` です。公開後にファイル名を変更しないでください。
- ライセンス根拠: [配布元メタデータ](https://api.gtfs-data.jp/v2/organizations/samukawatown/feeds/mokusei-go_southroute)。
- #1880 の都道府県判定を適用した実データ検証: 1系統、15路線別停留所、1運行パターン。生成された停留所の都道府県コードは神奈川県（14）を確認。
- ネイティブ実行した本番GraphQLスキーマで `stationsByName`、`linesByName`、`stationTrainTypes` の応答が空でなくエラーがないことを確認（2026-11-13）。全フィードを含むステージングへの配備確認は別途必要です。
