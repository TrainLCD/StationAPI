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

const LIMITED_EXPRESS_KIND_MAX_SPEED: f64 = 36.111_111_111_1; // 130 km/h

pub fn resolve_speed_profile(
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

    let max_speed = match kind.and_then(|v| TrainTypeKind::try_from(v).ok()) {
        Some(TrainTypeKind::LimitedExpress) | Some(TrainTypeKind::HighSpeedRapid) => {
            LIMITED_EXPRESS_KIND_MAX_SPEED
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
        let p = resolve_speed_profile(Some(LineType::BulletTrain as i32), true, None);
        assert_eq!(p.max_speed, BUS_MAX_SPEED);
        assert_eq!(p.max_acceleration, BUS_MAX_ACCEL);
        assert_eq!(p.max_deceleration, BUS_MAX_DECEL);
    }

    #[test]
    fn bullet_train_profile() {
        let p = resolve_speed_profile(Some(LineType::BulletTrain as i32), false, None);
        assert_eq!(p.max_speed, BULLET_TRAIN_MAX_SPEED);
        assert_eq!(p.max_acceleration, 0.72);
        assert_eq!(p.max_deceleration, 0.56);
    }

    #[test]
    fn subway_has_stronger_deceleration() {
        let p = resolve_speed_profile(Some(LineType::Subway as i32), false, None);
        assert_eq!(p.max_speed, SUBWAY_MAX_SPEED);
        assert_eq!(p.max_deceleration, 0.83);
    }

    #[test]
    fn unknown_line_type_falls_back_to_normal() {
        let p = resolve_speed_profile(None, false, None);
        assert_eq!(p.max_speed, NORMAL_MAX_SPEED);
        assert_eq!(p.max_acceleration, 0.83);
        assert_eq!(p.max_deceleration, 0.69);
    }

    #[test]
    fn limited_express_kind_caps_speed_over_line_type() {
        let p = resolve_speed_profile(
            Some(LineType::Normal as i32),
            false,
            Some(TrainTypeKind::LimitedExpress as i32),
        );
        assert_eq!(p.max_speed, LIMITED_EXPRESS_KIND_MAX_SPEED);
        assert_eq!(p.max_acceleration, 0.83);
        assert_eq!(p.max_deceleration, 0.69);
    }

    #[test]
    fn high_speed_rapid_kind_caps_speed() {
        let p = resolve_speed_profile(
            Some(LineType::Normal as i32),
            false,
            Some(TrainTypeKind::HighSpeedRapid as i32),
        );
        assert_eq!(p.max_speed, LIMITED_EXPRESS_KIND_MAX_SPEED);
    }

    #[test]
    fn default_kind_keeps_line_type_speed() {
        let p = resolve_speed_profile(
            Some(LineType::Normal as i32),
            false,
            Some(TrainTypeKind::Default as i32),
        );
        assert_eq!(p.max_speed, NORMAL_MAX_SPEED);
    }
}
