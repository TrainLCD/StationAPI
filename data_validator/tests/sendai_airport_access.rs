use std::path::Path;

use csv::StringRecord;

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

// All Sendai Airport Access rapid trains became local on 2026-03-14.
// https://www.senat.co.jp/2025/12/12/4483
#[test]
fn abolished_sendai_airport_rapid_is_not_selectable() {
    let rows = records("5!station_station_types.csv");
    assert!(rows.iter().all(|r| &r[3] != "958"));

    // Also catch a reintroduced rapid under a different group ID. These are
    // the airport line's own station IDs, not the shared Tohoku Main Line.
    let airport_stations = ["9921701", "9921702", "9921703", "9921704"];
    assert!(rows
        .iter()
        .filter(|r| airport_stations.contains(&&r[1]))
        .all(|r| &r[2] != "102"));
}

#[test]
fn sendai_airport_local_keeps_all_stops_and_both_natori_ids() {
    let rows = records("5!station_station_types.csv");
    let local: Vec<_> = rows.iter().filter(|r| &r[3] == "526").collect();
    let station_ids: Vec<_> = local.iter().map(|r| &r[1]).collect();
    assert_eq!(
        station_ids,
        [
            "9921704", "9921703", "9921702", "9921701", "1123139", "1123140", "1123141", "1123142",
            "1123143",
        ]
    );
    assert!(local.iter().all(|r| &r[2] == "100" && &r[4] == "0"));
}

#[test]
fn shared_rapid_type_remains_available_for_other_lines() {
    let types = records("4!types.csv");
    assert!(types.iter().any(|r| &r[1] == "102" && &r[2] == "快速"));
    let rows = records("5!station_station_types.csv");
    assert!(rows.iter().any(|r| &r[2] == "102" && &r[3] != "958"));
}
