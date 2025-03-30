use serde::{Deserialize, Serialize};

use crate::proto::StopCondition;

use super::{line::Line, station_number::StationNumber, train_type::TrainType as TrainTypeEntity};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Station {
    pub station_cd: i64,
    pub station_g_cd: i64,
    pub station_name: String,
    pub station_name_k: String,
    pub station_name_r: Option<String>,
    pub station_name_zh: Option<String>,
    pub station_name_ko: Option<String>,
    pub station_numbers: Vec<StationNumber>,
    pub station_number1: Option<String>,
    pub station_number2: Option<String>,
    pub station_number3: Option<String>,
    pub station_number4: Option<String>,
    pub three_letter_code: Option<String>,
    pub line_cd: i64,
    pub line: Option<Box<Line>>,
    pub lines: Vec<Line>,
    pub pref_cd: i64,
    pub post: String,
    pub address: String,
    pub lon: f64,
    pub lat: f64,
    pub open_ymd: String,
    pub close_ymd: String,
    pub e_status: i64,
    pub e_sort: i64,
    pub stop_condition: StopCondition,
    pub distance: Option<f64>,
    pub has_train_types: bool,
    pub train_type: Option<Box<TrainTypeEntity>>,
    // linesからJOIN
    pub company_cd: Option<i64>,
    pub line_name: Option<String>,
    pub line_name_k: Option<String>,
    pub line_name_h: Option<String>,
    pub line_name_r: Option<String>,
    pub line_name_zh: Option<String>,
    pub line_name_ko: Option<String>,
    pub line_color_c: Option<String>,
    pub line_type: Option<i64>,
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
    pub average_distance: f64,
    // station_station_typesからJOIN
    pub type_id: Option<i64>,
    pub sst_id: Option<i64>,
    pub type_cd: Option<i64>,
    pub line_group_cd: Option<i64>,
    pub pass: Option<i64>,
    // typesからJOIN
    pub type_name: Option<String>,
    pub type_name_k: Option<String>,
    pub type_name_r: Option<String>,
    pub type_name_zh: Option<String>,
    pub type_name_ko: Option<String>,
    pub color: Option<String>,
    pub direction: Option<i64>,
    pub kind: Option<i64>,
}

impl Station {
    #![allow(clippy::too_many_arguments)]
    pub fn new(
        station_cd: i64,
        station_g_cd: i64,
        station_name: String,
        station_name_k: String,
        station_name_r: Option<String>,
        station_name_zh: Option<String>,
        station_name_ko: Option<String>,
        station_numbers: Vec<StationNumber>,
        station_number1: Option<String>,
        station_number2: Option<String>,
        station_number3: Option<String>,
        station_number4: Option<String>,
        three_letter_code: Option<String>,
        line_cd: i64,
        line: Option<Box<Line>>,
        lines: Vec<Line>,
        pref_cd: i64,
        post: String,
        address: String,
        lon: f64,
        lat: f64,
        open_ymd: String,
        close_ymd: String,
        e_status: i64,
        e_sort: i64,
        stop_condition: StopCondition,
        distance: Option<f64>,
        has_train_types: bool,
        train_type: Option<Box<TrainTypeEntity>>,
        company_cd: Option<i64>,
        line_name: Option<String>,
        line_name_k: Option<String>,
        line_name_h: Option<String>,
        line_name_r: Option<String>,
        line_name_zh: Option<String>,
        line_name_ko: Option<String>,
        line_color_c: Option<String>,
        line_type: Option<i64>,
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
        line_group_cd: Option<i64>,
        average_distance: f64,
        pass: Option<i64>,
        type_id: Option<i64>,
        sst_id: Option<i64>,
        type_cd: Option<i64>,
        type_name: Option<String>,
        type_name_k: Option<String>,
        type_name_r: Option<String>,
        type_name_zh: Option<String>,
        type_name_ko: Option<String>,
        color: Option<String>,
        direction: Option<i64>,
        kind: Option<i64>,
    ) -> Self {
        Self {
            station_cd,
            station_g_cd,
            station_name,
            station_name_k,
            station_name_r,
            station_name_zh,
            station_name_ko,
            station_numbers,
            station_number1,
            station_number2,
            station_number3,
            station_number4,
            three_letter_code,
            line_cd,
            line,
            lines,
            pref_cd,
            post,
            address,
            lon,
            lat,
            open_ymd,
            close_ymd,
            e_status,
            e_sort,
            stop_condition,
            distance,
            has_train_types,
            train_type,
            company_cd,
            line_name,
            line_name_k,
            line_name_h,
            line_name_r,
            line_name_zh,
            line_name_ko,
            line_color_c,
            line_type,
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
            line_group_cd,
            pass,
            average_distance,
            type_id,
            sst_id,
            type_cd,
            type_name,
            type_name_k,
            type_name_r,
            type_name_zh,
            type_name_ko,
            color,
            direction,
            kind,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Station;
    use crate::domain::entity::{line::Line, station_number::StationNumber};
    use crate::proto::StopCondition;

    #[test]
    fn new() {
        let station_numbers = vec![StationNumber::new(
            "JY".to_string(),
            "#80C241".to_string(),
            "SQUARE".to_string(),
            "01".to_string(),
        )];

        let lines = vec![Line::new(
            11302,
            1,
            None,
            "山手線".to_string(),
            "ヤマノテセン".to_string(),
            "Yamanote Line".to_string(),
            Some("山手線".to_string()),
            Some("山手线".to_string()),
            Some("야마노테선".to_string()),
            Some("#80C241".to_string()),
            Some(1),
            vec![],
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
            1,
            None,
            None,
            None,
            None,
            None,
            29.0,
        )];

        let station = Station::new(
            1130208,
            1130208,
            "渋谷".to_string(),
            "シブヤ".to_string(),
            Some("渋谷".to_string()),
            Some("涩谷".to_string()),
            Some("시부야".to_string()),
            station_numbers,
            Some("20".to_string()),
            None,
            None,
            None,
            Some("SBY".to_string()),
            11302,
            None,
            lines,
            13,
            "150-0043".to_string(),
            "東京都渋谷区道玄坂一丁目1-1".to_string(),
            139.701238,
            35.658871,
            "1885-03-01".to_string(),
            "0000-00-00".to_string(),
            0,
            1130205,
            StopCondition::All,
            None,
            true,
            None,
            Some(2),
            Some("山手線".to_string()),
            Some("ヤマノテセン".to_string()),
            Some("山手線".to_string()),
            Some("Yamanote Line".to_string()),
            Some("山手线".to_string()),
            Some("야마노테선".to_string()),
            Some("#80C241".to_string()),
            Some(2),
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
            Some(11302),
            1075.968412,
            Some(0),
            Some(20),
            Some(99999), // NOTE: あえて存在しない前提の値にしている
            Some(100),
            Some("普通".to_string()),
            Some("フツウ".to_string()),
            Some("Local".to_string()),
            Some("慢车".to_string()),
            Some("보통".to_string()),
            Some("#1F63C6".to_string()),
            Some(0),
            Some(0),
        );

        assert_eq!(station.station_cd, 1130208);
        assert_eq!(station.station_g_cd, 1130208);
        assert_eq!(station.station_name, "渋谷");
        assert_eq!(station.station_name_k, "シブヤ");
        assert_eq!(station.station_name_r, Some("渋谷".to_string()));
        assert_eq!(station.station_name_zh, Some("涩谷".to_string()));
        assert_eq!(station.station_name_ko, Some("시부야".to_string()));
        assert_eq!(station.station_number1, Some("20".to_string()));
        assert_eq!(station.three_letter_code, Some("SBY".to_string()));
        assert_eq!(station.line_cd, 11302);
        assert_eq!(station.pref_cd, 13);
        assert_eq!(station.post, "150-0043");
        assert_eq!(station.address, "東京都渋谷区道玄坂一丁目1-1");
        assert_eq!(station.lon, 139.701238);
        assert_eq!(station.lat, 35.658871);
        assert_eq!(station.open_ymd, "1885-03-01");
        assert_eq!(station.close_ymd, "0000-00-00");
        assert_eq!(station.e_status, 0);
        assert_eq!(station.e_sort, 1130205);
        assert_eq!(station.stop_condition, StopCondition::All);
        assert_eq!(station.distance, None);
        assert!(station.has_train_types);
        assert_eq!(station.company_cd, Some(2));
        assert_eq!(station.line_name, Some("山手線".to_string()));
        assert_eq!(station.line_name_k, Some("ヤマノテセン".to_string()));
        assert_eq!(station.line_name_h, Some("山手線".to_string()));
        assert_eq!(station.line_name_r, Some("Yamanote Line".to_string()));
        assert_eq!(station.line_name_zh, Some("山手线".to_string()));
        assert_eq!(station.line_name_ko, Some("야마노테선".to_string()));
        assert_eq!(station.line_color_c, Some("#80C241".to_string()));
        assert_eq!(station.line_type, Some(2));
        assert_eq!(station.line_symbol1, Some("JY".to_string()));
        assert_eq!(station.line_symbol2, None);
        assert_eq!(station.line_symbol3, None);
        assert_eq!(station.line_symbol1_color, Some("#80C241".to_string()));
        assert_eq!(station.line_symbol2_color, None);
        assert_eq!(station.line_symbol3_color, None);
        assert_eq!(station.line_symbol4_color, None);
        assert_eq!(station.line_symbol1_shape, Some("SQUARE".to_string()));
        assert_eq!(station.line_symbol2_shape, None);
        assert_eq!(station.line_symbol3_shape, None);
        assert_eq!(station.line_symbol4_shape, None);
        assert_eq!(station.average_distance, 1075.968412);
        assert_eq!(station.type_id, Some(20));
    }
}
