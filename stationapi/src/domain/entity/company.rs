use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Company {
    pub company_cd: i32,
    pub rr_cd: i32,
    pub company_name: String,
    pub company_name_k: String,
    pub company_name_h: String,
    pub company_name_r: String,
    pub company_name_en: String,
    pub company_name_full_en: String,
    pub company_url: Option<String>,
    pub company_type: i32,
    pub e_status: i32,
    pub e_sort: i32,
}

impl Company {
    #![allow(clippy::too_many_arguments)]
    pub fn new(
        company_cd: i32,
        rr_cd: i32,
        company_name: String,
        company_name_k: String,
        company_name_h: String,
        company_name_r: String,
        company_name_en: String,
        company_name_full_en: String,
        company_url: Option<String>,
        company_type: i32,
        e_status: i32,
        e_sort: i32,
    ) -> Self {
        Self {
            company_cd,
            rr_cd,
            company_name,
            company_name_k,
            company_name_h,
            company_name_r,
            company_name_en,
            company_name_full_en,
            company_url,
            company_type,
            e_status,
            e_sort,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_company() -> Company {
        Company::new(
            1001,
            2001,
            "東日本旅客鉄道".to_string(),
            "ヒガシニホンリョカクテツドウ".to_string(),
            "ひがしにほんりょかくてつどう".to_string(),
            "Higashi-Nihon Ryokaku Tetsudou".to_string(),
            "JR East".to_string(),
            "East Japan Railway Company".to_string(),
            Some("https://www.jreast.co.jp".to_string()),
            1,
            1,
            1000,
        )
    }

    fn create_test_company_no_url() -> Company {
        Company::new(
            2001,
            3001,
            "東京都交通局".to_string(),
            "トウキョウトコウツウキョク".to_string(),
            "とうきょうとこうつうきょく".to_string(),
            "Toukyouto Koutsuu Kyoku".to_string(),
            "Tokyo Metropolitan Bureau of Transportation".to_string(),
            "Tokyo Metropolitan Bureau of Transportation".to_string(),
            None,
            2,
            1,
            2000,
        )
    }

    #[test]
    fn test_company_new() {
        let company = create_test_company();

        assert_eq!(company.company_cd, 1001);
        assert_eq!(company.rr_cd, 2001);
        assert_eq!(company.company_name, "東日本旅客鉄道");
        assert_eq!(company.company_name_k, "ヒガシニホンリョカクテツドウ");
        assert_eq!(company.company_name_h, "ひがしにほんりょかくてつどう");
        assert_eq!(company.company_name_r, "Higashi-Nihon Ryokaku Tetsudou");
        assert_eq!(company.company_name_en, "JR East");
        assert_eq!(company.company_name_full_en, "East Japan Railway Company");
        assert_eq!(
            company.company_url,
            Some("https://www.jreast.co.jp".to_string())
        );
        assert_eq!(company.company_type, 1);
        assert_eq!(company.e_status, 1);
        assert_eq!(company.e_sort, 1000);
    }

    #[test]
    fn test_company_new_with_none_url() {
        let company = create_test_company_no_url();

        assert_eq!(company.company_cd, 2001);
        assert_eq!(company.rr_cd, 3001);
        assert_eq!(company.company_name, "東京都交通局");
        assert_eq!(company.company_url, None);
    }

    #[test]
    fn test_company_partial_eq() {
        let company1 = create_test_company();
        let company2 = create_test_company();
        let company3 = create_test_company_no_url();

        assert_eq!(company1, company2);
        assert_ne!(company1, company3);
    }

    #[test]
    fn test_company_clone() {
        let company1 = create_test_company();
        let company2 = company1.clone();

        assert_eq!(company1, company2);
        // クローン後も独立したオブジェクトであることを確認
        assert_eq!(company1.company_cd, company2.company_cd);
        assert_eq!(company1.company_name, company2.company_name);
    }

    #[test]
    fn test_company_debug() {
        let company = create_test_company();
        let debug_string = format!("{company:?}");

        assert!(debug_string.contains("Company"));
        assert!(debug_string.contains("company_cd: 1001"));
        assert!(debug_string.contains("東日本旅客鉄道"));
    }

    #[test]
    fn test_company_serialization() {
        let company = create_test_company();

        // JSONシリアライゼーション
        let json = serde_json::to_string(&company).expect("シリアライゼーションに失敗しました");
        assert!(json.contains("1001"));
        assert!(json.contains("東日本旅客鉄道"));
        assert!(json.contains("JR East"));

        // JSONデシリアライゼーション
        let deserialized: Company =
            serde_json::from_str(&json).expect("デシリアライゼーションに失敗しました");
        assert_eq!(company, deserialized);
    }

    #[test]
    fn test_company_serialization_with_none_url() {
        let company = create_test_company_no_url();

        // URLがNoneの場合のシリアライゼーション
        let json = serde_json::to_string(&company).expect("シリアライゼーションに失敗しました");
        assert!(json.contains("\"company_url\":null"));

        // デシリアライゼーション
        let deserialized: Company =
            serde_json::from_str(&json).expect("デシリアライゼーションに失敗しました");
        assert_eq!(company, deserialized);
        assert_eq!(deserialized.company_url, None);
    }

    #[test]
    fn test_company_field_types() {
        let company = create_test_company();

        // 各フィールドの型が期待されるものであることを確認
        let _: i32 = company.company_cd;
        let _: i32 = company.rr_cd;
        let _: String = company.company_name;
        let _: String = company.company_name_k;
        let _: String = company.company_name_h;
        let _: String = company.company_name_r;
        let _: String = company.company_name_en;
        let _: String = company.company_name_full_en;
        let _: Option<String> = company.company_url;
        let _: i32 = company.company_type;
        let _: i32 = company.e_status;
        let _: i32 = company.e_sort;
    }

    #[test]
    fn test_company_edge_cases() {
        // エッジケース：空文字列
        let company_empty_strings = Company::new(
            0,
            0,
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            Some("".to_string()),
            0,
            0,
            0,
        );

        assert_eq!(company_empty_strings.company_name, "");
        assert_eq!(company_empty_strings.company_url, Some("".to_string()));

        // エッジケース：負の値
        let company_negative = Company::new(
            -1,
            -1,
            "Test Company".to_string(),
            "テストカンパニー".to_string(),
            "てすとかんぱにー".to_string(),
            "Tesuto Kanpanii".to_string(),
            "Test".to_string(),
            "Test Company Inc.".to_string(),
            None,
            -1,
            -1,
            -1,
        );

        assert_eq!(company_negative.company_cd, -1);
        assert_eq!(company_negative.rr_cd, -1);
        assert_eq!(company_negative.company_type, -1);
        assert_eq!(company_negative.e_status, -1);
        assert_eq!(company_negative.e_sort, -1);
    }
}
