use crate::{
    domain::{
        entity::{gtfs::TransportType, station::Station},
        ipa::{katakana_to_ipa, station_name_to_ipa},
    },
    proto::{Station as GrpcStation, TransportType as GrpcTransportType},
};

impl From<TransportType> for i32 {
    fn from(value: TransportType) -> Self {
        match value {
            TransportType::Rail => GrpcTransportType::Rail as i32,
            TransportType::Bus => GrpcTransportType::Bus as i32,
        }
    }
}

impl From<Station> for GrpcStation {
    fn from(station: Station) -> Self {
        let name_ipa = katakana_to_ipa(&station.station_name_k).filter(|ipa| !ipa.is_empty());
        let name_roman_ipa =
            station_name_to_ipa(&station.station_name_k, station.station_name_r.as_deref());
        Self {
            id: station.station_cd as u32,
            group_id: station.station_g_cd as u32,
            name: station.station_name,
            name_katakana: station.station_name_k,
            name_roman: station.station_name_r,
            name_chinese: station.station_name_zh,
            name_korean: station.station_name_ko,
            three_letter_code: station.three_letter_code,
            lines: station.lines.into_iter().map(|line| line.into()).collect(),
            line: station.line.map(|line| Box::new((*line).into())),
            prefecture_id: station.pref_cd as u32,
            postal_code: station.post,
            address: station.address,
            latitude: station.lat,
            longitude: station.lon,
            opened_at: station.open_ymd,
            closed_at: station.close_ymd,
            status: station.e_status,
            station_numbers: station
                .station_numbers
                .into_iter()
                .map(|num| num.into())
                .collect(),
            stop_condition: station.stop_condition.into(),
            distance: station.distance,
            has_train_types: Some(station.has_train_types),
            train_type: station.train_type.map(|tt| Box::new((*tt).into())),
            transport_type: station.transport_type.into(),
            name_ipa,
            name_roman_ipa,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::entity::{gtfs::TransportType, station::Station},
        proto::StopCondition,
    };

    fn create_test_station(name: &str, name_katakana: &str, name_roman: Option<&str>) -> Station {
        Station::new(
            1,
            1,
            name.to_string(),
            name_katakana.to_string(),
            name_roman.map(str::to_string),
            None,
            None,
            vec![],
            None,
            None,
            None,
            None,
            None,
            1,
            None,
            vec![],
            12,
            String::new(),
            String::new(),
            0.0,
            0.0,
            String::new(),
            String::new(),
            0,
            0,
            StopCondition::All,
            None,
            false,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            TransportType::Rail,
        )
    }

    #[test]
    fn test_station_sets_expected_roman_ipa_for_inagekaigan() {
        let grpc_station: GrpcStation =
            create_test_station("稲毛海岸", "イナゲカイガン", Some("Inagekaigan")).into();

        assert_eq!(
            grpc_station.name_roman_ipa,
            Some("inage ka.igaɴ".to_string())
        );
    }

    #[test]
    fn test_station_name_roman_ipa_falls_back_to_katakana() {
        let grpc_station: GrpcStation = create_test_station("渋谷", "シブヤ", Some("???")).into();

        assert_eq!(grpc_station.name_roman_ipa, Some("ɕibɯja".to_string()));
    }
}
