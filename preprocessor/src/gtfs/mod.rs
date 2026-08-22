//! GTFS / ODPT の取り込み。

pub mod feed;
pub mod integrate;
pub mod model;
pub mod odpt;
pub mod parse;

use anyhow::Result;

use crate::warn;
use model::GtfsData;

/// 全フィードを取得して読み込む。
///
/// 1 つのフィードが落ちても他は取り込む。事業者側の一時的な不調や URL 変更で
/// バスが丸ごと消えるのは影響が大きすぎるため、警告にとどめて先へ進む。
pub fn load() -> Result<GtfsData> {
    let mut data = GtfsData::default();

    for feed in feed::GTFS_FEEDS {
        if let Err(e) = feed::download(feed) {
            warn!("{} の取得に失敗: {e}。このフィードを飛ばす", feed.name);
            continue;
        }
        if let Err(e) = parse::load_feed(&mut data, feed) {
            warn!("{} の読み込みに失敗: {e}。このフィードを飛ばす", feed.name);
        }
    }

    match odpt::download() {
        Ok(odpt_data) => odpt::load(&mut data, &odpt_data),
        Err(e) => warn!("東急バスの ODPT JSON を取得できない: {e}。このフィードを飛ばす"),
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
