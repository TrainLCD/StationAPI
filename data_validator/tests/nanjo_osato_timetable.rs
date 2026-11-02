use std::path::Path;

use csv::StringRecord;

fn rows() -> Vec<StringRecord> {
    csv::Reader::from_path(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/5!station_station_types.csv"),
    )
    .unwrap()
    .records()
    .collect::<Result<_, _>>()
    .unwrap()
}

// Sources and train numbers: docs/nanjo-osato-timetable.md.
#[test]
fn nanjo_rapid_has_both_stopping_and_passing_trains() {
    let rows = rows();
    let matches: Vec<_> = rows
        .iter()
        .filter(|r| &r[1] == "1141516" && &r[2] == "102" && &r[3] == "964")
        .collect();
    assert_eq!(matches.len(), 1);
    assert_eq!(&matches[0][4], "2");
}

#[test]
fn osato_special_stop_is_scoped_to_the_nagoya_main_line_express() {
    let rows = rows();
    for (group, pass) in [("280", "5"), ("657", "1"), ("1069", "1")] {
        let matches: Vec<_> = rows
            .iter()
            .filter(|r| &r[1] == "3000145" && &r[2] == "345" && &r[3] == group)
            .collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(&matches[0][4], pass);
    }
}
