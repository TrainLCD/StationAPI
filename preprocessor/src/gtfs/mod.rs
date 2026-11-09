//! GTFS / ODPT の取り込み。

pub mod feed;
pub mod integrate;
pub mod model;
pub mod odpt;
pub mod parse;
mod prefecture;
mod registered;

use std::collections::HashSet;

use anyhow::{bail, ensure, Result};

use crate::warn;
use feed::GtfsFeed;
use model::GtfsData;

/// 従来のフィードを読み込み、取得や解析に失敗した場合は警告する。
fn load_gtfs_feed(data: &mut GtfsData, feed: &GtfsFeed) {
    if let Err(e) = feed::download(feed) {
        warn!("{} の取得に失敗: {e}。このフィードを飛ばす", feed.name);
        return;
    }
    if let Err(e) = parse::load_feed(data, feed, false) {
        warn!("{} の読み込みに失敗: {e}。このフィードを飛ばす", feed.name);
    }
}

/// フィードを一時データに読み込み、全路線に停車順がある場合にだけ返す。
fn read_complete_feed(feed: &GtfsFeed, prepare: impl FnOnce(&mut GtfsData)) -> Option<GtfsData> {
    let mut data = GtfsData::default();
    if let Err(error) = feed::download(feed)
        .and_then(|()| parse::load_feed(&mut data, feed, true))
        .and_then(|()| {
            prepare(&mut data);
            verify_patterns(&data)
        })
    {
        warn!(
            "{} の読み込みに失敗: {error}。このフィードを飛ばす",
            feed.name
        );
        return None;
    }
    Some(data)
}

/// 路線ごとに、停車時刻に結び付く便があることを確認する。
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

/// 全フィードを取得して読み込む。
///
/// 1 つのフィードが落ちても他は取り込む。事業者側の一時的な不調や URL 変更で
/// バスが丸ごと消えるのは影響が大きすぎるため、警告にとどめて先へ進む。
pub fn load(companies: &mut crate::table::Table) -> Result<GtfsData> {
    let mut data = GtfsData::default();

    for feed in feed::GTFS_FEEDS {
        load_gtfs_feed(&mut data, feed);
    }

    match odpt::download() {
        Ok(odpt_data) => odpt::load(&mut data, &odpt_data),
        Err(e) => warn!("東急バスの ODPT JSON を取得できない: {e}。このフィードを飛ばす"),
    }

    // 既存の路線・停留所 ID の衝突解決順を維持するため、東急の後に足す。
    load_gtfs_feed(&mut data, &feed::NAGOYA_CITY_BUS);
    load_gtfs_feed(&mut data, &feed::HOKKAIDO_CHUO_BUS);
    load_gtfs_feed(&mut data, &feed::HOKKAIDO_EXPRESS_BUS);
    load_gtfs_feed(&mut data, &feed::JOTETSU_BUS);
    load_gtfs_feed(&mut data, &feed::CHIYODA_KAZAGURUMA);
    load_gtfs_feed(&mut data, &feed::ODAKYU_BUS);
    load_gtfs_feed(&mut data, &feed::NISHI_TOKYO_BUS);
    if let Some(chiibus) = read_complete_feed(&feed::MINATO_CHIIBUS, |_| {}) {
        data.append_feed(chiibus);
    }

    load_gtfs_feed(&mut data, &feed::BUNKYO_BGURU);
    registered::load(&mut data, companies)?;
    data.drop_dangling_parents();

    // 1 つも取り込めていない場合は失敗させる。ここで成功にすると、バスが
    // 丸ごと欠けた generated/*.csv がビルド成功として出てしまう。
    // 鉄道だけで良い場合は DISABLE_BUS_FEATURE=true を使う。
    if data.routes.is_empty() {
        bail!(
            "GTFS のフィードを 1 つも取り込めなかった。\
             ODPT_ACCESS_TOKEN を設定するか、バスが不要なら DISABLE_BUS_FEATURE=true を指定する"
        );
    }

    crate::info!(
        "GTFS 取り込み: 系統 {} / 停留所 {} / 便 {} / 停車 {}",
        data.routes.len(),
        data.stops.len(),
        data.trips.len(),
        data.stop_times.len()
    );
    Ok(data)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn complete_feed_rejects_invalid_stop_times() {
        let dir =
            std::env::temp_dir().join(format!("stationapi_pp_strict_gtfs_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("routes.txt"),
            "route_id,route_long_name,route_type\nr,テスト,3\n",
        )
        .unwrap();
        fs::write(
            dir.join("stops.txt"),
            "stop_id,stop_name,stop_lat,stop_lon\ns,停留所,35.0,139.0\n",
        )
        .unwrap();
        fs::write(dir.join("trips.txt"), "route_id,trip_id\nr,t\n").unwrap();
        let feed = GtfsFeed {
            path: dir.to_str().unwrap(),
            ..feed::MINATO_CHIIBUS
        };
        let stop_times = dir.join("stop_times.txt");
        let valid = "trip_id,stop_id,stop_sequence\nt,s,1\n";
        fs::write(&stop_times, valid).unwrap();
        let accepts_valid = read_complete_feed(&feed, |_| {}).is_some();
        fs::write(&stop_times, format!("{valid}unknown,s,2\n")).unwrap();
        let rejects_unknown_trip = read_complete_feed(&feed, |_| {}).is_none();
        fs::write(&stop_times, format!("{valid}t,unknown,2\n")).unwrap();
        let rejects_unknown_stop = read_complete_feed(&feed, |_| {}).is_none();
        fs::write(&stop_times, format!("{valid}t,s,1\n")).unwrap();
        let rejects_duplicate_sequence = read_complete_feed(&feed, |_| {}).is_none();
        fs::write(&stop_times, "trip_id,stop_id,stop_sequence\nt,s,invalid\n").unwrap();
        let rejects_invalid_sequence = read_complete_feed(&feed, |_| {}).is_none();
        fs::write(&stop_times, "trip_id,stop_id,stop_sequence\nt,s,-1\n").unwrap();
        let rejects_negative_sequence = read_complete_feed(&feed, |_| {}).is_none();
        fs::write(&stop_times, valid).unwrap();
        fs::write(
            dir.join("routes.txt"),
            "route_id,route_long_name,route_type\nr,テスト,3\npaused,運休中,3\n",
        )
        .unwrap();
        fs::write(
            dir.join("stops.txt"),
            "stop_id,stop_name,stop_lat,stop_lon,parent_station\ns,乗り場,35.0,139.0,p\np,親停留所,35.0,139.0,\nu,未使用,36.0,139.0,\n",
        )
        .unwrap();
        let accepts_exclusion_before_validation = read_complete_feed(&feed, |data| {
            data.exclude_routes(&HashSet::from([feed::scoped_id(&feed, "paused")]));
        })
        .is_some_and(|data| {
            data.routes.len() == 1
                && data.has_stop(&feed::scoped_id(&feed, "s"))
                && data.has_stop(&feed::scoped_id(&feed, "p"))
                && !data.has_stop(&feed::scoped_id(&feed, "u"))
        });
        fs::remove_dir_all(dir).unwrap();
        assert!(
            accepts_valid
                && rejects_unknown_trip
                && rejects_unknown_stop
                && rejects_duplicate_sequence
                && rejects_invalid_sequence
                && rejects_negative_sequence
                && accepts_exclusion_before_validation
        );
    }
}
