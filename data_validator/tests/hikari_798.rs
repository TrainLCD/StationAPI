use std::path::Path;

use csv::StringRecord;
use stationapi::domain::service_rules::service_rule;

const HIKARI_798_TYPE: i32 = 568;
const HIKARI_798_GROUP: i32 = 1213;

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

fn station_pattern(rows: &[StringRecord], group: i32) -> Vec<(i32, i32, i32)> {
    rows.iter()
        .filter(|row| row[3].parse::<i32>().unwrap() == group)
        .map(|row| {
            (
                row[1].parse().unwrap(),
                row[2].parse().unwrap(),
                row[4].parse().unwrap(),
            )
        })
        .collect()
}

#[test]
fn hikari_798_has_the_official_hakata_to_okayama_pattern() {
    let types = records("4!types.csv");
    let specific: Vec<_> = types
        .iter()
        .filter(|row| row[1].parse::<i32>().unwrap() == HIKARI_798_TYPE)
        .collect();
    assert_eq!(specific.len(), 1);
    assert_eq!(&specific[0][2], "ひかり798号");
    assert_eq!(&specific[0][8], "1"); // 上り: 格納した岡山→博多の逆順
    assert_eq!(&specific[0][9], "4"); // 新幹線は他のひかりと同じ区分

    let rows = records("5!station_station_types.csv");
    let pattern = station_pattern(&rows, HIKARI_798_GROUP);
    let expected_stations = (100306..=100319).collect::<Vec<_>>();
    let stations = pattern
        .iter()
        .map(|&(station, _, _)| station)
        .collect::<Vec<_>>();
    assert_eq!(stations, expected_stations);
    assert!(pattern.iter().all(|&(_, ty, _)| ty == HIKARI_798_TYPE));
    assert!(pattern.iter().all(|&(_, _, pass)| matches!(pass, 0 | 1)));
    let inbound_stops = pattern
        .iter()
        .rev()
        .filter_map(|&(station, _, pass)| (pass == 0).then_some(station))
        .collect::<Vec<_>>();
    assert_eq!(
        inbound_stops,
        [100319, 100318, 100312, 100310, 100308, 100306]
    );
    assert!(rows
        .iter()
        .filter(|row| row[2].parse::<i32>().unwrap() == HIKARI_798_TYPE)
        .all(|row| row[3].parse::<i32>().unwrap() == HIKARI_798_GROUP));
}

#[test]
fn timetable_exclusion_is_scoped_to_the_special_hikari_pattern() {
    let special_rule = service_rule(Some(HIKARI_798_TYPE), Some(HIKARI_798_GROUP)).unwrap();
    assert!(special_rule.requires_timetable);
    assert_eq!(special_rule.boarding_wait_seconds, None);
    assert!(service_rule(Some(HIKARI_798_TYPE), None).is_none());
    assert!(service_rule(Some(2), Some(2)).is_none());

    let types = records("4!types.csv");
    let normal = types.iter().find(|row| &row[1] == "2").unwrap();
    assert_eq!(&normal[2], "ひかり");
    assert_eq!(&normal[8], "0");
    let rows = records("5!station_station_types.csv");
    let normal_sanyo = station_pattern(&rows, 2)
        .into_iter()
        .filter(|&(station, _, _)| (100301..=100319).contains(&station))
        .collect::<Vec<_>>();
    let expected_pass = [0, 0, 2, 0, 2, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0];
    let expected = (100301..=100319)
        .zip(expected_pass)
        .map(|(station, pass)| (station, 2, pass))
        .collect::<Vec<_>>();
    assert_eq!(normal_sanyo, expected);
}
