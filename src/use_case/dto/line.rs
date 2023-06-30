use crate::{domain::entity::line::Line, pb::LineResponse};
use prost::alloc::boxed::Box;

impl From<Line> for LineResponse {
    fn from(value: Line) -> Self {
        Self {
            id: value.line_cd,
            name_short: value.line_name,
            name_katakana: value.line_name_k,
            name_full: value.line_name_h,
            name_roman: value.line_name_r,
            name_chinese: value.line_name_zh.unwrap_or("".to_string()),
            name_korean: value.line_name_ko.unwrap_or("".to_string()),
            color: value.line_color_c,
            line_type: value.line_type as i32,
            company_id: value.company_cd,
            line_symbols: value.line_symbols,
            status: value.e_status as i32,
            station: value.station.map(Box::new),
            company: value.company,
        }
    }
}

#[cfg(test)]
mod tests {
    use fake::{Fake, Faker};

    #[test]
    fn from() {
        use super::*;
        let line: Line = Faker.fake();
        let Line {
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
            line_color_t: _,
            line_type,
            line_symbols,
            line_symbol_primary: _,
            line_symbol_secondary: _,
            line_symbol_extra: _,
            line_symbol_primary_color: _,
            line_symbol_secondary_color: _,
            line_symbol_extra_color: _,
            line_symbol_primary_shape: _,
            line_symbol_secondary_shape: _,
            line_symbol_extra_shape: _,
            lon: _,
            lat: _,
            zoom: _,
            e_status,
            e_sort: _,
            station,
        } = line.clone();
        let actual = LineResponse::from(line);

        assert_eq!(actual.id, line_cd);
        assert_eq!(actual.company_id, company_cd);
        assert_eq!(actual.name_short, line_name);
        assert_eq!(actual.name_katakana, line_name_k);
        assert_eq!(actual.name_full, line_name_h);
        assert_eq!(actual.name_roman, line_name_r);
        assert_eq!(actual.name_chinese, line_name_zh.unwrap_or("".to_string()));
        assert_eq!(actual.name_korean, line_name_ko.unwrap_or("".to_string()));
        assert_eq!(actual.color, line_color_c);
        assert_eq!(actual.line_type, line_type as i32);
        assert_eq!(actual.line_symbols, line_symbols);
        assert_eq!(actual.status, e_status as i32);
        assert_eq!(actual.station, None);
        assert_eq!(actual.company, company);
        assert_eq!(actual.station, station.map(Box::new));
    }
}
