pub fn normalize_for_search(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if ('ぁ'..='ん').contains(&c) {
                // ひらがな → カタカナ
                std::char::from_u32(c as u32 + 0x60).unwrap_or(c)
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
