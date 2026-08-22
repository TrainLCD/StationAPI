//! GTFS の中間表現。
//!
//! かつては `gtfs_*` テーブルへ一度書き込んでから SQL で統合していた。ここでは
//! その中間テーブルをメモリ上に持つ。統合処理が実際に読む列だけを持たせてある
//! (calendar / shapes / feed_info / agencies は出力に一切効かないので取り込まない)。
//!
//! 主キーの重複は `ON CONFLICT DO NOTHING` と同じく先勝ちで捨てる。

use std::collections::{HashMap, HashSet};

use crate::warn;

#[derive(Debug, Clone)]
pub struct Route {
    pub route_id: String,
    pub route_short_name: Option<String>,
    pub route_long_name: Option<String>,
    /// GTFS フィードには英語名の列が無く、東急 ODPT からも入らないため常に `None`。
    /// 種別名のローマ字化で参照されるので、列としては残してある。
    pub route_long_name_r: Option<String>,
    pub route_type: i32,
    pub route_color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Stop {
    pub stop_id: String,
    pub stop_name: String,
    pub stop_name_k: Option<String>,
    pub stop_name_r: Option<String>,
    pub stop_name_zh: Option<String>,
    pub stop_name_ko: Option<String>,
    pub stop_lat: f64,
    pub stop_lon: f64,
    /// 空文字は取り込み時に `None` へ寄せる。親子関係を持つフィードだけが値を持つ。
    pub parent_station: Option<String>,
}

impl Stop {
    /// 物理的な停留所を表す ID。親があれば親、無ければ自分。
    pub fn parent_stop_id(&self) -> &str {
        self.parent_station.as_deref().unwrap_or(&self.stop_id)
    }
}

#[derive(Debug, Clone)]
pub struct Trip {
    pub trip_id: String,
    pub route_id: String,
    pub trip_headsign: Option<String>,
    pub direction_id: Option<i32>,
    pub shape_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StopTime {
    pub trip_id: String,
    pub stop_id: String,
    pub stop_sequence: i32,
    pub shape_dist_traveled: Option<f64>,
}

/// 取り込んだ GTFS 一式。行の並びは取り込み順を保つ。
#[derive(Default)]
pub struct GtfsData {
    pub routes: Vec<Route>,
    pub stops: Vec<Stop>,
    pub trips: Vec<Trip>,
    pub stop_times: Vec<StopTime>,

    route_ids: HashSet<String>,
    stop_ids: HashSet<String>,
    trip_ids: HashSet<String>,
    /// (trip_id, stop_sequence) の一意制約。
    stop_time_keys: HashSet<(String, i32)>,
}

impl GtfsData {
    pub fn push_route(&mut self, route: Route) {
        if self.route_ids.insert(route.route_id.clone()) {
            self.routes.push(route);
        }
    }

    pub fn push_stop(&mut self, stop: Stop) {
        if self.stop_ids.insert(stop.stop_id.clone()) {
            self.stops.push(stop);
        }
    }

    /// 参照先の route が無い trip は取り込まない (外部キー相当)。
    pub fn push_trip(&mut self, trip: Trip) {
        if !self.route_ids.contains(&trip.route_id) {
            warn!(
                "trip {} の route_id {} が routes に無いため捨てる",
                trip.trip_id, trip.route_id
            );
            return;
        }
        if self.trip_ids.insert(trip.trip_id.clone()) {
            self.trips.push(trip);
        }
    }

    /// 参照先の trip / stop が無い行は取り込まない (外部キー相当)。
    pub fn push_stop_time(&mut self, stop_time: StopTime) {
        if !self.trip_ids.contains(&stop_time.trip_id)
            || !self.stop_ids.contains(&stop_time.stop_id)
        {
            return;
        }
        let key = (stop_time.trip_id.clone(), stop_time.stop_sequence);
        if self.stop_time_keys.insert(key) {
            self.stop_times.push(stop_time);
        }
    }

    pub fn has_stop(&self, stop_id: &str) -> bool {
        self.stop_ids.contains(stop_id)
    }

    /// stop_id -> 物理停留所 ID の索引。
    pub fn parent_stop_ids(&self) -> HashMap<&str, &str> {
        self.stops
            .iter()
            .map(|stop| (stop.stop_id.as_str(), stop.parent_stop_id()))
            .collect()
    }

    /// trip_id -> stop_sequence 昇順の stop_times。
    pub fn stop_times_by_trip(&self) -> HashMap<&str, Vec<&StopTime>> {
        let mut map: HashMap<&str, Vec<&StopTime>> = HashMap::new();
        for stop_time in &self.stop_times {
            map.entry(stop_time.trip_id.as_str())
                .or_default()
                .push(stop_time);
        }
        for times in map.values_mut() {
            times.sort_by_key(|time| time.stop_sequence);
        }
        map
    }
}
