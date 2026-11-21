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

const LOCAL_STATIONS: [u32; 10] = [
    9991901, 9991902, 9991903, 9991904, 9991905, 9991906, 9991907, 9991910, 9991908, 9991909,
];

fn check_pattern(rows: &[StringRecord], group: &str, ty: &str, expected: &[u32]) {
    let actual: Vec<_> = rows.iter().filter(|row| &row[3] == group).collect();
    assert_eq!(actual.len(), expected.len());
    for (row, expected_id) in actual.into_iter().zip(expected) {
        assert_eq!(row[1].parse::<u32>().unwrap(), *expected_id);
        assert_eq!(&row[2], ty);
        assert_eq!(&row[4], "0");
    }
}

#[test]
fn minamiaso_through_covers_both_tateno_ids_and_preserves_line_local() {
    let rows = records("5!station_station_types.csv");
    let through: Vec<_> = [1192312, 1192313, 1192314]
        .into_iter()
        .chain(LOCAL_STATIONS)
        .collect();
    check_pattern(&rows, "1212", "567", &through);
    check_pattern(&rows, "1000099919", "100", &LOCAL_STATIONS);

    let stations = records("3!stations.csv");
    for (station_cd, line_cd) in [("1192314", "11923"), ("9991901", "99919")] {
        let row = stations.iter().find(|row| &row[0] == station_cd).unwrap();
        assert_eq!(&row[1], "1192314");
        assert_eq!(&row[13], line_cd);
    }
}

#[test]
fn only_the_low_priority_bidirectional_through_service_requires_the_timetable() {
    let types = records("4!types.csv");
    let rows: Vec<_> = types.iter().filter(|row| &row[1] == "567").collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(&rows[0][8], "0");
    assert_eq!(&rows[0][9], "0");
    assert_eq!(&rows[0][10], "0");
    let local = types.iter().find(|row| &row[1] == "100").unwrap();
    assert!(local[10].parse::<u32>().unwrap() > 0);

    assert!(
        service_rule(Some(567), Some(1212))
            .unwrap()
            .requires_timetable
    );
    assert!(service_rule(Some(567), None).is_none());
    assert!(service_rule(Some(100), Some(1000099919)).is_none());
    assert!(service_rule(Some(100), Some(867)).is_none());
}
