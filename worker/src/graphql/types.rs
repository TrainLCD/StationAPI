//! GraphQL のオブジェクト型。
//!
//! 公開スキーマは循環参照を避けるため Station/StationNested のように
//! 同一構造の型を 2 つ持つ。SDL を一致させる必要があるので、こちらも
//! マクロで同じ定義から 2 つの型を作る。
//!
//! 値は proto の型から変換する。IPA や TTS セグメントは use_case の DTO が
//! 計算しているので、domain エンティティからではなく proto を経由することで
//! そのロジックをそのまま使える。

use async_graphql::SimpleObject;
use stationapi::proto;

use super::enums::*;
use super::scalar::UInt32;

#[derive(SimpleObject, Clone)]
#[graphql(name = "TtsSegment")]
pub struct TtsSegment {
    pub surface: Option<String>,
    pub fallback_text: Option<String>,
    pub pronunciation: Option<String>,
    pub alphabet: Option<TtsAlphabet>,
    pub lang: Option<String>,
    pub separator: Option<String>,
}

impl From<proto::TtsSegment> for TtsSegment {
    fn from(v: proto::TtsSegment) -> Self {
        Self {
            surface: Some(v.surface),
            fallback_text: Some(v.fallback_text),
            pronunciation: Some(v.pronunciation),
            alphabet: Some(TtsAlphabet::from(v.alphabet)),
            lang: Some(v.lang),
            separator: Some(v.separator),
        }
    }
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "StationNumber")]
pub struct StationNumber {
    pub line_symbol: Option<String>,
    pub line_symbol_color: Option<String>,
    pub line_symbol_shape: Option<String>,
    pub station_number: Option<String>,
}

impl From<proto::StationNumber> for StationNumber {
    fn from(v: proto::StationNumber) -> Self {
        Self {
            line_symbol: Some(v.line_symbol),
            line_symbol_color: Some(v.line_symbol_color),
            line_symbol_shape: Some(v.line_symbol_shape),
            station_number: Some(v.station_number),
        }
    }
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "LineSymbol")]
pub struct LineSymbol {
    pub symbol: Option<String>,
    pub color: Option<String>,
    pub shape: Option<String>,
}

impl From<proto::LineSymbol> for LineSymbol {
    fn from(v: proto::LineSymbol) -> Self {
        Self {
            symbol: Some(v.symbol),
            color: Some(v.color),
            shape: Some(v.shape),
        }
    }
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "Company")]
pub struct Company {
    pub id: Option<i32>,
    pub railroad_id: Option<i32>,
    pub name_short: Option<String>,
    pub name_katakana: Option<String>,
    pub name_full: Option<String>,
    pub name_english_short: Option<String>,
    pub name_english_full: Option<String>,
    pub url: Option<String>,
    pub r#type: Option<CompanyType>,
    pub status: Option<OperationStatus>,
    pub name: Option<String>,
}

impl From<proto::Company> for Company {
    fn from(v: proto::Company) -> Self {
        Self {
            id: Some(v.id as i32),
            railroad_id: Some(v.railroad_id as i32),
            name_short: Some(v.name_short.clone()),
            name_katakana: Some(v.name_katakana),
            name_full: Some(v.name_full),
            name_english_short: Some(v.name_english_short),
            name_english_full: Some(v.name_english_full),
            url: v.url,
            r#type: Some(CompanyType::from(v.r#type)),
            status: Some(OperationStatus::from(v.status)),
            // 公開スキーマは name に nameShort を入れている
            name: Some(v.name_short),
        }
    }
}

/// Station と StationNested は同一構造。SDL を合わせるため両方を定義する。
macro_rules! define_station {
    ($ident:ident, $name:literal) => {
        #[derive(SimpleObject, Clone)]
        #[graphql(name = $name)]
        pub struct $ident {
            pub id: Option<i32>,
            pub group_id: Option<UInt32>,
            pub name: Option<String>,
            pub name_katakana: Option<String>,
            pub name_roman: Option<String>,
            pub name_chinese: Option<String>,
            pub name_korean: Option<String>,
            pub three_letter_code: Option<String>,
            pub prefecture_id: Option<i32>,
            pub postal_code: Option<String>,
            pub address: Option<String>,
            pub latitude: Option<f64>,
            pub longitude: Option<f64>,
            pub opened_at: Option<String>,
            pub closed_at: Option<String>,
            pub status: Option<OperationStatus>,
            pub station_numbers: Option<Vec<StationNumber>>,
            pub stop_condition: Option<StopCondition>,
            pub distance: Option<f64>,
            pub has_train_types: Option<bool>,
            pub train_type: Option<Box<TrainTypeNested>>,
            pub lines: Option<Vec<LineNested>>,
            pub line: Option<Box<LineNested>>,
            pub transport_type: Option<TransportType>,
            pub name_ipa: Option<String>,
            pub name_roman_ipa: Option<String>,
            pub name_tts_segments: Option<Vec<TtsSegment>>,
        }

        impl From<proto::Station> for $ident {
            fn from(v: proto::Station) -> Self {
                Self {
                    id: Some(v.id as i32),
                    group_id: Some(UInt32(v.group_id)),
                    name: Some(v.name),
                    name_katakana: Some(v.name_katakana),
                    name_roman: v.name_roman,
                    name_chinese: v.name_chinese,
                    name_korean: v.name_korean,
                    three_letter_code: v.three_letter_code,
                    prefecture_id: Some(v.prefecture_id as i32),
                    postal_code: Some(v.postal_code),
                    address: Some(v.address),
                    latitude: Some(v.latitude),
                    longitude: Some(v.longitude),
                    opened_at: Some(v.opened_at),
                    closed_at: Some(v.closed_at),
                    status: Some(OperationStatus::from(v.status)),
                    station_numbers: Some(v.station_numbers.into_iter().map(Into::into).collect()),
                    stop_condition: Some(StopCondition::from(v.stop_condition)),
                    distance: v.distance,
                    has_train_types: v.has_train_types,
                    train_type: v.train_type.map(|t| Box::new((*t).into())),
                    lines: Some(v.lines.into_iter().map(Into::into).collect()),
                    line: v.line.map(|l| Box::new((*l).into())),
                    transport_type: Some(TransportType::from(v.transport_type)),
                    name_ipa: v.name_ipa,
                    name_roman_ipa: v.name_roman_ipa,
                    name_tts_segments: Some(
                        v.name_tts_segments.into_iter().map(Into::into).collect(),
                    ),
                }
            }
        }
    };
}

macro_rules! define_line {
    ($ident:ident, $name:literal) => {
        #[derive(SimpleObject, Clone)]
        #[graphql(name = $name)]
        pub struct $ident {
            pub id: Option<i32>,
            pub name_short: Option<String>,
            pub name_katakana: Option<String>,
            pub name_full: Option<String>,
            pub name_roman: Option<String>,
            pub name_chinese: Option<String>,
            pub name_korean: Option<String>,
            pub color: Option<String>,
            pub line_type: Option<LineType>,
            pub line_symbols: Option<Vec<LineSymbol>>,
            pub status: Option<OperationStatus>,
            pub station: Option<Box<StationNested>>,
            pub company: Option<Company>,
            pub train_type: Option<Box<TrainTypeNested>>,
            pub average_distance: Option<f64>,
            pub transport_type: Option<TransportType>,
            pub name_ipa: Option<String>,
            pub name_roman_ipa: Option<String>,
            pub name_tts_segments: Option<Vec<TtsSegment>>,
        }

        impl From<proto::Line> for $ident {
            fn from(v: proto::Line) -> Self {
                Self {
                    id: Some(v.id as i32),
                    name_short: Some(v.name_short),
                    name_katakana: Some(v.name_katakana),
                    name_full: Some(v.name_full),
                    name_roman: v.name_roman,
                    name_chinese: v.name_chinese,
                    name_korean: v.name_korean,
                    color: Some(v.color),
                    line_type: Some(LineType::from(v.line_type)),
                    line_symbols: Some(v.line_symbols.into_iter().map(Into::into).collect()),
                    status: Some(OperationStatus::from(v.status)),
                    station: v.station.map(|s| Box::new((*s).into())),
                    company: v.company.map(Into::into),
                    train_type: v.train_type.map(|t| Box::new((*t).into())),
                    average_distance: Some(v.average_distance),
                    transport_type: Some(TransportType::from(v.transport_type)),
                    name_ipa: v.name_ipa,
                    name_roman_ipa: v.name_roman_ipa,
                    name_tts_segments: Some(
                        v.name_tts_segments.into_iter().map(Into::into).collect(),
                    ),
                }
            }
        }
    };
}

macro_rules! define_train_type {
    ($ident:ident, $name:literal) => {
        #[derive(SimpleObject, Clone)]
        #[graphql(name = $name)]
        pub struct $ident {
            pub id: Option<i32>,
            pub type_id: Option<i32>,
            pub group_id: Option<UInt32>,
            pub name: Option<String>,
            pub name_katakana: Option<String>,
            pub name_roman: Option<String>,
            pub name_chinese: Option<String>,
            pub name_korean: Option<String>,
            pub color: Option<String>,
            pub lines: Option<Vec<LineNested>>,
            pub line: Option<Box<LineNested>>,
            pub direction: Option<TrainDirection>,
            pub kind: Option<TrainTypeKind>,
            pub name_ipa: Option<String>,
            pub name_roman_ipa: Option<String>,
            pub name_tts_segments: Option<Vec<TtsSegment>>,
        }

        impl From<proto::TrainType> for $ident {
            fn from(v: proto::TrainType) -> Self {
                Self {
                    id: Some(v.id as i32),
                    type_id: Some(v.type_id as i32),
                    group_id: Some(UInt32(v.group_id)),
                    name: Some(v.name),
                    name_katakana: Some(v.name_katakana),
                    name_roman: v.name_roman,
                    name_chinese: v.name_chinese,
                    name_korean: v.name_korean,
                    color: Some(v.color),
                    lines: Some(v.lines.into_iter().map(Into::into).collect()),
                    line: v.line.map(|l| Box::new((*l).into())),
                    direction: Some(TrainDirection::from(v.direction)),
                    kind: Some(TrainTypeKind::from(v.kind)),
                    name_ipa: v.name_ipa,
                    name_roman_ipa: v.name_roman_ipa,
                    name_tts_segments: Some(
                        v.name_tts_segments.into_iter().map(Into::into).collect(),
                    ),
                }
            }
        }
    };
}

define_station!(Station, "Station");
define_station!(StationNested, "StationNested");
define_line!(Line, "Line");
define_line!(LineNested, "LineNested");
define_train_type!(TrainType, "TrainType");
define_train_type!(TrainTypeNested, "TrainTypeNested");

#[derive(SimpleObject)]
#[graphql(name = "Route")]
pub struct Route {
    pub id: Option<UInt32>,
    pub stops: Option<Vec<StationNested>>,
}

impl From<proto::Route> for Route {
    fn from(v: proto::Route) -> Self {
        Self {
            id: Some(UInt32(v.id)),
            stops: Some(v.stops.into_iter().map(Into::into).collect()),
        }
    }
}

#[derive(SimpleObject)]
#[graphql(name = "RoutePage")]
pub struct RoutePage {
    pub routes: Option<Vec<Route>>,
    pub next_page_token: Option<String>,
}

#[derive(SimpleObject)]
#[graphql(name = "RouteTypePage")]
pub struct RouteTypePage {
    pub train_types: Option<Vec<TrainType>>,
    pub next_page_token: Option<String>,
}

#[derive(SimpleObject)]
#[graphql(name = "EstimatedArrivalStop")]
pub struct EstimatedArrivalStop {
    pub station_id: Option<i32>,
    pub station_group_id: Option<UInt32>,
    pub cumulative_minutes: Option<f64>,
    pub stops_here: Option<bool>,
    pub departure_cumulative_minutes: Option<f64>,
}

#[derive(SimpleObject)]
#[graphql(name = "EstimatedArrivalRoute")]
pub struct EstimatedArrivalRoute {
    pub id: Option<i32>,
    pub stops: Option<Vec<EstimatedArrivalStop>>,
}

#[derive(SimpleObject)]
#[graphql(name = "EstimatedArrivalPage")]
pub struct EstimatedArrivalPage {
    pub routes: Option<Vec<EstimatedArrivalRoute>>,
}

#[derive(SimpleObject)]
#[graphql(name = "TrainRouteSegment")]
pub struct TrainRouteSegment {
    pub station: Option<StationNested>,
    pub stops: Option<bool>,
    pub distance_from_previous: Option<f64>,
    pub max_speed: Option<f64>,
    pub max_acceleration: Option<f64>,
    pub max_deceleration: Option<f64>,
}

impl From<proto::TrainRouteSegment> for TrainRouteSegment {
    fn from(v: proto::TrainRouteSegment) -> Self {
        Self {
            station: v.station.map(Into::into),
            stops: Some(v.stops),
            distance_from_previous: Some(v.distance_from_previous),
            max_speed: Some(v.max_speed),
            max_acceleration: Some(v.max_acceleration),
            max_deceleration: Some(v.max_deceleration),
        }
    }
}

#[derive(SimpleObject)]
#[graphql(name = "TrainRouteResponse")]
pub struct TrainRouteResponse {
    pub segments: Option<Vec<TrainRouteSegment>>,
}
