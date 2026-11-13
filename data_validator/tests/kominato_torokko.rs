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

// Official source and applicability: docs/route-service-audit.md.
#[test]
fn torokko_keeps_both_directions_and_only_three_stops_to_takataki() {
    let types = records("4!types.csv");
    let types: Vec<_> = types.iter().filter(|r| &r[1] == "559").collect();
    assert_eq!(types.len(), 1);
    assert_eq!(&types[0][2], "房総里山トロッコ");
    assert_eq!(&types[0][8], "0");
    assert_eq!(&types[0][9], "2");
    let rows = records("5!station_station_types.csv");
    let group: Vec<_> = rows.iter().filter(|r| &r[3] == "1205").collect();
    let actual: Vec<u32> = group.iter().map(|r| r[1].parse().unwrap()).collect();
    assert_eq!(actual, (9932501..=9932512).collect::<Vec<_>>());
    for row in group {
        assert_eq!(&row[2], "559");
        let station = row[1].parse::<u32>().unwrap();
        assert_eq!(
            row[4].parse::<u32>().unwrap(),
            u32::from(![9932501, 9932508, 9932512].contains(&station))
        );
    }
    assert!(rows
        .iter()
        .filter(|r| &r[2] == "559")
        .all(|r| &r[3] == "1205"));
    let rule = service_rule(Some(559), Some(1205)).unwrap();
    assert!(rule.requires_timetable);
    assert_eq!(rule.boarding_wait_seconds, None);
    assert!(service_rule(Some(100), Some(1000099325)).is_none());
}
