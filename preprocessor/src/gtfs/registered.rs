//! フィード別の設定を読み込み、既存フィードの後に追加する。

use std::{fs, path::Path};

use anyhow::{ensure, Context, Result};
use serde::Deserialize;

use super::{feed, model::GtfsData, read_complete_feed};
use crate::table::{int, text, Table};

const NAMESPACE_PREFIX: &str = "zz_gtfs_";
const ODPT_AUTH_PREFIX: &str = "https://api.odpt.org/";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    name: String,
    url: String,
    source: String,
    license: String,
    #[serde(default)]
    excluded_route_ids: Vec<String>,
    company_cd: i32,
    company: Option<Company>,
    #[serde(default)]
    allow_routes_without_trips: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Company {
    name: String,
    name_full: Option<String>,
    kana: String,
    english: String,
    url: String,
}

/// 設定ファイルから登録した名前空間を識別する。
pub(super) fn is_registered(id: &str) -> bool {
    id.starts_with(NAMESPACE_PREFIX)
}

/// 既存の採番を保つため、設定ファイルを名前順に適用する。
pub(super) fn load(data: &mut GtfsData, companies: &mut Table) -> Result<()> {
    let mut paths = fs::read_dir("data/gtfs-feeds")?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    paths.retain(|path| path.extension().is_some_and(|ext| ext == "json"));
    paths.sort();
    for path in paths {
        load_registration(data, companies, &path)
            .with_context(|| format!("GTFS 設定 {}", path.display()))?;
    }
    Ok(())
}

/// 設定を検証し、取り込みに成功したフィードと事業者を追加する。
fn load_registration(data: &mut GtfsData, companies: &mut Table, path: &Path) -> Result<()> {
    let id = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    ensure!(
        is_registered(id)
            && id.len() > NAMESPACE_PREFIX.len()
            && id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
        "ファイル名は zz_gtfs_ に続く英小文字・数字・アンダースコアを使う"
    );
    let registration: Registration = serde_json::from_reader(fs::File::open(path)?)?;
    ensure!(
        registration.url.starts_with("https://")
            && registration.source.starts_with("https://")
            && !registration.license.trim().is_empty(),
        "HTTPS の配布先・出典とライセンスが必要"
    );
    let cache = format!("data/gtfs-cache/{id}");
    let feed = feed::GtfsFeed {
        id,
        name: &registration.name,
        path: &cache,
        url: &registration.url,
        requires_consumer_key: registration.url.starts_with(ODPT_AUTH_PREFIX),
    };

    let Some(mut feed_data) = read_complete_feed(&feed, |data| {
        if !registration.excluded_route_ids.is_empty() {
            let excluded = registration
                .excluded_route_ids
                .iter()
                .map(|route_id| feed::scoped_id(&feed, route_id))
                .collect();
            data.exclude_routes(&excluded);
        }
        if registration.allow_routes_without_trips {
            data.drop_routes_without_trips();
        }
    }) else {
        return Ok(());
    };
    register_company(companies, &registration)?;
    for route in &mut feed_data.routes {
        route.company_cd = Some(registration.company_cd);
    }
    data.append_feed(feed_data);
    crate::info!("{} の GTFS を取り込んだ", feed.name);
    Ok(())
}

/// 既存の事業者を再利用し、同じコードで異なる事業者を登録する誤りを防ぐ。
fn register_company(table: &mut Table, registration: &Registration) -> Result<()> {
    let code = registration.company_cd;
    let code_cell = int(code);
    let code_col = table.col("company_cd");
    let existing = table.rows().iter().find(|row| row[code_col] == code_cell);
    let Some(company) = &registration.company else {
        ensure!(existing.is_some(), "事業者コード {code} が未登録");
        return Ok(());
    };
    let row = vec![
        code_cell,
        int(99),
        text(&company.name),
        text(&company.kana),
        text(company.name_full.as_deref().unwrap_or(&company.name)),
        text(&company.name),
        text(&company.english),
        text(&company.english),
        text(&company.url),
        int(0),
        int(0),
        int(code),
    ];
    if let Some(existing) = existing {
        ensure!(*existing == row, "事業者コード {code} が異なる事業者と重複");
    } else {
        table.push(row);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn company_full_name_is_optional_and_keeps_the_short_name() {
        let mut table = Table::new(crate::rail::COMPANY_COLUMNS, Some("company_cd"));
        let mut registration: Registration = serde_json::from_value(serde_json::json!({
            "name": "テストフィード",
            "url": "https://example.com/feed.zip",
            "source": "https://example.com/",
            "license": "CC0",
            "company_cd": 1,
            "company": {
                "name": "テストバス",
                "kana": "テストバス",
                "english": "Test Bus",
                "url": "https://example.com/"
            }
        }))
        .unwrap();
        register_company(&mut table, &registration).unwrap();

        registration.company_cd = 2;
        registration.company.as_mut().unwrap().name_full = Some("テストバス株式会社".into());
        register_company(&mut table, &registration).unwrap();

        let full = table.col("company_name_h");
        let short = table.col("company_name_r");
        assert_eq!(table.rows()[0][full].as_deref(), Some("テストバス"));
        assert_eq!(table.rows()[1][full].as_deref(), Some("テストバス株式会社"));
        assert_eq!(table.rows()[1][short].as_deref(), Some("テストバス"));
    }
}

#[cfg(test)]
mod limonbus_tests {
    use super::*;

    #[test]
    fn limonbus_preserves_routes_patterns_platforms_and_prefectures() {
        let registration: Registration = serde_json::from_str(include_str!(
            "../../../data/gtfs-feeds/zz_gtfs_0262_osaka_0535_limonbus.json"
        ))
        .unwrap();
        let feed = feed::GtfsFeed {
            id: "zz_gtfs_0262_osaka_0535_limonbus",
            name: &registration.name,
            path: concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/limonbus"),
            url: &registration.url,
            requires_consumer_key: false,
        };
        let mut gtfs = GtfsData::default();
        crate::gtfs::parse::load_feed(&mut gtfs, &feed, true).unwrap();
        crate::gtfs::verify_patterns(&gtfs).unwrap();
        assert_eq!(gtfs.routes.len(), 4);
        assert_eq!(gtfs.stops.len(), 11);
        assert_eq!(gtfs.trips.len(), 4);
        for route in &mut gtfs.routes {
            route.company_cd = Some(registration.company_cd);
        }
        let mut dataset =
            crate::rail::Dataset::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../data"))
                .unwrap();
        register_company(&mut dataset.companies, &registration).unwrap();
        let lines = dataset.lines.len();
        let stations = dataset.stations.len();
        let types = dataset.types.len();
        let sst = dataset.sst.len();
        crate::gtfs::integrate::integrate(&mut dataset, &gtfs).unwrap();
        assert_eq!(dataset.lines.len() - lines, 4);
        assert_eq!(dataset.types.len() - types, 4);
        assert_eq!(dataset.stations.len() - stations, 24);
        assert_eq!(dataset.sst.len() - sst, 24);
        let name = dataset.stations.col("station_name");
        let prefecture = dataset.stations.col("pref_cd");
        let platform = dataset.stations.col("platform_code");
        let expected = [
            ("大阪・なんば", "27"),
            ("京都駅", "26"),
            ("金沢駅", "17"),
            ("高山駅", "21"),
            ("ひだホテルプラザ", "21"),
            ("平湯", "21"),
            ("河口湖駅", "19"),
            ("ハイランドリゾートホテル＆スパ", "19"),
            ("東京八重洲", "13"),
            ("松本", "20"),
        ];
        for (stop, code) in expected {
            let rows: Vec<_> = dataset.stations.rows()[stations..]
                .iter()
                .filter(|row| row[name].as_deref() == Some(stop))
                .collect();
            assert!(!rows.is_empty(), "missing {stop}");
            assert!(
                rows.iter()
                    .all(|row| row[prefecture].as_deref() == Some(code)),
                "wrong prefecture for {stop}"
            );
            if stop == "大阪・なんば" {
                assert!(rows.iter().all(|row| row[platform].as_deref() == Some("5")));
            }
        }
    }
}
