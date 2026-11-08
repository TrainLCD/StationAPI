//! フィード別の設定を読み込み、既存フィードの後に追加する。

use std::{collections::HashSet, fs, path::Path};

use anyhow::{ensure, Context, Result};
use serde::Deserialize;

use super::{feed, model::GtfsData, parse};
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
    company_cd: i32,
    company: Option<Company>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Company {
    name: String,
    kana: String,
    english: String,
    url: String,
}

pub(super) fn is_registered(id: &str) -> bool {
    id.starts_with(NAMESPACE_PREFIX)
}

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

    // 途中で壊れたフィードの一部だけが出力に混ざらないよう、別のデータに読む。
    let mut feed_data = GtfsData::default();
    if let Err(error) = feed::download(&feed)
        .and_then(|()| parse::load_feed(&mut feed_data, &feed))
        .and_then(|()| verify_patterns(&feed_data))
    {
        crate::warn!(
            "{} の読み込みに失敗: {error}。このフィードを飛ばす",
            feed.name
        );
        return Ok(());
    }

    register_company(companies, &registration)?;
    for mut route in feed_data.routes {
        route.company_cd = Some(registration.company_cd);
        data.push_route(route);
    }
    for stop in feed_data.stops {
        data.push_stop(stop);
    }
    for trip in feed_data.trips {
        data.push_trip(trip);
    }
    for time in feed_data.stop_times {
        data.push_stop_time(time);
    }
    crate::info!("{} の GTFS を取り込んだ", feed.name);
    Ok(())
}

fn verify_patterns(data: &GtfsData) -> Result<()> {
    let trips_with_stops: HashSet<_> = data
        .stop_times
        .iter()
        .map(|time| time.trip_id.as_str())
        .collect();
    let served_routes: HashSet<_> = data
        .trips
        .iter()
        .filter(|trip| trips_with_stops.contains(trip.trip_id.as_str()))
        .map(|trip| trip.route_id.as_str())
        .collect();
    ensure!(
        !data.routes.is_empty()
            && data
                .routes
                .iter()
                .all(|route| served_routes.contains(route.route_id.as_str())),
        "有効な停車順がない路線がある"
    );
    Ok(())
}

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
        text(&company.name),
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
