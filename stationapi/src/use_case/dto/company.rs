use crate::{domain::entity::company::Company, model::Company as ModelCompany};

impl From<Company> for ModelCompany {
    fn from(company: Company) -> Self {
        Self {
            id: company.company_cd as u32,
            railroad_id: company.rr_cd as u32,
            name: company.company_name,
            name_short: company.company_name_r,
            name_katakana: company.company_name_k,
            name_full: company.company_name_h,
            name_english_short: company.company_name_en,
            name_english_full: company.company_name_full_en,
            url: company.company_url,
            r#type: company.company_type,
            status: company.e_status,
        }
    }
}
