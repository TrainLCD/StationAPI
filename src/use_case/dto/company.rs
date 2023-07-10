use crate::{domain::entity::company::Company, pb::Company as GrpcCompany};

impl From<Company> for GrpcCompany {
    fn from(company: Company) -> Self {
        Self {
            id: company.company_cd,
            railroad_id: company.rr_cd,
            name: company.company_name,
            name_katakana: company.company_name_k,
            name_full: company.company_name_h,
            name_english_short: company.company_name_en,
            name_english_full: company.company_name_full_en,
            url: company.company_url,
            r#type: company.company_type as i32,
            status: company.e_status as i32,
        }
    }
}
