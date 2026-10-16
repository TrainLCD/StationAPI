use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use csv::StringRecord;

fn records(path: &Path) -> Vec<StringRecord> {
    csv::Reader::from_path(path)
        .unwrap()
        .records()
        .collect::<Result<_, _>>()
        .unwrap()
}

// Line-specific station IDs at junctions must appear on both sides, in order.
fn sections(group: u32) -> Vec<(u32, u32, u32)> {
    match group {
        65 | 67..=71 => vec![(24001, 1, 34)],
        66 | 252 | 416 | 760..=762 => vec![(24001, 1, 18), (24002, 18, 45)],
        253 | 412..=415 | 758 | 759 => vec![(24001, 1, 33), (24003, 33, 53)],
        254 | 285..=287 | 757 => {
            vec![(24007, 1, 4), (24001, 4, 18), (24002, 18, 45)]
        }
        255 => vec![(24007, 1, 4), (24001, 4, 33), (24003, 33, 53)],
        756 => vec![(24007, 1, 4), (24001, 4, 34)],
        763 | 764 => vec![(24001, 1, 23), (24004, 23, 46)],
        765 => vec![(24004, 23, 46)],
        766 => vec![(24001, 1, 29), (24005, 29, 47)],
        767 => vec![(24005, 29, 47)],
        _ => panic!("unaudited Keio group: {group}"),
    }
}

#[test]
fn existing_keio_patterns_match_official_stops_and_complete_station_sequences() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let data = root.join("data");
    let stations = records(&data.join("3!stations.csv"));
    let station_by_id: HashMap<&str, &StringRecord> = stations.iter().map(|r| (&r[0], r)).collect();
    let types = records(&data.join("4!types.csv"));
    let type_names: HashMap<&str, &str> = types.iter().map(|r| (&r[1], &r[2])).collect();
    let fixture = records(&root.join("data_validator/tests/fixtures/keio_stops.csv"));
    let official: HashMap<u32, &str> = fixture
        .iter()
        .map(|r| (r[0].parse().unwrap(), &r[1]))
        .collect();
    assert_eq!(official.len(), 53);

    let mut actual: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for row in records(&data.join("5!station_station_types.csv")) {
        let station = station_by_id[&row[1]];
        let line: u32 = station[13].parse().unwrap();
        if !matches!(line, 24001..=24005 | 24007) {
            continue;
        }
        let number: u32 = station[8].parse().unwrap();
        let group: u32 = row[3].parse().unwrap();
        let name = type_names[&row[2]];
        let category = if name.starts_with("京王ライナー") {
            "京王ライナー"
        } else if name == "Mt.TAKAO" {
            "Mt.TAKAO号"
        } else {
            name
        };
        // Per-row type is essential: Toei locals become Keio express services.
        let mut expected = u32::from(!official[&number].split('|').any(|t| t == category));
        if group == 758 {
            // Mt.TAKAO down: Shinjuku, Meidaimae, Takaosanguchi only.
            expected = u32::from(!matches!(number, 1 | 6 | 53));
        }
        if number == 23 && category == "特急" {
            // Racecourse through service stops; main-line services partly stop.
            expected = if group == 764 { 0 } else { 5 };
        }
        assert_eq!(
            row[4].parse::<u32>().unwrap(),
            expected,
            "group {group}, station {} ({}), type {name}",
            &station[2],
            &station[0]
        );
        actual.entry(group).or_default().push(row[1].to_owned());
    }

    let audited: BTreeSet<u32> = [
        65, 66, 67, 68, 69, 70, 71, 252, 253, 254, 255, 285, 286, 287, 412, 413, 414, 415, 416,
        756, 757, 758, 759, 760, 761, 762, 763, 764, 765, 766, 767,
    ]
    .into_iter()
    .collect();
    assert_eq!(actual.keys().copied().collect::<BTreeSet<_>>(), audited);
    for (group, station_ids) in actual {
        let mut expected = Vec::new();
        for (line, first, last) in sections(group) {
            let mut section: Vec<&StringRecord> = stations
                .iter()
                .filter(|r| {
                    r[13].parse::<u32>().unwrap() == line
                        && &r[21] == "0"
                        && (first..=last).contains(&r[8].parse::<u32>().unwrap())
                })
                .collect();
            section.sort_by_key(|r| (r[22].parse::<u32>().unwrap(), r[0].to_owned()));
            expected.extend(section.into_iter().map(|r| r[0].to_owned()));
        }
        assert_eq!(
            station_ids, expected,
            "incomplete/out-of-order group {group}"
        );
    }
}
