use std::path::Path;

use csv::StringRecord;
use stationapi::domain::service_rules::service_rule;

fn records(filename: &str) -> Vec<StringRecord> {
    csv::Reader::from_path(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../data")
            .join(filename),
    )
    .unwrap()
    .records()
    .collect::<Result<_, _>>()
    .unwrap()
}

#[test]
fn extra_limited_express_matches_the_published_stops_in_each_direction() {
    let rows = records("5!station_station_types.csv");
    let northbound: Vec<u32> = (3102701..=3102744)
        .chain(3100901..=3100914)
        .chain(3101001..=3101005)
        .collect();
    let southbound: Vec<u32> = (3102701..=3102744).chain(3100901..=3100904).collect();
    let northbound_stops = [
        3101005, 3101002, 3101001, 3100914, 3100913, 3100904, 3100901, 3102744, 3102739, 3102731,
        3102721, 3102713, 3102701,
    ];
    let southbound_stops = [
        3102701, 3102713, 3102721, 3102731, 3102739, 3102744, 3100901, 3100904,
    ];
    for (type_id, group_id, stations, stops) in [
        ("565", "1210", northbound, northbound_stops.as_slice()),
        ("566", "1211", southbound, southbound_stops.as_slice()),
    ] {
        let pattern: Vec<_> = rows.iter().filter(|row| &row[3] == group_id).collect();
        let actual: Vec<u32> = pattern.iter().map(|row| row[1].parse().unwrap()).collect();
        assert_eq!(actual, stations);
        for (station, row) in actual.into_iter().zip(pattern) {
            assert_eq!(&row[2], type_id);
            assert_eq!(
                row[4].parse::<u32>().unwrap(),
                u32::from(!stops.contains(&station))
            );
        }
    }
}

#[test]
fn timetable_rules_are_scoped_and_regular_hisai_stop_is_preserved() {
    let types = records("4!types.csv");
    for (type_id, group_id, direction) in [(565, 1210, "1"), (566, 1211, "2")] {
        let id = type_id.to_string();
        let matching_types: Vec<_> = types.iter().filter(|row| &row[1] == id.as_str()).collect();
        assert_eq!(matching_types.len(), 1);
        assert_eq!(&matching_types[0][8], direction);
        assert!(matching_types[0][4].starts_with("Special Limited Express (for "));
        assert_eq!(&matching_types[0][9], "4");
        assert_eq!(&matching_types[0][10], "0");
        assert!(
            service_rule(Some(type_id), Some(group_id))
                .unwrap()
                .requires_timetable
        );
        assert!(service_rule(Some(type_id), Some(106)).is_none());
    }
    assert!(service_rule(Some(306), Some(106)).is_none());
    let rows = records("5!station_station_types.csv");
    let regular: Vec<_> = rows.iter().filter(|row| &row[3] == "106").collect();
    assert_eq!(regular.len(), 79);
    assert!(regular.iter().all(|row| &row[2] == "306"));
    let hisai: Vec<_> = regular.iter().filter(|row| &row[1] == "3102742").collect();
    assert_eq!(hisai.len(), 1);
    assert_eq!(&hisai[0][4], "0");
}
