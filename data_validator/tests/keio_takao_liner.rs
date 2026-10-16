use std::path::Path;

use csv::StringRecord;
use stationapi::domain::service_rules::service_rule;

fn records(path: &Path) -> Vec<StringRecord> {
    csv::Reader::from_path(path)
        .unwrap()
        .records()
        .collect::<Result<_, _>>()
        .unwrap()
}

#[test]
fn takao_liner_has_its_own_complete_pattern_and_requires_timetable() {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../data");
    let types = records(&data.join("4!types.csv"));
    let liner_types: Vec<_> = types.iter().filter(|r| &r[1] == "413").collect();
    assert_eq!(liner_types.len(), 1);
    let liner = liner_types[0];
    assert_eq!(&liner[2], "京王ライナー(高尾山口方面)");
    assert_eq!(&liner[8], "0"); // Both directions, like the other Keio Liners.
    assert_eq!(&liner[9], "2");

    let all_rows = records(&data.join("5!station_station_types.csv"));
    let rows: Vec<_> = all_rows.iter().filter(|r| &r[3] == "1188").collect();
    let expected_stations: Vec<u32> = std::iter::once(2400101)
        .chain(2400104..=2400133)
        .chain(2400301..=2400307)
        .collect();
    let actual_stations: Vec<u32> = rows.iter().map(|r| r[1].parse().unwrap()).collect();
    assert_eq!(actual_stations, expected_stations);

    // Official up/down stopping lists, checked 2026-10-16:
    // https://www.keio.co.jp/zasekishitei/
    // https://www.keio.co.jp/zasekishitei/index_n.html
    // Both Kitano IDs are intentional; Katakura, Yamada and Hazama pass.
    let stops = [
        2400101, 2400106, 2400124, 2400125, 2400127, 2400129, 2400133, 2400301, 2400304, 2400306,
        2400307,
    ];
    for row in rows {
        let station: u32 = row[1].parse().unwrap();
        assert_eq!(&row[2], "413");
        assert_eq!(
            row[4].parse::<u32>().unwrap(),
            u32::from(!stops.contains(&station))
        );
    }
    assert!(all_rows
        .iter()
        .filter(|r| &r[2] == "413")
        .all(|r| &r[3] == "1188"));
    let rule = service_rule(Some(413), Some(1188)).expect("Takao Liner operating rule");
    assert!(rule.requires_timetable);
    assert_eq!(rule.boarding_wait_seconds, None);
}
