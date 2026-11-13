# 二宮町コミュニティバス

StationAPIの路線・停留所・運行パターンは、二宮町の[GTFSデータ](https://gtfs-data.jp/search?target_feed=ninomiyatown*Ninomiyatowncommunitybus)（CC BY 4.0）を加工して作成しています。

- 管理番号: #1775 / #1789 の 273。
- 確認日: 2026-11-13（JST）。[取得先](https://api.gtfs-data.jp/v2/organizations/ninomiyatown/feeds/Ninomiyatowncommunitybus/files/feed.zip)。
- 配布期間: 20221001〜20261231。版: `2022v1-2026年10月06日 09時59分26秒-西沢ツールver8.10`。
- 取得ZIPのSHA-256: `a83890a56a8bed20f07aa2fb9e670b140eb3538de1eda7dc0878c1844b7788a4`。
- 原本: 2系統、61停留所レコード、10便、244停車レコード。
- 既存の登録フィードに同じ取得URLはなく、既存の東急・小田急・西東京フィードとは対象サービスが異なります。同名停留所の共有は路線データの重複とは区別します。
- ID用の接頭辞 `0261` は既存登録の後に配置するための値です。調査の通し番号は `0273` です。公開後にファイル名を変更しないでください。
- ライセンス根拠: [配布元メタデータ](https://api.gtfs-data.jp/v2/organizations/ninomiyatown/feeds/Ninomiyatowncommunitybus)。
- #1880 の都道府県判定を適用した実データ検証: 2系統、65路線別停留所、6運行パターン。生成された停留所の都道府県コードは神奈川県（14）を確認。
- ネイティブ実行した本番GraphQLスキーマで `stationsByName`、`linesByName`、`stationTrainTypes` の応答が空でなくエラーがないことを確認（2026-11-13）。全フィードを含むステージングへの配備確認は別途必要です。
