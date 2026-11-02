# 南条・大里の停車扱い（2026-11-02確認）

2026年3月14日改正の現行ダイヤに照合した修正。

- ハピラインふくい快速（type102 / group964）の南条1141516は、1021Mが通過し1023M・1022Mが停車するため、全停車0から一部通過2へ変更する。根拠: [公式全線時刻表（上下）](https://www.hapi-line.co.jp/timetable/)。
- 名鉄名古屋本線急行（type345 / group280）の大里3000145は、下り521列車が平日6:08・土休日6:10に特別停車するため、全通過1から一部停車5へ変更する。根拠: [平日表1ページ](https://www.meitetsu.co.jp/train/timetable/__icsFiles/afieldfile/2026/06/22/kai_20260314_NH_W1.pdf)、[土休日表1ページ](https://www.meitetsu.co.jp/train/timetable/__icsFiles/afieldfile/2026/02/25/20260314_NH_H1_1.pdf)。

共有種別の定義・方向・別系統657/1069・alias・生成各停は変更しない。
これは便別時刻表や運行日を実装する変更ではない。一部停車/通過フラグは個々の便の停車保証ではない。
回帰テスト: `cargo test -p data_validator --test nanjo_osato_timetable`。
