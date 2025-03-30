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
}
