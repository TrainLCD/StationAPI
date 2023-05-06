use crate::service::CompanyResponse;

#[derive(sqlx::FromRow, Clone)]
pub struct Company {
    pub company_cd: u32,
    pub rr_cd: u32,
    pub company_name: String,
    pub company_name_k: String,
    pub company_name_h: String,
    pub company_name_r: String,
    pub company_name_en: String,
    company_name_full_en: String,
    pub company_url: String,
    pub company_type: i32,
    pub e_status: u32,
    pub e_sort: u32,
}

impl From<Company> for CompanyResponse {
    fn from(value: Company) -> Self {
        CompanyResponse {
            id: value.company_cd,
            railroad_id: value.rr_cd,
            name_general: value.company_name,
            name_katakana: value.company_name_k,
            name_full: value.company_name_h,
            name_short: value.company_name_r,
            name_english_short: value.company_name_en,
            name_english_full: value.company_name_full_en,
            url: value.company_url,
            r#type: value.company_type,
            status: value.e_status as i32,
        }
    }
}
