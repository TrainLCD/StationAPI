use super::*;

struct Fixture(std::path::PathBuf);
impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "stationapi-daito-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&dir).unwrap();
        for entry in fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/daito")).unwrap()
        {
            let entry = entry.unwrap();
            fs::copy(entry.path(), dir.join(entry.file_name())).unwrap();
        }
        Self(dir)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn daito_mixed_encoding_imports_all_routes_and_osaka_stops() {
    let fixture = Fixture::new();
    let registration: Registration = serde_json::from_str(include_str!(
        "../../../data/gtfs-feeds/zz_gtfs_0263_osaka_extra_daito.json"
    ))
    .unwrap();
    assert!(std::str::from_utf8(&fs::read(fixture.0.join("trips.txt")).unwrap()).is_err());
    registration
        .normalization
        .unwrap()
        .prepare(&fixture.0)
        .unwrap();
    let normalized = fs::read(fixture.0.join("trips.txt")).unwrap();
    assert!(std::str::from_utf8(&normalized).is_ok());
    registration
        .normalization
        .unwrap()
        .prepare(&fixture.0)
        .unwrap();
    assert_eq!(fs::read(fixture.0.join("trips.txt")).unwrap(), normalized);
    let feed = feed::GtfsFeed {
        id: "zz_gtfs_0263_osaka_extra_daito",
        name: &registration.name,
        path: fixture.0.to_str().unwrap(),
        url: &registration.url,
        requires_consumer_key: false,
    };
    let mut gtfs = GtfsData::default();
    crate::gtfs::parse::load_feed(&mut gtfs, &feed, true).unwrap();
    crate::gtfs::verify_patterns(&gtfs).unwrap();
    assert_eq!(
        (
            gtfs.routes.len(),
            gtfs.stops.len(),
            gtfs.trips.len(),
            gtfs.stop_times.len()
        ),
        (17, 238, 158, 1893)
    );
    for route in &mut gtfs.routes {
        route.company_cd = Some(registration.company_cd);
    }
    let mut dataset =
        crate::rail::Dataset::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../data")).unwrap();
    register_company(&mut dataset.companies, &registration).unwrap();
    let lines = dataset.lines.len();
    let stations = dataset.stations.len();
    let types = dataset.types.len();
    crate::gtfs::integrate::integrate(&mut dataset, &gtfs).unwrap();
    assert_eq!(dataset.lines.len() - lines, 17);
    let pref = dataset.stations.col("pref_cd");
    assert_eq!(dataset.stations.len() - stations, 222);
    assert!(dataset.stations.rows()[stations..]
        .iter()
        .all(|row| row[pref].as_deref() == Some("27")));
    assert_eq!(dataset.types.len() - types, 20);
    eprintln!(
        "Daito: lines={} stations={} patterns={}",
        dataset.lines.len() - lines,
        dataset.stations.len() - stations,
        dataset.types.len() - types
    );
}

#[test]
fn rejects_invalid_encoding_and_paths_without_replacement() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join("trips.txt"), [0x81]).unwrap();
    assert!(Normalization::Daito2026.prepare(&fixture.0).is_err());
    assert_eq!(fs::read(fixture.0.join("trips.txt")).unwrap(), [0x81]);
}

#[test]
fn unexpected_stop_defects_do_not_modify_either_file() {
    let fixture = Fixture::new();
    let trips_before = fs::read(fixture.0.join("trips.txt")).unwrap();
    let stops = fs::read_to_string(fixture.0.join("stops.txt")).unwrap();
    let stops = stops.replace("S53,総合文化センター", "UNKNOWN,総合文化センター");
    fs::write(fixture.0.join("stops.txt"), &stops).unwrap();
    assert!(Normalization::Daito2026.prepare(&fixture.0).is_err());
    assert_eq!(fs::read(fixture.0.join("trips.txt")).unwrap(), trips_before);
    assert_eq!(
        fs::read_to_string(fixture.0.join("stops.txt")).unwrap(),
        stops
    );
}

#[test]
fn daito_staging_write_failure_preserves_cache_and_can_retry() {
    let fixture = Fixture::new();
    let trips_before = fs::read(fixture.0.join("trips.txt")).unwrap();
    let stops_before = fs::read(fixture.0.join("stops.txt")).unwrap();
    // 最初の一時ファイルを書き込んだ後、二つ目の書き込みを失敗させる。
    let blocked = fixture.0.join("stops.txt.tmp");
    fs::create_dir(&blocked).unwrap();
    assert!(Normalization::Daito2026.prepare(&fixture.0).is_err());
    assert_eq!(fs::read(fixture.0.join("trips.txt")).unwrap(), trips_before);
    assert_eq!(fs::read(fixture.0.join("stops.txt")).unwrap(), stops_before);
    fs::remove_dir(blocked).unwrap();
    Normalization::Daito2026.prepare(&fixture.0).unwrap();
    assert!(std::str::from_utf8(&fs::read(fixture.0.join("trips.txt")).unwrap()).is_ok());
    assert!(!fixture.0.join("trips.txt.tmp").exists());
    assert!(!fixture.0.join("stops.txt.tmp").exists());
}
