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
fn koedo_tanashi_is_only_a_holiday_pattern_stop() {
    const ORDINARY_STOPS: [u32; 6] = [2200701, 2200702, 2200721, 2200722, 2200726, 2200729];
    const TANASHI: u32 = 2200717;

    let rows = records("5!station_station_types.csv");
    for (type_cd, group_id, stops_at_tanashi) in [("504", "215", false), ("564", "1209", true)] {
        let group: Vec<_> = rows.iter().filter(|row| &row[3] == group_id).collect();
        assert_eq!(group.len(), 29);
        for (row, station_cd) in group.into_iter().zip(2200701..=2200729) {
            assert_eq!(row[1].parse::<u32>().unwrap(), station_cd);
            assert_eq!(&row[2], type_cd);
            let stops =
                ORDINARY_STOPS.contains(&station_cd) || (stops_at_tanashi && station_cd == TANASHI);
            assert_eq!(row[4].parse::<u32>().unwrap(), u32::from(!stops));
        }
    }
}

#[test]
fn koedo_tanashi_excludes_only_the_bidirectional_special_pattern() {
    let rows = records("4!types.csv");
    let types: Vec<_> = rows.iter().filter(|row| &row[1] == "564").collect();
    assert_eq!(types.len(), 1);
    assert_eq!(&types[0][2], "小江戸号(土休日ダイヤ・田無停車)");
    assert_eq!(&types[0][8], "0");
    assert_eq!(&types[0][9], "4");

    assert!(
        service_rule(Some(564), Some(1209))
            .unwrap()
            .requires_timetable
    );
    assert!(service_rule(Some(564), None).is_none());
    assert!(service_rule(Some(504), Some(215)).is_none());
}
