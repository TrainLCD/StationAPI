use std::path::Path;

use csv::StringRecord;
use stationapi::domain::service_rules::{parse_stop_rules, service_rule};

fn records(filename: &str) -> Vec<StringRecord> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../data")
        .join(filename);
    csv::Reader::from_path(path)
        .unwrap()
        .records()
        .collect::<Result<_, _>>()
        .unwrap()
}

// Official sources and effective dates: docs/route-service-audit.md.
#[test]
fn evening_wing_keeps_the_complete_downbound_pattern_with_new_boarding_stops() {
    let rows = records("5!station_station_types.csv");
    let wing: Vec<_> = rows.iter().filter(|r| &r[3] == "81").collect();
    let expected: Vec<u32> = (2700102..=2700147).chain(2700501..=2700509).collect();
    assert_eq!(
        wing.iter()
            .map(|r| r[1].parse().unwrap())
            .collect::<Vec<u32>>(),
        expected
    );
    let stops = [
        2700102, 2700112, 2700115, 2700126, 2700133, 2700138, 2700139, 2700145, 2700147, 2700501,
        2700502, 2700503, 2700504, 2700505, 2700506, 2700507, 2700508, 2700509,
    ];
    for row in wing {
        assert_eq!(&row[2], "403");
        assert_eq!(
            row[4].parse::<u32>().unwrap(),
            u32::from(!stops.contains(&row[1].parse().unwrap()))
        );
    }
    let types = records("4!types.csv");
    let ty = types.iter().find(|r| &r[1] == "403").unwrap();
    assert_eq!(&ty[8], "2");
    assert!(
        service_rule(Some(403), Some(81))
            .unwrap()
            .requires_timetable
    );

    // Morning Wing still passes these stations: do not alter the other service.
    for station in ["2700112", "2700115", "2700126"] {
        assert!(rows
            .iter()
            .any(|r| &r[1] == station && &r[2] == "402" && &r[3] == "80" && &r[4] == "1"));
    }
}

#[test]
fn evening_wing_added_stops_are_boarding_only_in_stored_downbound_direction() {
    let rules = parse_stop_rules(include_str!("../../data/route_stop_rules.csv")).unwrap();
    let stations = records("3!stations.csv");
    for (station, group) in [(2700112, 2700112), (2700115, 1130104), (2700126, 1130105)] {
        let metadata = stations
            .iter()
            .find(|r| r[0] == station.to_string())
            .unwrap();
        assert_eq!(metadata[1].parse::<i32>().unwrap(), group);
        let matches: Vec<_> = rules
            .iter()
            .filter(|r| r.type_cd == 403 && r.station_group_id == group)
            .collect();
        assert_eq!(matches.len(), 1);
        let r = matches[0];
        assert!(r.forward_board);
        assert!(!r.forward_alight && !r.reverse_board && !r.reverse_alight);
        assert!(!rules
            .iter()
            .any(|r| r.type_cd == 402 && r.station_group_id == group));
    }
}

#[test]
fn abolished_tojo_rapid_is_removed_without_removing_shared_seibu_rapid() {
    let rows = records("5!station_station_types.csv");
    assert!(rows.iter().all(|r| &r[3] != "238"));
    assert!(rows
        .iter()
        .filter(|r| r[1].starts_with("21001"))
        .all(|r| &r[2] != "379"));
    let types = records("4!types.csv");
    assert!(types.iter().any(|r| &r[1] == "379" && &r[2] == "快速"));
    for group in ["184", "188", "620"] {
        assert!(rows.iter().any(|r| &r[2] == "379" && &r[3] == group));
    }
    for group in ["236", "237", "239", "240", "241", "242"] {
        assert!(rows.iter().any(|r| &r[3] == group));
    }
}
