use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Company {
    pub company_cd: i32,
    pub rr_cd: i32,
    pub company_name: String,
    pub company_name_k: String,
    pub company_name_h: String,
    pub company_name_r: String,
    pub company_name_en: String,
    pub company_name_full_en: String,
    pub company_url: String,
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
        company_url: String,
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
