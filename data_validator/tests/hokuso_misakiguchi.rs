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
fn holiday_836t_keeps_all_local_stops_and_changes_to_tokkyu_at_sengakuji() {
    let rows = records("5!station_station_types.csv");
    let route: Vec<_> = rows.iter().filter(|r| &r[3] == "1207").collect();
    let expected: Vec<u32> = (9934001..=9934014)
        .rev()
        .chain([2300110, 2300207, 2300206, 2300109])
        .chain((2300201..=2300205).rev())
        .chain((9930207..=9930220).rev())
        .chain(2700101..=2700147)
        .chain(2700501..=2700509)
        .collect();
    assert_eq!(
        route
            .iter()
            .map(|r| r[1].parse().unwrap())
            .collect::<Vec<u32>>(),
        expected
    );
    // Official 836T column: mainline_holiday2026.pdf, printed page 81.
    let express_stops = [
        2700101, 2700102, 2700105, 2700109, 2700112, 2700115, 2700123, 2700126, 2700133, 2700138,
        2700139, 2700140, 2700144, 2700145, 2700147,
    ];
    for row in route {
        let station: u32 = row[1].parse().unwrap();
        let keikyu =
            (2700101..=2700147).contains(&station) || (2700501..=2700509).contains(&station);
        assert_eq!(&row[2], if keikyu { "562" } else { "561" });
        let stops =
            !keikyu || express_stops.contains(&station) || (2700501..=2700509).contains(&station);
        assert_eq!(row[4].parse::<u32>().unwrap(), u32::from(!stops));
    }
}

#[test]
fn dedicated_types_are_forward_only_and_excluded_from_timetable_free_search() {
    let types = records("4!types.csv");
    for (id, name, kind) in [
        (561, "普通(三崎口方面)", "0"),
        (562, "特急(三崎口方面)", "3"),
    ] {
        let matching: Vec<_> = types.iter().filter(|r| r[1] == id.to_string()).collect();
        assert_eq!(matching.len(), 1);
        let ty = matching[0];
        assert_eq!(&ty[2], name);
        assert_eq!(&ty[8], "2");
        assert_eq!(&ty[9], kind);
        assert_eq!(&ty[10], "0");
        assert!(
            service_rule(Some(id), Some(1207))
                .unwrap()
                .requires_timetable
        );
        assert!(service_rule(Some(id), None).is_none());
    }
    let rows = records("5!station_station_types.csv");
    assert!(rows
        .iter()
        .filter(|r| ["561", "562"].contains(&&r[2]))
        .all(|r| &r[3] == "1207"));
    // Existing shared services remain available; group-specific new rules must not affect them.
    for (id, group) in [(355, 426), (306, 1172), (312, 1172), (312, 1173)] {
        assert!(service_rule(Some(id), Some(group)).is_none());
    }
    for (group, count, first, last) in [
        ("426", 42, "9930201", "9934015"),
        ("1172", 93, "2700509", "9934015"),
        ("1173", 95, "2300611", "2700509"),
    ] {
        let existing: Vec<_> = rows.iter().filter(|r| &r[3] == group).collect();
        assert_eq!(existing.len(), count);
        assert_eq!(&existing.first().unwrap()[1], first);
        assert_eq!(&existing.last().unwrap()[1], last);
    }
    assert!(rows
        .iter()
        .any(|r| &r[3] == "1172" && &r[1] == "2300205" && &r[4] == "1"));
    assert!(rows
        .iter()
        .any(|r| &r[3] == "1173" && &r[1] == "2300627" && &r[4] == "1"));
}
