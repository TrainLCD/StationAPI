use std::{collections::BTreeMap, path::Path};

use csv::StringRecord;
use stationapi::domain::service_rules::{parse_stop_rules, service_rule};

fn records(path: &Path) -> Vec<StringRecord> {
    csv::Reader::from_path(path)
        .unwrap()
        .records()
        .collect::<Result<_, _>>()
        .unwrap()
}

#[test]
fn sightseeing_patterns_match_official_directional_stops() {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../data");
    let rows = records(&data.join("5!station_station_types.csv"));
    let types = records(&data.join("4!types.csv"));
    let stations = records(&data.join("3!stations.csv"));
    let names: BTreeMap<_, _> = stations.iter().map(|r| (&r[0], &r[2])).collect();
    // Official A-Train timetable and Kanpachi/Ichiroku route information,
    // checked 2026-10-21; sources and hospitality stops are documented in
    // docs/kagoshima-services-audit.md. Stored order is Kumamoto/Hakata first.
    for (ty, group, direction, stops) in [
        (552, 1196, "2", vec!["熊本", "宇土", "網田", "三角"]),
        (553, 1197, "1", vec!["熊本", "宇土", "三角"]),
        (
            554,
            1198,
            "2",
            vec!["博多", "田主丸", "恵良", "由布院", "大分", "別府"],
        ),
        (
            555,
            1199,
            "1",
            vec![
                "博多",
                "久留米",
                "うきは",
                "天ケ瀬",
                "由布院",
                "大分",
                "別府",
            ],
        ),
    ] {
        let actual: Vec<_> = rows.iter().filter(|r| r[3] == group.to_string()).collect();
        let mut actual_stops: Vec<_> = actual
            .iter()
            .filter(|r| &r[4] == "0")
            .map(|r| names[&r[1]])
            .collect();
        actual_stops.dedup();
        assert_eq!(actual_stops, stops);
        assert!(actual
            .iter()
            .all(|r| r[2] == ty.to_string() && matches!(&r[4], "0" | "1")));
        let metadata = types.iter().find(|r| r[1] == ty.to_string()).unwrap();
        assert_eq!(&metadata[8], direction);
        assert_eq!(&metadata[9], "4");
        assert!(
            service_rule(Some(ty), Some(group))
                .unwrap()
                .requires_timetable
        );

        let expected: Vec<i32> = if ty <= 553 {
            vec![
                1190341, 1192401, 1190351, 1190342, 1192402, 1190350, 1192412, 1190343, 1192403,
                1192404, 1192405, 1192406, 1192407, 1192408, 1192409, 1192410, 1192411,
            ]
        } else {
            (1190301..=1190318)
                .chain([1191301, 1191337])
                .chain(1191302..=1191336)
                .chain((1190640..=1190643).rev())
                .collect()
        };
        assert_eq!(
            actual
                .iter()
                .map(|r| r[1].parse::<i32>().unwrap())
                .collect::<Vec<_>>(),
            expected
        );
    }
}

#[test]
fn hospitality_stops_do_not_allow_boarding_and_directional_permissions_match_faq() {
    let rules = parse_stop_rules(include_str!("../../data/route_stop_rules.csv")).unwrap();
    for (ty, gid, permissions) in [
        (554, 1190101, [true, false, false, false]),
        (554, 1191307, [false; 4]),
        (554, 1191320, [false; 4]),
        (554, 1191324, [false, true, false, false]),
        (554, 1190643, [false, true, false, false]),
        (554, 1190640, [false, true, false, false]),
        (555, 1190640, [false, false, true, false]),
        (555, 1190643, [false, false, true, false]),
        (555, 1191324, [false, false, true, false]),
        (555, 1191316, [false; 4]),
        (555, 1191309, [false; 4]),
        (555, 1190318, [false, false, false, true]),
        (555, 1190101, [false, false, false, true]),
    ] {
        let r = rules
            .iter()
            .find(|r| r.type_cd == ty && r.station_group_id == gid)
            .unwrap();
        assert_eq!(
            [
                r.forward_board,
                r.forward_alight,
                r.reverse_board,
                r.reverse_alight
            ],
            permissions
        );
    }
}

#[test]
fn all_kagoshima_rapid_trains_stop_at_ebitsu() {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../data");
    let rows = records(&data.join("5!station_station_types.csv"));
    let ebitsu = rows
        .iter()
        .find(|r| &r[3] == "259" && &r[1] == "1190217")
        .unwrap();
    assert_eq!(&ebitsu[4], "0");
}
