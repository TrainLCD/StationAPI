# Issue #1654: 路線記号・駅番号の確認

対象: 最新 dev の正規 CSV と Worker の路線・駅付帯情報組み立て。稼働中 API やアプリ画面の再現確認は含まない。

## 修正

- 各務原線 (30016): 第1記号 KG、第2記号 IY は既存の正しい設定。新鵜沼 IY-17 のため IY は残す。名鉄岐阜は station_number3=60 が既存だが対応する第3記号がなかったため NH と色・形状を追加し、NH-60 を返す。
- JR博多: 1190231 / 1190301 / 1190812 は公式表記が記号なしの 00。get_station_numbers でこの3駅IDの第1番号のみ路線記号なしで返し、形状・色は元の路線設定を維持する（鹿児島本線: HALF_SQUARE、福北ゆたか線: HALF_SQUARE_DARK_TEXT）。路線自体の JA/JB/JC は維持し、地下鉄博多 (K-11 / N-18) は対象外。計算量は既存どおり駅ごと O(1)。

## 現行実装で確認した項目

- 副都心線 28010: F / ROUND。
- 都営三田線 99303: I / ROUND。
- TX南千住・北千住と同一駅グループの常磐快速線: 1132004=JJ-04、1132005=JJ-05。
- 七隈線の博多 9990718: N-18。
- 京成: 会社13、路線23001などは KS / KEISEI。
- 中央・総武線: 11313、JB / SQUARE。京葉線11326とは別ID。

乗換路線は lines_of_groups が各駅の line_cd でマスターを引き、付帯情報の組み立ては (line_cd, station_g_cd) で駅番号を紐付ける。上記の別路線の値が混ざる処理は現行ソースでは確認できなかった。これらを修正済み・稼働環境で解消済みとは断定しない。

## 根拠

- [名鉄公式: 名鉄岐阜 NH60・各務原線](https://trainbus.meitetsu.co.jp/meitetsu-transfer/pc/diagram/TrainRailRoadSearch?nodeId=00004202)
- [JR九州公式: 駅ナンバリング路線図](https://www.jrkyushu.co.jp/railway/routemap/rootmap_num.pdf)
- [JR東日本公式: 南千住 JJ04](https://www.jreast.co.jp/estation/station/info.aspx?StationCd=1487)

## 検証

正規CSVの13駅について、記号スロットと駅番号の対応および博多の00を照合済み。git diff --check 成功。
Rustの回帰テストを追加。作業フォルダー内にRustを用意し、cargo fmt --all と cargo fmt --all -- --check に成功。cargo test -p stationapi test_issue_1654 は Windows の link.exe 不在でビルドできなかったため、更新後のCIで確認する。初回コミットのCIではTest Suite・Clippy・Checkとデータ整合性検証は成功、Rustfmtは失敗していた。

データはビルド時に埋め込まれるため、反映には Worker の再ビルド・再デプロイが必要。

## レビュー後の補正

NUMBER_ONLY はアプリの NumberingIcon に描画分岐がなく、表示が消えるため不適切だった。路線記号なしという番号表記とアイコン形状を混同せず、元の形状と色を維持する。アプリ画面での表示確認は未実施。
