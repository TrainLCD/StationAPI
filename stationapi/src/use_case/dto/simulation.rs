use crate::domain::speed_table::line_speed_override_kmh;
use crate::proto::{LineType, TrainTypeKind};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpeedProfile {
    pub max_speed: f64,
    pub max_acceleration: f64,
    pub max_deceleration: f64,
}

const BUS_MAX_SPEED: f64 = 11.111_111_111_11; // 40 km/h
const BUS_MAX_ACCEL: f64 = 1.2;
const BUS_MAX_DECEL: f64 = 1.3;

const BULLET_TRAIN_MAX_SPEED: f64 = 88.888_888_89; // 320 km/h
const MONORAIL_OR_AGT_MAX_SPEED: f64 = 22.222_222_222_22; // 80 km/h
const SUBWAY_MAX_SPEED: f64 = 22.222_222_222_22; // 80 km/h
const NORMAL_MAX_SPEED: f64 = 25.0; // 90 km/h
const TRAM_MAX_SPEED: f64 = 11.111_111_111_11; // 40 km/h

const LIMITED_EXPRESS_KIND_FLOOR_SPEED: f64 = 36.111_111_111_1; // 130 km/h

pub fn resolve_speed_profile(
    line_cd: i32,
    line_type: Option<i32>,
    is_bus: bool,
    kind: Option<i32>,
) -> SpeedProfile {
    if is_bus {
        return SpeedProfile {
            max_speed: BUS_MAX_SPEED,
            max_acceleration: BUS_MAX_ACCEL,
            max_deceleration: BUS_MAX_DECEL,
        };
    }

    let (line_max_speed, accel, decel) = match line_type.and_then(|v| LineType::try_from(v).ok()) {
        Some(LineType::BulletTrain) => (BULLET_TRAIN_MAX_SPEED, 0.72, 0.56),
        Some(LineType::Subway) => (SUBWAY_MAX_SPEED, 0.83, 0.83),
        Some(LineType::MonorailOrAgt) => (MONORAIL_OR_AGT_MAX_SPEED, 0.97, 0.69),
        Some(LineType::Tram) => (TRAM_MAX_SPEED, 0.83, 0.69),
        _ => (NORMAL_MAX_SPEED, 0.83, 0.69),
    };

    // 路線 × 種別の較正テーブル(実路線の時刻表と公表運転速度に基づく実効値)が
    // あれば最優先で使う。スカイライナー 160km/h のような路線種別・種別の
    // 一般則では表現できない実勢速度をここで反映する。
    if let Some(v_kmh) = line_speed_override_kmh(line_cd, kind) {
        return SpeedProfile {
            max_speed: v_kmh / 3.6,
            max_acceleration: accel,
            max_deceleration: decel,
        };
    }

    // 種別による 130km/h は在来線の速達種別を底上げするための下限値。
    // 新幹線(のぞみ等は kind=LimitedExpress)では路線種別の上限の方が速いため、
    // 種別値で引き下げない。
    let max_speed = match kind.and_then(|v| TrainTypeKind::try_from(v).ok()) {
        Some(TrainTypeKind::LimitedExpress) | Some(TrainTypeKind::HighSpeedRapid) => {
            line_max_speed.max(LIMITED_EXPRESS_KIND_FLOOR_SPEED)
        }
        _ => line_max_speed,
    };

    SpeedProfile {
        max_speed,
        max_acceleration: accel,
        max_deceleration: decel,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bus_overrides_line_type() {
        let p = resolve_speed_profile(0, Some(LineType::BulletTrain as i32), true, None);
        assert_eq!(p.max_speed, BUS_MAX_SPEED);
        assert_eq!(p.max_acceleration, BUS_MAX_ACCEL);
        assert_eq!(p.max_deceleration, BUS_MAX_DECEL);
    }

    #[test]
    fn bullet_train_profile() {
        let p = resolve_speed_profile(0, Some(LineType::BulletTrain as i32), false, None);
        assert_eq!(p.max_speed, BULLET_TRAIN_MAX_SPEED);
        assert_eq!(p.max_acceleration, 0.72);
        assert_eq!(p.max_deceleration, 0.56);
    }

    #[test]
    fn subway_has_stronger_deceleration() {
        let p = resolve_speed_profile(0, Some(LineType::Subway as i32), false, None);
        assert_eq!(p.max_speed, SUBWAY_MAX_SPEED);
        assert_eq!(p.max_deceleration, 0.83);
    }

    #[test]
    fn unknown_line_type_falls_back_to_normal() {
        let p = resolve_speed_profile(0, None, false, None);
        assert_eq!(p.max_speed, NORMAL_MAX_SPEED);
        assert_eq!(p.max_acceleration, 0.83);
        assert_eq!(p.max_deceleration, 0.69);
    }

    #[test]
    fn limited_express_kind_raises_speed_over_line_type() {
        let p = resolve_speed_profile(
            0,
            Some(LineType::Normal as i32),
            false,
            Some(TrainTypeKind::LimitedExpress as i32),
        );
        assert_eq!(p.max_speed, LIMITED_EXPRESS_KIND_FLOOR_SPEED);
        assert_eq!(p.max_acceleration, 0.83);
        assert_eq!(p.max_deceleration, 0.69);
    }

    #[test]
    fn limited_express_kind_does_not_slow_down_bullet_train() {
        // のぞみ等は kind=LimitedExpress で登録されているが、
        // 新幹線の路線上限(320km/h)を 130km/h に引き下げてはいけない。
        let p = resolve_speed_profile(
            0,
            Some(LineType::BulletTrain as i32),
            false,
            Some(TrainTypeKind::LimitedExpress as i32),
        );
        assert_eq!(p.max_speed, BULLET_TRAIN_MAX_SPEED);
        assert_eq!(p.max_acceleration, 0.72);
        assert_eq!(p.max_deceleration, 0.56);
    }

    #[test]
    fn high_speed_rapid_kind_raises_speed() {
        let p = resolve_speed_profile(
            0,
            Some(LineType::Normal as i32),
            false,
            Some(TrainTypeKind::HighSpeedRapid as i32),
        );
        assert_eq!(p.max_speed, LIMITED_EXPRESS_KIND_FLOOR_SPEED);
    }

    #[test]
    fn default_kind_keeps_line_type_speed() {
        let p = resolve_speed_profile(
            0,
            Some(LineType::Normal as i32),
            false,
            Some(TrainTypeKind::Default as i32),
        );
        assert_eq!(p.max_speed, NORMAL_MAX_SPEED);
    }

    #[test]
    fn speed_table_override_takes_precedence() {
        // 成田スカイアクセス線(23006)のスカイライナー(LimitedExpress)は
        // 較正テーブルの 160km/h が種別下限 130km/h より優先される。
        let p = resolve_speed_profile(
            23006,
            Some(LineType::Normal as i32),
            false,
            Some(TrainTypeKind::LimitedExpress as i32),
        );
        assert!((p.max_speed - 160.0 / 3.6).abs() < 1e-9, "{}", p.max_speed);
        assert_eq!(p.max_acceleration, 0.83);
        assert_eq!(p.max_deceleration, 0.69);

        // 東急東横線(26001)の特急は実効 80km/h へ引き下げる(130km/h 下限より優先)。
        let p = resolve_speed_profile(
            26001,
            Some(LineType::Normal as i32),
            false,
            Some(TrainTypeKind::LimitedExpress as i32),
        );
        assert!((p.max_speed - 80.0 / 3.6).abs() < 1e-9, "{}", p.max_speed);

        // テーブルに無い (line, kind) は従来どおり(130km/h 下限)。
        let p = resolve_speed_profile(
            26001,
            Some(LineType::Normal as i32),
            false,
            Some(TrainTypeKind::HighSpeedRapid as i32),
        );
        assert_eq!(p.max_speed, LIMITED_EXPRESS_KIND_FLOOR_SPEED);
    }

    #[test]
    fn bus_ignores_speed_table() {
        // バスは line_cd がテーブルに載る路線でも固定プロファイルのまま。
        let p = resolve_speed_profile(23006, Some(3), true, Some(7));
        assert_eq!(p.max_speed, BUS_MAX_SPEED);
    }
}
