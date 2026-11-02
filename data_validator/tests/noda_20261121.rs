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

// Official 2026-11-21 revision: docs/noda-20261121.md.
#[test]
fn funabashi_section_express_keeps_all_twelve_stations_and_six_stops() {
    let rows = records("5!station_station_types.csv");
    let pattern: Vec<_> = rows.iter().filter(|r| &r[3] == "1205").collect();
    assert_eq!(
        pattern
            .iter()
            .map(|r| r[1].parse().unwrap())
            .collect::<Vec<u32>>(),
        (2100424..=2100435).collect::<Vec<_>>()
    );
    let stops = [2100424, 2100428, 2100430, 2100433, 2100434, 2100435];
    for row in pattern {
        assert_eq!(&row[2], "559");
        assert_eq!(
            row[4].parse::<u32>().unwrap(),
            u32::from(!stops.contains(&row[1].parse().unwrap()))
        );
    }
}

#[test]
fn weekday_morning_direction_and_timetable_rule_do_not_affect_shared_type() {
    let types = records("4!types.csv");
    let matches: Vec<_> = types.iter().filter(|r| &r[1] == "559").collect();
    assert_eq!(matches.len(), 1);
    assert_eq!(&matches[0][2], "区間急行(船橋方面)");
    assert_eq!(&matches[0][8], "2");
    assert_eq!(&matches[0][9], "3");
    assert!(
        service_rule(Some(559), Some(1205))
            .unwrap()
            .requires_timetable
    );
    let shared = types.iter().find(|r| &r[1] == "319").unwrap();
    assert_eq!(&shared[8], "0");
    assert!(service_rule(Some(319), Some(333)).is_none());
    let rows = records("5!station_station_types.csv");
    let existing: Vec<_> = rows.iter().filter(|r| &r[3] == "333").collect();
    assert_eq!(existing.len(), 24);
    for (offset, row) in existing.iter().enumerate() {
        assert_eq!(row[1].parse::<usize>().unwrap(), 2100401 + offset);
        assert_eq!(&row[2], "319");
        let station = row[1].parse::<u32>().unwrap();
        let stop = station == 2100401 || station == 2100406 || station >= 2100410;
        assert_eq!(row[4].parse::<u32>().unwrap(), u32::from(!stop));
    }
}
