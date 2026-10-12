//! Explicit operating constraints, independent of route-ranking weights.
use super::entity::station::Station;
use serde::Deserialize;
use std::{collections::HashMap, sync::OnceLock};
#[derive(Clone, Debug, Deserialize)]
pub struct ServiceRule {
    pub type_cd: i32,
    pub line_group_id: Option<i32>,
    pub requires_timetable: bool,
    pub boarding_wait_seconds: Option<i32>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct StopRule {
    pub type_cd: i32,
    pub station_group_id: i32,
    pub forward_board: bool,
    pub forward_alight: bool,
    pub reverse_board: bool,
    pub reverse_alight: bool,
}
/// Parse canonical service rules.
pub fn parse_service_rules(text: &str) -> Result<Vec<ServiceRule>, csv::Error> {
    csv::Reader::from_reader(text.as_bytes())
        .deserialize()
        .collect()
}
/// Parse canonical directional stop rules.
pub fn parse_stop_rules(text: &str) -> Result<Vec<StopRule>, csv::Error> {
    csv::Reader::from_reader(text.as_bytes())
        .deserialize()
        .collect()
}
/// Group-specific rules take precedence over type-wide rules.
pub fn service_rule(type_cd: Option<i32>, group: Option<i32>) -> Option<&'static ServiceRule> {
    type Rules = HashMap<(i32, Option<i32>), ServiceRule>;
    static RULES: OnceLock<Rules> = OnceLock::new();
    let rules = RULES.get_or_init(|| {
        parse_service_rules(include_str!("../../../data/route_service_rules.csv"))
            .expect("service rules")
            .into_iter()
            .map(|r| ((r.type_cd, r.line_group_id), r))
            .collect()
    });
    let type_cd = type_cd?;
    rules
        .get(&(type_cd, group))
        .or_else(|| rules.get(&(type_cd, None)))
}
fn stop_rule(station: &Station) -> Option<&'static StopRule> {
    static RULES: OnceLock<HashMap<(i32, i32), StopRule>> = OnceLock::new();
    RULES
        .get_or_init(|| {
            parse_stop_rules(include_str!("../../../data/route_stop_rules.csv"))
                .expect("stop rules")
                .into_iter()
                .map(|r| ((r.type_cd, r.station_group_id), r))
                .collect()
        })
        .get(&(station.type_cd?, station.station_g_cd))
}
/// Pickup permission in stored or reverse service direction.
pub fn board_allowed(station: &Station, forward: bool) -> bool {
    stop_rule(station).is_none_or(|r| {
        if forward {
            r.forward_board
        } else {
            r.reverse_board
        }
    })
}
/// Drop-off permission in stored or reverse service direction.
pub fn alight_allowed(station: &Station, forward: bool) -> bool {
    stop_rule(station).is_none_or(|r| {
        if forward {
            r.forward_alight
        } else {
            r.reverse_alight
        }
    })
}
/// Wait at the actual boarding station.
pub fn boarding_wait(station: &Station) -> i32 {
    service_rule(station.type_cd, station.line_group_cd)
        .and_then(|r| r.boarding_wait_seconds)
        .unwrap_or_else(|| super::route_search::boarding_wait_seconds(station.kind))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn admission_is_specific_to_the_declared_service() {
        assert!(
            service_rule(Some(316), Some(406))
                .unwrap()
                .requires_timetable
        );
        assert!(
            service_rule(Some(379), Some(188))
                .unwrap()
                .requires_timetable
        );
        assert!(service_rule(Some(379), Some(184)).is_none());
        let rule = parse_stop_rules(include_str!("../../../data/route_stop_rules.csv"))
            .unwrap()
            .remove(0);
        assert!(!rule.forward_board && rule.forward_alight);
        assert!(!rule.reverse_board && !rule.reverse_alight);
    }
}
