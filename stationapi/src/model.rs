//! API が返す値の表現。
//!
//! かつては gRPC の `.proto` から prost が生成した型をそのまま使っていた。
//! gRPC をやめたあとも、IPA や TTS セグメントの組み立てが use_case 層の
//! DTO 変換にぶら下がっているため、domain エンティティと GraphQL 型の間に
//! 挟まるこの表現は残してある。
//!
//! 値の並びと名前は公開スキーマ (`schema/public.graphql`) の元になっている
//! ので、勝手に変えるとクライアントが壊れる。列挙子の数値は
//! クライアントとの取り決めなので、途中に挿入せず末尾に足すこと。

/// `i32` から列挙型への変換と、既定値を持つ列挙型を定義する。
///
/// 数値は永続化された値 (`types.kind` など) やクライアントとの取り決めに
/// 対応するため、明示的に書く。
macro_rules! coded_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident {
            $($variant:ident = $value:literal),+ $(,)?
        }
        default = $default:ident;
    ) => {
        $(#[$meta])*
        // serde はドメインエンティティが列挙型を直に持つため要る
        // (`Station::stop_condition` など)。
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[derive(serde::Serialize, serde::Deserialize)]
        #[repr(i32)]
        pub enum $name {
            $($variant = $value),+
        }

        impl Default for $name {
            fn default() -> Self {
                $name::$default
            }
        }

        impl From<$name> for i32 {
            fn from(value: $name) -> Self {
                value as i32
            }
        }

        impl TryFrom<i32> for $name {
            type Error = UnknownCode;

            fn try_from(value: i32) -> Result<Self, Self::Error> {
                match value {
                    $($value => Ok($name::$variant),)+
                    _ => Err(UnknownCode(value)),
                }
            }
        }
    };
}

/// 列挙型に無い数値が来たときのエラー。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownCode(pub i32);

impl std::fmt::Display for UnknownCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "未知のコード: {}", self.0)
    }
}

impl std::error::Error for UnknownCode {}

coded_enum! {
    /// 駅・路線・事業者の運用状態。
    pub enum OperationStatus {
        InOperation = 0,
        NotOpened = 1,
        Closed = 2,
    }
    default = InOperation;
}

coded_enum! {
    /// 鉄道かバスか。検索の絞り込みにも使う。
    pub enum TransportType {
        TransportTypeUnspecified = 0,
        Rail = 1,
        Bus = 2,
        RailAndBus = 3,
    }
    default = TransportTypeUnspecified;
}

coded_enum! {
    /// TTS の読みをどの表記体系で返しているか。
    pub enum TtsAlphabet {
        TtsAlphabetUnspecified = 0,
        Ipa = 1,
        Yomigana = 2,
        Plain = 3,
    }
    default = TtsAlphabetUnspecified;
}

coded_enum! {
    /// その系統がその駅に停車するかどうか。
    pub enum StopCondition {
        All = 0,
        Not = 1,
        Partial = 2,
        Weekday = 3,
        Holiday = 4,
        PartialStop = 5,
    }
    default = All;
}

coded_enum! {
    /// 系統の運転方向。
    pub enum TrainDirection {
        Both = 0,
        Inbound = 1,
        Outbound = 2,
    }
    default = Both;
}

coded_enum! {
    /// 列車種別の区分。到達時間の推定で速度倍率を引くのに使う。
    pub enum TrainTypeKind {
        Default = 0,
        Branch = 1,
        Rapid = 2,
        Express = 3,
        LimitedExpress = 4,
        HighSpeedRapid = 5,
        CommuterRapid = 6,
        BusRoute = 7,
    }
    default = Default;
}

coded_enum! {
    /// 路線の種別。速度の既定値を引くのに使う。
    pub enum LineType {
        OtherLineType = 0,
        BulletTrain = 1,
        Normal = 2,
        Subway = 3,
        Tram = 4,
        MonorailOrAgt = 5,
    }
    default = OtherLineType;
}

coded_enum! {
    /// 事業者の区分。
    pub enum CompanyType {
        OtherCompany = 0,
        Jr = 1,
        Private = 2,
        Major = 3,
        SemiMajor = 4,
    }
    default = OtherCompany;
}

/// 列挙型を持つ整数フィールドは `i32` のまま持つ。範囲外の値が来ても
/// 応答全体を落とさず、表示側で既定値へ倒すため。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Company {
    pub id: u32,
    pub railroad_id: u32,
    pub name_short: String,
    pub name_katakana: String,
    pub name_full: String,
    pub name_english_short: String,
    pub name_english_full: String,
    pub url: Option<String>,
    /// [`CompanyType`]
    pub r#type: i32,
    /// [`OperationStatus`]
    pub status: i32,
    pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LineSymbol {
    pub symbol: String,
    pub color: String,
    pub shape: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StationNumber {
    pub line_symbol: String,
    pub line_symbol_color: String,
    pub line_symbol_shape: String,
    pub station_number: String,
}

/// TTS 用の 1 セグメント。
///
/// `lang` は "en-US" や "ja-JP" のような BCP-47 言語タグで、`surface` /
/// `fallback_text` / `pronunciation` をクライアントがどの言語として解釈するかを
/// 示す。`separator` はこのセグメントの後ろにだけ付ける区切りで、次の
/// セグメントとの間に挿入し、末尾では無視する。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TtsSegment {
    pub surface: String,
    pub fallback_text: String,
    pub pronunciation: String,
    /// [`TtsAlphabet`]
    pub alphabet: i32,
    pub lang: String,
    pub separator: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrainType {
    pub id: u32,
    pub type_id: u32,
    pub group_id: u32,
    pub name: String,
    pub name_katakana: String,
    pub name_roman: Option<String>,
    pub name_chinese: Option<String>,
    pub name_korean: Option<String>,
    pub color: String,
    pub lines: Vec<Line>,
    /// 同一種別の問い合わせのときだけ入る。`Line` と相互に参照するため
    /// 間接化が要る。
    pub line: Option<Box<Line>>,
    /// [`TrainDirection`]
    pub direction: i32,
    /// [`TrainTypeKind`]
    pub kind: i32,
    /// TTS 用カタカナ名の IPA 転写
    pub name_ipa: Option<String>,
    /// TTS 用英語名の IPA 転写
    pub name_roman_ipa: Option<String>,
    pub name_tts_segments: Vec<TtsSegment>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Line {
    pub id: u32,
    pub name_short: String,
    pub name_katakana: String,
    pub name_full: String,
    pub name_roman: Option<String>,
    pub name_chinese: Option<String>,
    pub name_korean: Option<String>,
    pub color: String,
    /// [`LineType`]
    pub line_type: i32,
    pub line_symbols: Vec<LineSymbol>,
    /// [`OperationStatus`]
    pub status: i32,
    pub station: Option<Box<Station>>,
    pub company: Option<Company>,
    /// 同一種別の問い合わせのときだけ入る。
    pub train_type: Option<Box<TrainType>>,
    pub average_distance: f64,
    /// [`TransportType`]
    pub transport_type: i32,
    pub name_ipa: Option<String>,
    pub name_roman_ipa: Option<String>,
    pub name_tts_segments: Vec<TtsSegment>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Station {
    pub id: u32,
    pub group_id: u32,
    pub name: String,
    pub name_katakana: String,
    pub name_roman: Option<String>,
    pub name_chinese: Option<String>,
    pub name_korean: Option<String>,
    pub three_letter_code: Option<String>,
    pub lines: Vec<Line>,
    pub line: Option<Box<Line>>,
    pub prefecture_id: u32,
    pub postal_code: String,
    pub address: String,
    pub latitude: f64,
    pub longitude: f64,
    pub opened_at: String,
    pub closed_at: String,
    /// [`OperationStatus`]
    pub status: i32,
    pub station_numbers: Vec<StationNumber>,
    /// [`StopCondition`]
    pub stop_condition: i32,
    pub distance: Option<f64>,
    pub has_train_types: Option<bool>,
    pub train_type: Option<Box<TrainType>>,
    /// [`TransportType`]
    pub transport_type: i32,
    pub name_ipa: Option<String>,
    pub name_roman_ipa: Option<String>,
    pub name_tts_segments: Vec<TtsSegment>,
}

/// 経路 1 本ぶんの停車駅。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Route {
    pub id: u32,
    pub stops: Vec<Station>,
}

/// 走行シミュレーション用の 1 区間。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrainRouteSegment {
    /// 緯度経度・stop_condition・train_type・line(line_type) を含む。
    pub station: Option<Station>,
    /// この駅に停車するか。通過駅 (`StopCondition::Not`) は false。
    pub stops: bool,
    /// 直前駅からの距離 (メートル, Haversine)。先頭は 0。
    pub distance_from_previous: f64,
    /// この駅へ向かう区間の最高巡航速度 (m/s)。正値。
    pub max_speed: f64,
    /// 最大加速度 (m/s^2)。正値。
    pub max_acceleration: f64,
    /// 最大減速度 (m/s^2)。正値 (絶対値として持つ)。
    pub max_deceleration: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip() {
        assert_eq!(
            TrainTypeKind::try_from(4),
            Ok(TrainTypeKind::LimitedExpress)
        );
        assert_eq!(LineType::try_from(5), Ok(LineType::MonorailOrAgt));
        assert_eq!(StopCondition::try_from(1), Ok(StopCondition::Not));
        assert_eq!(TrainTypeKind::BusRoute as i32, 7);
    }

    #[test]
    fn unknown_codes_are_rejected_rather_than_defaulted() {
        // 既定値へ倒すのは表示側の役目。ここで黙って 0 にすると、
        // データ不整合が種別「普通」として紛れ込む。
        assert!(TrainTypeKind::try_from(99).is_err());
        assert!(LineType::try_from(-1).is_err());
    }

    #[test]
    fn defaults_match_the_zero_variant() {
        assert_eq!(StopCondition::default(), StopCondition::All);
        assert_eq!(
            TransportType::default(),
            TransportType::TransportTypeUnspecified
        );
        assert_eq!(CompanyType::default(), CompanyType::OtherCompany);
    }
}
