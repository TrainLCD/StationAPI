use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Company {
    pub company_cd: i64,
    pub rr_cd: i64,
    pub company_name: String,
    pub company_name_k: String,
    pub company_name_h: String,
    pub company_name_r: String,
    pub company_name_en: String,
    pub company_name_full_en: String,
    pub company_url: Option<String>,
    pub company_type: i64,
    pub e_status: i64,
    pub e_sort: i64,
}

impl Company {
    #![allow(clippy::too_many_arguments)]
    pub fn new(
        company_cd: i64,
        rr_cd: i64,
        company_name: String,
        company_name_k: String,
        company_name_h: String,
        company_name_r: String,
        company_name_en: String,
        company_name_full_en: String,
        company_url: Option<String>,
        company_type: i64,
        e_status: i64,
        e_sort: i64,
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
    use super::Company;

    #[test]
    fn new() {
        let company = Company::new(
            1,
            2,
            "JR東日本".to_string(),
            "ジェイアールヒガシニホン".to_string(),
            "東日本旅客鉄道株式会社".to_string(),
            "JR東日本".to_string(),
            "JR East".to_string(),
            "East Japan Railway Company".to_string(),
            Some("https://www.jreast.co.jp/".to_string()),
            1,
            0,
            1,
        );
        assert_eq!(
            company,
            Company {
                company_cd: 1,
                rr_cd: 2,
                company_name: "JR東日本".to_string(),
                company_name_k: "ジェイアールヒガシニホン".to_string(),
                company_name_h: "東日本旅客鉄道株式会社".to_string(),
                company_name_r: "JR東日本".to_string(),
                company_name_en: "JR East".to_string(),
                company_name_full_en: "East Japan Railway Company".to_string(),
                company_url: Some("https://www.jreast.co.jp/".to_string()),
                company_type: 1,
                e_status: 0,
                e_sort: 1
            }
        );
    }
}
