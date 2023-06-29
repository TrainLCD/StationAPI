use getset::{Getters, Setters};

use crate::pb::LineSymbol;

#[derive(Debug, Clone, PartialEq, Getters, Setters)]
pub struct Line {
    #[getset(get = "pub")]
    pub line_cd: u32,
    #[getset(get = "pub")]
    pub company_cd: u32,
    #[getset(get = "pub")]
    pub line_name: String,
    #[getset(get = "pub")]
    pub line_name_k: String,
    #[getset(get = "pub")]
    pub line_name_h: String,
    #[getset(get = "pub")]
    pub line_name_r: String,
    #[getset(get = "pub")]
    pub line_name_zh: Option<String>,
    #[getset(get = "pub")]
    pub line_name_ko: Option<String>,
    #[getset(get = "pub")]
    pub line_color_c: String,
    #[getset(get = "pub")]
    pub line_color_t: String,
    #[getset(get = "pub")]
    pub line_type: u32,
    #[getset(get = "pub", set = "pub")]
    pub line_symbols: Vec<LineSymbol>,
    #[getset(get = "pub")]
    pub line_symbol_primary: Option<String>,
    #[getset(get = "pub")]
    pub line_symbol_secondary: Option<String>,
    #[getset(get = "pub")]
    pub line_symbol_extra: Option<String>,
    #[getset(get = "pub")]
    pub line_symbol_primary_color: Option<String>,
    #[getset(get = "pub")]
    pub line_symbol_secondary_color: Option<String>,
    #[getset(get = "pub")]
    pub line_symbol_extra_color: Option<String>,
    #[getset(get = "pub")]
    pub line_symbol_primary_shape: Option<String>,
    #[getset(get = "pub")]
    pub line_symbol_secondary_shape: Option<String>,
    #[getset(get = "pub")]
    pub line_symbol_extra_shape: Option<String>,
    #[getset(get = "pub")]
    pub lon: f64,
    #[getset(get = "pub")]
    pub lat: f64,
    #[getset(get = "pub")]
    pub zoom: u32,
    #[getset(get = "pub")]
    pub e_status: u32,
    #[getset(get = "pub")]
    pub e_sort: u32,
}

impl Line {
    #![allow(clippy::too_many_arguments)]
    pub fn new(
        line_cd: u32,
        company_cd: u32,
        line_name: String,
        line_name_k: String,
        line_name_h: String,
        line_name_r: String,
        line_name_zh: Option<String>,
        line_name_ko: Option<String>,
        line_color_c: String,
        line_color_t: String,
        line_type: u32,
        line_symbols: Vec<LineSymbol>,
        line_symbol_primary: Option<String>,
        line_symbol_secondary: Option<String>,
        line_symbol_extra: Option<String>,
        line_symbol_primary_color: Option<String>,
        line_symbol_secondary_color: Option<String>,
        line_symbol_extra_color: Option<String>,
        line_symbol_primary_shape: Option<String>,
        line_symbol_secondary_shape: Option<String>,
        line_symbol_extra_shape: Option<String>,
        lon: f64,
        lat: f64,
        zoom: u32,
        e_status: u32,
        e_sort: u32,
    ) -> Self {
        Self {
            line_cd,
            company_cd,
            line_name,
            line_name_k,
            line_name_h,
            line_name_r,
            line_name_zh,
            line_name_ko,
            line_color_c,
            line_color_t,
            line_type,
            line_symbols,
            line_symbol_primary,
            line_symbol_secondary,
            line_symbol_extra,
            line_symbol_primary_color,
            line_symbol_secondary_color,
            line_symbol_extra_color,
            line_symbol_primary_shape,
            line_symbol_secondary_shape,
            line_symbol_extra_shape,
            lon,
            lat,
            zoom,
            e_status,
            e_sort,
        }
    }
}
