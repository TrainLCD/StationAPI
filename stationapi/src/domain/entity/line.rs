use serde::{Deserialize, Serialize};

use super::{company::Company, line_symbol::LineSymbol, station::Station, train_type::TrainType};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Line {
    pub line_cd: i64,
    pub company_cd: i64,
    pub company: Option<Company>,
    pub line_name: Option<String>,
    pub line_name_k: Option<String>,
    pub line_name_h: Option<String>,
    pub line_name_r: Option<String>,
    pub line_name_zh: Option<String>,
    pub line_name_ko: Option<String>,
    pub line_color_c: Option<String>,
    pub line_type: Option<i64>,
    pub line_symbols: Vec<LineSymbol>,
    pub line_symbol1: Option<String>,
    pub line_symbol2: Option<String>,
    pub line_symbol3: Option<String>,
    pub line_symbol4: Option<String>,
    pub line_symbol1_color: Option<String>,
    pub line_symbol2_color: Option<String>,
    pub line_symbol3_color: Option<String>,
    pub line_symbol4_color: Option<String>,
    pub line_symbol1_shape: Option<String>,
    pub line_symbol2_shape: Option<String>,
    pub line_symbol3_shape: Option<String>,
    pub line_symbol4_shape: Option<String>,
    pub e_status: i64,
    pub e_sort: i64,
    pub average_distance: Option<f32>,
    pub station: Option<Station>,
    pub train_type: Option<TrainType>,
    pub line_group_cd: Option<i64>,
    pub station_cd: Option<i64>,
    pub station_g_cd: Option<i64>,
}

impl Line {
    #![allow(clippy::too_many_arguments)]
    pub fn new(
        line_cd: i64,
        company_cd: i64,
        company: Option<Company>,
        line_name: Option<String>,
        line_name_k: Option<String>,
        line_name_h: Option<String>,
        line_name_r: Option<String>,
        line_name_zh: Option<String>,
        line_name_ko: Option<String>,
        line_color_c: Option<String>,
        line_type: Option<i64>,
        line_symbols: Vec<LineSymbol>,
        line_symbol1: Option<String>,
        line_symbol2: Option<String>,
        line_symbol3: Option<String>,
        line_symbol4: Option<String>,
        line_symbol1_color: Option<String>,
        line_symbol2_color: Option<String>,
        line_symbol3_color: Option<String>,
        line_symbol4_color: Option<String>,
        line_symbol1_shape: Option<String>,
        line_symbol2_shape: Option<String>,
        line_symbol3_shape: Option<String>,
        line_symbol4_shape: Option<String>,
        e_status: i64,
        e_sort: i64,
        station: Option<Station>,
        train_type: Option<TrainType>,
        line_group_cd: Option<i64>,
        station_cd: Option<i64>,
        station_g_cd: Option<i64>,
        average_distance: Option<f32>,
    ) -> Self {
        Self {
            line_cd,
            company_cd,
            company,
            line_name,
            line_name_k,
            line_name_h,
            line_name_r,
            line_name_zh,
            line_name_ko,
            line_color_c,
            line_type,
            line_symbols,
            line_symbol1,
            line_symbol2,
            line_symbol3,
            line_symbol4,
            line_symbol1_color,
            line_symbol2_color,
            line_symbol3_color,
            line_symbol4_color,
            line_symbol1_shape,
            line_symbol2_shape,
            line_symbol3_shape,
            line_symbol4_shape,
            e_status,
            e_sort,
            station,
            train_type,
            line_group_cd,
            station_cd,
            station_g_cd,
            average_distance,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Line;
    use crate::domain::entity::company::Company;
    use crate::domain::entity::line_symbol::LineSymbol;

    #[test]
    fn new() {
        let company = Company::new(
            2,
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

        let line_symbols = vec![LineSymbol::new(
            "JY".to_string(),
            "#80C241".to_string(),
            "SQUARE".to_string(),
        )];

        let line = Line::new(
            11302,
            2,
            Some(company),
            Some("山手線".to_string()),
            Some("ヤマノテセン".to_string()),
            Some("山手線".to_string()),
            Some("Yamanote Line".to_string()),
            Some("山手线".to_string()),
            Some("야마노테선".to_string()),
            Some("#80C241".to_string()),
            Some(2),
            line_symbols,
            Some("JY".to_string()),
            None,
            None,
            None,
            Some("#80C241".to_string()),
            None,
            None,
            None,
            Some("SQUARE".to_string()),
            None,
            None,
            None,
            0,
            11302,
            None,
            None,
            None,
            None,
            None,
            Some(1_075.968_4),
        );

        assert_eq!(line.line_cd, 11302);
        assert_eq!(line.company_cd, 2);
        assert_eq!(line.line_name, Some("山手線".to_string()));
        assert_eq!(line.line_name_k, Some("ヤマノテセン".to_string()));
        assert_eq!(line.line_name_h, Some("山手線".to_string()));
        assert_eq!(line.line_name_r, Some("Yamanote Line".to_string()));
        assert_eq!(line.line_name_zh, Some("山手线".to_string()));
        assert_eq!(line.line_name_ko, Some("야마노테선".to_string()));
        assert_eq!(line.line_color_c, Some("#80C241".to_string()));
        assert_eq!(line.line_type, Some(2));
        assert_eq!(line.line_symbol1, Some("JY".to_string()));
        assert_eq!(line.line_symbol1_color, Some("#80C241".to_string()));
        assert_eq!(line.line_symbol1_shape, Some("SQUARE".to_string()));
        assert_eq!(line.e_status, 0);
        assert_eq!(line.e_sort, 11302);
        assert_eq!(line.average_distance, Some(1_075.968_4));
    }
}
