pub fn normalize_for_search(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if ('ぁ'..='ん').contains(&c) {
                // ひらがな → カタカナ
                // 変換に失敗した場合は元の文字を返す
                std::char::from_u32(c as u32 + 0x60).unwrap_or(c)
            } else if ('０'..='９').contains(&c) {
                // 全角数字 → 半角数字
                std::char::from_u32(c as u32 - 0xFEE0).unwrap_or(c)
            } else if ('Ａ'..='Ｚ').contains(&c) || ('ａ'..='ｚ').contains(&c) {
                // 全角英字 → 半角英字
                std::char::from_u32(c as u32 - 0xFEE0).unwrap_or(c)
            } else {
                c
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_for_search() {
        assert_eq!(normalize_for_search("とうきょう"), "トウキョウ");
        // 混合文字列のテスト
        assert_eq!(normalize_for_search("東京TOKYO"), "東京TOKYO");
        // 空文字列のテスト
        assert_eq!(normalize_for_search(""), "");
        // ひらがな・カタカナ混合のテスト
        assert_eq!(normalize_for_search("とうキョウ"), "トウキョウ");
    }

    #[test]
    fn test_normalize_hiragana_to_katakana() {
        // 全ひらがな→カタカナ変換
        assert_eq!(normalize_for_search("しんじゅく"), "シンジュク");
        assert_eq!(normalize_for_search("おおさか"), "オオサカ");
        assert_eq!(normalize_for_search("きょうと"), "キョウト");
        // 小さいひらがな
        assert_eq!(normalize_for_search("ぁぃぅぇぉ"), "ァィゥェォ");
        assert_eq!(normalize_for_search("っゃゅょ"), "ッャュョ");
        // ひらがなの範囲の端（ぁ〜ん）
        assert_eq!(normalize_for_search("ぁ"), "ァ");
        assert_eq!(normalize_for_search("ん"), "ン");
    }

    #[test]
    fn test_normalize_fullwidth_numbers() {
        // 全角数字→半角数字変換
        assert_eq!(normalize_for_search("０１２３４５６７８９"), "0123456789");
        assert_eq!(normalize_for_search("東京１２３"), "東京123");
    }

    #[test]
    fn test_normalize_fullwidth_alphabet() {
        // 全角英字→半角英字変換
        assert_eq!(normalize_for_search("ＡＢＣＤＥ"), "ABCDE");
        assert_eq!(normalize_for_search("ａｂｃｄｅ"), "abcde");
        assert_eq!(normalize_for_search("Ｚ"), "Z");
        assert_eq!(normalize_for_search("ｚ"), "z");
    }

    #[test]
    fn test_normalize_mixed_input() {
        // 漢字+ひらがな+全角数字の混合
        assert_eq!(normalize_for_search("東京駅１番線"), "東京駅1番線");
        // ひらがな+漢字+全角英字
        assert_eq!(normalize_for_search("しんじゅく駅ＷＥＳＴ"), "シンジュク駅WEST");
        // 複合パターン
        assert_eq!(
            normalize_for_search("とうきょう１２３ＡＢＣ"),
            "トウキョウ123ABC"
        );
    }

    #[test]
    fn test_normalize_preserves_other_characters() {
        // カタカナはそのまま
        assert_eq!(normalize_for_search("トウキョウ"), "トウキョウ");
        // 漢字はそのまま
        assert_eq!(normalize_for_search("東京"), "東京");
        // 半角英数字はそのまま
        assert_eq!(normalize_for_search("Tokyo123"), "Tokyo123");
        // 記号はそのまま
        assert_eq!(normalize_for_search("東京-品川"), "東京-品川");
        assert_eq!(normalize_for_search("（テスト）"), "（テスト）");
    }

    #[test]
    fn test_normalize_station_name_search_patterns() {
        // 実際の駅名検索で使われるパターン
        assert_eq!(normalize_for_search("しながわ"), "シナガワ");
        assert_eq!(normalize_for_search("うえの"), "ウエノ");
        assert_eq!(normalize_for_search("あきはばら"), "アキハバラ");
        assert_eq!(normalize_for_search("いけぶくろ"), "イケブクロ");
        assert_eq!(normalize_for_search("しぶや"), "シブヤ");
    }
}
