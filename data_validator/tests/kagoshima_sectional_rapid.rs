use std::{collections::BTreeMap, path::Path};

use csv::StringRecord;

fn records(path: &Path) -> Vec<StringRecord> {
    csv::Reader::from_path(path)
        .unwrap()
        .records()
        .collect::<Result<_, _>>()
        .unwrap()
}

#[test]
fn kagoshima_sectional_rapid_matches_official_train_stops() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let stations = records(&root.join("data/3!stations.csv"));
    let names: BTreeMap<_, _> = stations.iter().map(|r| (&r[0], &r[2])).collect();
    let rows = records(&root.join("data/5!station_station_types.csv"));
    let fixture =
        records(&root.join("data_validator/tests/fixtures/kagoshima_sectional_rapid.csv"));
    let mut trains: BTreeMap<(&str, &str), Vec<(&str, &str)>> = BTreeMap::new();
    for row in &fixture {
        trains
            .entry((&row[0], &row[1]))
            .or_default()
            .push((&row[2], &row[3]));
    }
    assert_eq!(trains.len(), 11);
    for ((group, train), expected) in trains {
        let mut actual: Vec<_> = rows
            .iter()
            .filter(|r| &r[3] == group)
            .map(|r| (names[&r[1]], &r[4]))
            .collect();
        // Hakata has two line-specific IDs but represents one physical stop.
        actual.dedup();
        let from = actual.iter().position(|r| r.0 == expected[0].0).unwrap();
        let to = actual
            .iter()
            .position(|r| r.0 == expected.last().unwrap().0)
            .unwrap();
        let actual = if from <= to {
            actual[from..=to].to_vec()
        } else {
            actual[to..=from].iter().rev().copied().collect()
        };
        assert_eq!(actual, expected, "group {group}, train {train}");
    }
}

#[test]
fn kagoshima_sectional_rapid_groups_are_complete_and_selectable_at_hakata() {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../data");
    let rows = records(&data.join("5!station_station_types.csv"));
    let types = records(&data.join("4!types.csv"));
    let rapid: Vec<_> = rows.iter().filter(|r| &r[3] == "259").collect();
    for (group, type_id, first, last, direction) in [
        (1189, 545, "1190202", "1190231", "1"),
        (1190, 546, "1190202", "1190329", "0"),
        (1191, 547, "1190202", "1190329", "0"),
        (1192, 548, "1190202", "1190329", "0"),
        (1193, 549, "1190202", "1190329", "1"),
        (1194, 550, "1190202", "1190321", "1"),
        (1195, 551, "1190205", "1190321", "1"),
    ] {
        let group = group.to_string();
        let type_id = type_id.to_string();
        let ty: Vec<_> = types.iter().filter(|r| r[1] == type_id).collect();
        assert_eq!(ty.len(), 1);
        assert!(ty[0][2].starts_with("区間快速("));
        assert_eq!(&ty[0][8], direction);
        assert_eq!(&ty[0][9], "2");
        let actual: Vec<_> = rows.iter().filter(|r| r[3] == group).collect();
        assert!(actual.iter().all(|r| r[2] == type_id));
        assert!(actual.iter().all(|r| matches!(&r[4], "0" | "1")));
        let from = rapid.iter().position(|r| &r[1] == first).unwrap();
        let to = rapid.iter().position(|r| &r[1] == last).unwrap();
        assert_eq!(
            actual.iter().map(|r| &r[1]).collect::<Vec<_>>(),
            rapid[from..=to].iter().map(|r| &r[1]).collect::<Vec<_>>()
        );
        for hakata in ["1190231", "1190301"] {
            let count = actual
                .iter()
                .filter(|r| &r[1] == hakata && &r[4] == "0")
                .count();
            assert_eq!(count, usize::from(group != "1189" || hakata == "1190231"));
        }
    }
}
