use fake::Dummy;

#[derive(Dummy, Clone, Debug)]
pub struct Company {
    pub company_cd: u32,
    pub rr_cd: u32,
    pub company_name: String,
    pub company_name_k: String,
    pub company_name_h: String,
    pub company_name_r: String,
    pub company_name_en: String,
    pub company_name_full_en: String,
    pub company_url: String,
    pub company_type: u32,
    pub e_status: u32,
    pub e_sort: u32,
}

impl Company {
    #![allow(clippy::too_many_arguments)]
    pub fn new(
        company_cd: u32,
        rr_cd: u32,
        company_name: String,
        company_name_k: String,
        company_name_h: String,
        company_name_r: String,
        company_name_en: String,
        company_name_full_en: String,
        company_url: String,
        company_type: u32,
        e_status: u32,
        e_sort: u32,
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
