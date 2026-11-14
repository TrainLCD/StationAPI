//! フィード別の設定を読み込み、既存フィードの後に追加する。

use std::{fs, path::Path};

use anyhow::{ensure, Context, Result};
use serde::Deserialize;

use super::{feed, model::GtfsData, read_complete_feed_prepared};
use crate::table::{int, text, Table};

#[path = "normalization.rs"]
mod normalization;
use normalization::Normalization;

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
    #[serde(default)]
    normalization: Option<Normalization>,
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

    let Some(mut feed_data) = read_complete_feed_prepared(
        &feed,
        || {
            registration
                .normalization
                .map_or(Ok(()), |profile| profile.prepare(Path::new(&cache)))
        },
        |data| {
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
        },
    ) else {
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

#[cfg(test)]
#[path = "daito_tests.rs"]
mod daito_tests;

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
