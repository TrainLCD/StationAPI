# 茅ヶ崎市コミュニティバス「えぼし号」

StationAPIの路線・停留所・運行パターンは、茅ヶ崎市の[GTFSデータ](https://gtfs-data.jp/search?target_feed=chigasakicity*eboshigou)（CC BY 4.0）を加工して作成しています。

- 管理番号: #1775 / #1789 の 272。
- 確認日: 2026-11-13（JST）。[取得先](https://api.gtfs-data.jp/v2/organizations/chigasakicity/feeds/eboshigou/files/feed.zip)。
- 配布期間: 20260502〜20270331。版: `-2026年04月27日 12時54分18秒-西沢ツールver8.31`。
- 取得ZIPのSHA-256: `a2f742d9d6e509f90ff21dd168d21bcf20c6a94fd08959385a489126ac5ff5d8`。
- 原本: 6系統、222停留所レコード、61便、2363停車レコード。
- 既存の登録フィードに同じ取得URLはなく、既存の東急・小田急・西東京フィードとは対象サービスが異なります。同名停留所の共有は路線データの重複とは区別します。
- ID用の接頭辞 `0261` は既存登録の後に配置するための値です。調査の通し番号は `0272` です。公開後にファイル名を変更しないでください。
- ライセンス根拠: [配布元メタデータ](https://api.gtfs-data.jp/v2/organizations/chigasakicity/feeds/eboshigou)。
- #1880 の都道府県判定を適用した実データ検証: 6系統、244路線別停留所、8運行パターン。生成された停留所の都道府県コードは神奈川県（14）を確認。
- ネイティブ実行した本番GraphQLスキーマで `stationsByName`、`linesByName`、`stationTrainTypes` の応答が空でなくエラーがないことを確認（2026-11-13）。全フィードを含むステージングへの配備確認は別途必要です。
