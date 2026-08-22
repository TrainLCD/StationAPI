//! GraphQL の enum。値の並びは BFF のスキーマに合わせてある。
//! proto 側は i32 で入ってくるので、範囲外はスキーマ既定値へ倒す。
//!
//! `OtherLineType` や `TransportTypeUnspecified` のように型名を含む値があるが、
//! これは BFF のスキーマ (ひいては proto) の名前をそのまま出す必要があるため。
#![allow(clippy::enum_variant_names)]

use async_graphql::Enum;

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
#[graphql(rename_items = "PascalCase", name = "LineType")]
pub enum LineType {
    OtherLineType,
    BulletTrain,
    Normal,
    Subway,
    Tram,
    MonorailOrAGT,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
#[graphql(rename_items = "PascalCase", name = "TrainTypeKind")]
pub enum TrainTypeKind {
    Default,
    Branch,
    Rapid,
    Express,
    LimitedExpress,
    HighSpeedRapid,
    CommuterRapid,
    BusRoute,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
#[graphql(rename_items = "PascalCase", name = "OperationStatus")]
pub enum OperationStatus {
    InOperation,
    NotOpened,
    Closed,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
#[graphql(rename_items = "PascalCase", name = "StopCondition")]
pub enum StopCondition {
    All,
    Not,
    Partial,
    Weekday,
    Holiday,
    PartialStop,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
#[graphql(rename_items = "PascalCase", name = "CompanyType")]
pub enum CompanyType {
    OtherCompany,
    #[graphql(name = "JR")]
    JR,
    Private,
    Major,
    SemiMajor,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
#[graphql(rename_items = "PascalCase", name = "TrainDirection")]
pub enum TrainDirection {
    Both,
    Inbound,
    Outbound,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
#[graphql(rename_items = "PascalCase", name = "TransportType")]
pub enum TransportType {
    TransportTypeUnspecified,
    Rail,
    Bus,
    RailAndBus,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
#[graphql(rename_items = "PascalCase", name = "TtsAlphabet")]
pub enum TtsAlphabet {
    TtsAlphabetUnspecified,
    Ipa,
    Yomigana,
    Plain,
}

macro_rules! from_i32 {
    ($ty:ident, $default:expr, $($value:expr => $variant:ident),+ $(,)?) => {
        impl From<i32> for $ty {
            fn from(value: i32) -> Self {
                match value {
                    $($value => $ty::$variant,)+
                    _ => $default,
                }
            }
        }
    };
}

from_i32!(LineType, LineType::OtherLineType,
    0 => OtherLineType, 1 => BulletTrain, 2 => Normal, 3 => Subway, 4 => Tram, 5 => MonorailOrAGT);
from_i32!(TrainTypeKind, TrainTypeKind::Default,
    0 => Default, 1 => Branch, 2 => Rapid, 3 => Express, 4 => LimitedExpress,
    5 => HighSpeedRapid, 6 => CommuterRapid, 7 => BusRoute);
from_i32!(OperationStatus, OperationStatus::InOperation,
    0 => InOperation, 1 => NotOpened, 2 => Closed);
from_i32!(StopCondition, StopCondition::All,
    0 => All, 1 => Not, 2 => Partial, 3 => Weekday, 4 => Holiday, 5 => PartialStop);
from_i32!(CompanyType, CompanyType::OtherCompany,
    0 => OtherCompany, 1 => JR, 2 => Private, 3 => Major, 4 => SemiMajor);
from_i32!(TrainDirection, TrainDirection::Both,
    0 => Both, 1 => Inbound, 2 => Outbound);
from_i32!(TransportType, TransportType::TransportTypeUnspecified,
    0 => TransportTypeUnspecified, 1 => Rail, 2 => Bus, 3 => RailAndBus);
from_i32!(TtsAlphabet, TtsAlphabet::TtsAlphabetUnspecified,
    0 => TtsAlphabetUnspecified, 1 => Ipa, 2 => Yomigana, 3 => Plain);

impl TransportType {
    /// GraphQL の引数を proto の値へ戻す。未指定は既定 (鉄道のみ) に倒す。
    pub fn to_proto(self) -> i32 {
        self as i32
    }
}
