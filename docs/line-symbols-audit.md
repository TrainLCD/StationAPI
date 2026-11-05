# Issue #1654: 路線記号・駅番号の確認

対象: dev の正規 CSV と Worker の路線・駅付帯情報組み立て。稼働中 API やアプリ画面の再現確認は含まない。

## 修正

各務原線 (30016) は第1記号 KG、第2記号 IY が設定済み。新鵜沼 IY-17 のため IY は残す。名鉄岐阜は station_number3=60 が既存だが対応する第3記号がなかったため、NH と色・形状を追加し NH-60 を返す。

API は CSV の同じスロットにある路線記号・色・形状と駅番号を組み合わせる。駅IDによる例外処理は追加しない。実行時の処理・計算量・スキーマは変更しない。

## 現行実装で確認した項目

- 副都心線 28010: F / ROUND。
- 都営三田線 99303: I / ROUND。
- TX南千住・北千住と同一駅グループの常磐快速線: 1132004=JJ-04、1132005=JJ-05。
- 七隈線の博多 9990718: N-18。
- JR博多: CSVの組み合わせから 1190231=JA-00、1190301=JB-00、1190812=JC-00 を返す。路線の形状・色も維持する。現地案内の「00」とAPIの構造化された記号・番号を混同し、駅IDで記号を消す変更は行わない。福岡の乗換表示の報告については、現行APIとアプリの再現調査が必要。
- 京成: 会社13、路線23001などは KS / KEISEI。
- 中央・総武線: 11313、JB / SQUARE。京葉線11326とは別ID。

乗換路線は lines_of_groups が各駅の line_cd でマスターを引き、付帯情報の組み立ては (line_cd, station_g_cd) で駅番号を紐付ける。上記の別路線の値が混ざる処理は現行ソースでは確認できなかった。これらを修正済み・稼働環境で解消済みとは断定しない。

## 根拠

- [名鉄公式: 名鉄岐阜 NH60・各務原線](https://trainbus.meitetsu.co.jp/meitetsu-transfer/pc/diagram/TrainRailRoadSearch?nodeId=00004202)
- [JR九州公式: 駅ナンバリング路線図](https://www.jrkyushu.co.jp/railway/routemap/rootmap_num.pdf)
- [JR東日本公式: 南千住 JJ04](https://www.jreast.co.jp/estation/station/info.aspx?StationCd=1487)

## 検証

正規CSVの記号・形状と13駅の番号を組み立てる回帰テストを追加。JR博多は元の記号・形状・色が保持されることを検証する。

ローカルで cargo fmt --all -- --check と git diff --check を実行する。Rustテストは Windows の link.exe 不在でローカル実行できないためCIで確認する。

データはビルド時に埋め込まれるため、反映には Worker の再ビルド・再デプロイが必要。
