//! GTFS を lines / stations / types / station_station_types へ統合する。
//!
//! かつては `gtfs_*` テーブルに対する SQL (再帰 CTE と窓関数) で行っていた処理を
//! そのまま Rust に写したもの。並び順や同点時の決着条件は出力の
//! `station_station_types.id` (= 停車順序) に直結するため、元の `ORDER BY` を
//! 崩さないようにしてある。

use std::collections::{HashMap, HashSet};

use stationapi::domain::arrival_estimation::haversine_distance;
use stationapi::domain::romaji::strip_macrons;

use super::model::{GtfsData, Stop};
use crate::codes::{
    bus_line_cd, bus_line_group_cd, bus_station_cd, bus_station_g_cd, bus_type_cd,
    company_cd_for_gtfs_route, hiragana_to_katakana,
};
use crate::info;
use crate::rail::{assign_serial, Dataset};
use crate::table::{int, text, Cell};

/// バス系統を表す `types.kind`。`TrainTypeKind::BusRoute` と同じ値。
const BUS_ROUTE_KIND: i32 = 7;
/// 路線色を持たないフィード向けの既定色。
const DEFAULT_BUS_LINE_COLOR: &str = "#1f63c6";
/// 同じ名前のバス停をひとつの物理停留所とみなす距離。
const BUS_STOP_GROUPING_RADIUS_METERS: f64 = 250.0;

/// 物理停留所 -> [(系統, 停車順)]。
pub type StopRouteMap = HashMap<String, Vec<(String, i32)>>;

/// GTFS 一式を鉄道側のデータセットへ統合する。
pub fn integrate(dataset: &mut Dataset, gtfs: &GtfsData) {
    if gtfs.routes.is_empty() {
        info!("GTFS の系統が無いため統合を省略する");
        return;
    }

    routes_to_lines(dataset, gtfs);
    let stop_route_map = build_stop_route_mapping(gtfs);
    info!("物理停留所 {} 件の停車順を決めた", stop_route_map.len());
    stops_to_stations(dataset, gtfs, &stop_route_map);
    trip_variations_to_types(dataset, gtfs);

    // types / station_station_types はバス行を足したので採番し直す。
    assign_serial(&mut dataset.types, "id");
    assign_serial(&mut dataset.sst, "id");
}

/// gtfs_routes を lines へ。
fn routes_to_lines(dataset: &mut Dataset, gtfs: &GtfsData) {
    let mut added = 0usize;
    for route in &gtfs.routes {
        // 接頭辞から事業者を引けない系統は取り込まない。
        let Some(company_cd) = company_cd_for_gtfs_route(&route.route_id) else {
            continue;
        };
        let line_cd = bus_line_cd(&route.route_id);
        let line_name = route
            .route_short_name
            .clone()
            .or_else(|| route.route_long_name.clone())
            .unwrap_or_default();
        let line_color = route
            .route_color
            .as_deref()
            .filter(|color| !color.is_empty())
            .map(|color| {
                if color.starts_with('#') {
                    color.to_string()
                } else {
                    format!("#{color}")
                }
            })
            .unwrap_or_else(|| DEFAULT_BUS_LINE_COLOR.to_string());

        let mut row = dataset.lines.blank_row();
        set(&dataset.lines, &mut row, "line_cd", int(line_cd));
        set(&dataset.lines, &mut row, "company_cd", int(company_cd));
        set(
            &dataset.lines,
            &mut row,
            "line_name",
            text(line_name.clone()),
        );
        set(
            &dataset.lines,
            &mut row,
            "line_name_k",
            text(line_name.clone()),
        );
        set(&dataset.lines, &mut row, "line_name_h", text(line_name));
        set(
            &dataset.lines,
            &mut row,
            "line_name_r",
            text(route.route_long_name.clone().unwrap_or_default()),
        );
        set(&dataset.lines, &mut row, "line_color_c", text(line_color));
        set(&dataset.lines, &mut row, "line_type", int(route.route_type));
        set(&dataset.lines, &mut row, "e_status", int(0));
        set(&dataset.lines, &mut row, "e_sort", int(line_cd));
        // average_distance は鉄道側でしか計算しない。バス路線は DDL の既定値と
        // 同じ 0 を入れる (空欄にすると Worker 側で NULL 扱いになる)。
        set(&dataset.lines, &mut row, "average_distance", int(0));
        set(&dataset.lines, &mut row, "transport_type", int(1));
        if dataset.lines.push(row) {
            added += 1;
        }
    }
    info!("バス系統 {added} 本を路線として取り込んだ");
}

/// 代表 shape を選ぶ順位。停留所数 -> 走行距離 -> direction -> shape_id の順に見る。
/// 停留所数を direction より先に見るのは、池86 のように全区間が direction 未設定の
/// 便にしか記録されていない系統があるため。
type ShapeRank<'a> = (usize, Option<f64>, u8, &'a str);

/// 代表便を選ぶ順位。代表 shape に乗っているか -> direction_id=0 か ->
/// 停留所数 -> 走行距離 -> 停車数 -> trip_id の順に見る。
type TripRank<'a> = (u8, u8, usize, Option<f64>, usize, &'a str);

/// 循環便の 1 停留所。(物理停留所 ID, 停留所名, ローマ字名)。
type LoopStop<'a> = (&'a str, &'a str, Option<&'a str>);

/// 代表となる便の情報。
struct MainTrip<'a> {
    trip_id: &'a str,
    direction_id: Option<i32>,
}

/// 変種便にしか現れない停留所と、その前後の停留所。
struct VariantOnly<'a> {
    direction_id: Option<i32>,
    prev: Option<&'a str>,
    next: Option<&'a str>,
}

/// (物理停留所, 系統) -> 停車順 を決める。
///
/// # 決め方
///
/// 1. 系統ごとに代表の便を 1 本選ぶ。その便の `stop_sequence` を正とする。
///    選択は「代表 shape に乗っているか」「direction_id=0 か」「停留所数」
///    「走行距離」「便 ID」の順で決める。代表 shape は最も多くの停留所を通る
///    shape のこと。距離を direction より先に見るのは、池86 のように全区間が
///    direction_id 未設定の便にしか記録されていない系統があるため。
/// 2. 代表便に無い停留所は、変種便での前後の停留所が代表便のどこに当たるかを
///    見て内挿する。前後がどちらも変種便だけの停留所なら、鎖をたどって
///    代表便に当たるまで遡る。
pub fn build_stop_route_mapping(gtfs: &GtfsData) -> StopRouteMap {
    let stop_times_by_trip = gtfs.stop_times_by_trip();
    let parent_of = gtfs.parent_stop_ids();

    // 停留所を 1 件も持たない便は、SQL 側でも JOIN で落ちていたので除く。
    let trips: Vec<&super::model::Trip> = gtfs
        .trips
        .iter()
        .filter(|trip| stop_times_by_trip.contains_key(trip.trip_id.as_str()))
        .collect();

    // --- 系統ごとの代表 shape -------------------------------------------------
    // (route, shape, direction) ごとに集計し、停留所数 -> 距離 -> direction ->
    // shape_id の順で最良のものを採る。
    struct ShapeGroup<'a> {
        shape_id: &'a str,
        direction_id: Option<i32>,
        stops: HashSet<&'a str>,
        max_dist: Option<f64>,
    }
    let mut shape_groups: HashMap<(&str, &str, Option<i32>), ShapeGroup> = HashMap::new();
    for trip in &trips {
        let Some(shape_id) = trip.shape_id.as_deref() else {
            continue;
        };
        let entry = shape_groups
            .entry((trip.route_id.as_str(), shape_id, trip.direction_id))
            .or_insert_with(|| ShapeGroup {
                shape_id,
                direction_id: trip.direction_id,
                stops: HashSet::new(),
                max_dist: None,
            });
        for stop_time in &stop_times_by_trip[trip.trip_id.as_str()] {
            entry.stops.insert(stop_time.stop_id.as_str());
            entry.max_dist = max_option(entry.max_dist, stop_time.shape_dist_traveled);
        }
    }
    let mut canonical_shape: HashMap<&str, &str> = HashMap::new();
    let mut best_shape: HashMap<&str, ShapeRank> = HashMap::new();
    for ((route_id, _, _), group) in &shape_groups {
        let key = (
            group.stops.len(),
            group.max_dist,
            direction_rank(group.direction_id),
            group.shape_id,
        );
        let better = match best_shape.get(route_id) {
            Some(current) => shape_is_better(&key, current),
            None => true,
        };
        if better {
            best_shape.insert(route_id, key);
            canonical_shape.insert(route_id, group.shape_id);
        }
    }

    // --- 系統ごとの代表便 -----------------------------------------------------
    let mut main_trips: HashMap<&str, MainTrip> = HashMap::new();
    let mut best_trip: HashMap<&str, TripRank> = HashMap::new();
    for trip in &trips {
        let times = &stop_times_by_trip[trip.trip_id.as_str()];
        let on_canonical = match (
            trip.shape_id.as_deref(),
            canonical_shape.get(trip.route_id.as_str()),
        ) {
            (Some(shape), Some(canonical)) if shape == *canonical => 0,
            _ => 1,
        };
        let distinct_stops = times
            .iter()
            .map(|time| time.stop_id.as_str())
            .collect::<HashSet<_>>()
            .len();
        let max_dist = times
            .iter()
            .fold(None, |acc, time| max_option(acc, time.shape_dist_traveled));
        let key = (
            on_canonical,
            u8::from(trip.direction_id != Some(0)),
            distinct_stops,
            max_dist,
            times.len(),
            trip.trip_id.as_str(),
        );
        let better = match best_trip.get(trip.route_id.as_str()) {
            Some(current) => trip_is_better(&key, current),
            None => true,
        };
        if better {
            best_trip.insert(trip.route_id.as_str(), key);
            main_trips.insert(
                trip.route_id.as_str(),
                MainTrip {
                    trip_id: trip.trip_id.as_str(),
                    direction_id: trip.direction_id,
                },
            );
        }
    }

    // --- 代表便の停車順 -------------------------------------------------------
    // 同じ物理停留所を複数回通る便では、最初に現れた順番を採る。
    let mut main_stops: HashMap<&str, HashMap<&str, i32>> = HashMap::new();
    for (route_id, main) in &main_trips {
        let mut per_route: HashMap<&str, i32> = HashMap::new();
        for stop_time in &stop_times_by_trip[main.trip_id] {
            let Some(parent) = parent_of.get(stop_time.stop_id.as_str()) else {
                continue;
            };
            per_route
                .entry(parent)
                .and_modify(|seq| *seq = (*seq).min(stop_time.stop_sequence))
                .or_insert(stop_time.stop_sequence);
        }
        main_stops.insert(route_id, per_route);
    }
    let main_max_seq: HashMap<&str, i32> = main_stops
        .iter()
        .filter_map(|(route_id, stops)| stops.values().copied().max().map(|max| (*route_id, max)))
        .collect();

    // --- 変種便にしかない停留所 -----------------------------------------------
    let main_trip_ids: HashSet<&str> = main_trips.values().map(|main| main.trip_id).collect();
    // (route, parent) -> 採用する前後関係。優先度 (前後とも代表便にある > 片方 > なし)、
    // 次に stop_sequence の小さいものを採る。
    let mut variant_only: HashMap<(&str, &str), VariantOnly> = HashMap::new();
    let mut variant_rank: HashMap<(&str, &str), (u8, i32, &str)> = HashMap::new();
    for trip in &trips {
        if main_trip_ids.contains(trip.trip_id.as_str()) {
            continue;
        }
        let route_id = trip.route_id.as_str();
        let empty = HashMap::new();
        let route_main = main_stops.get(route_id).unwrap_or(&empty);
        let times = &stop_times_by_trip[trip.trip_id.as_str()];
        let parents: Vec<&str> = times
            .iter()
            .filter_map(|time| parent_of.get(time.stop_id.as_str()).copied())
            .collect();
        if parents.len() != times.len() {
            continue;
        }
        for (i, parent) in parents.iter().enumerate() {
            if route_main.contains_key(parent) {
                continue;
            }
            let prev = if i > 0 { Some(parents[i - 1]) } else { None };
            let next = parents.get(i + 1).copied();
            let has_prev = prev.is_some_and(|p| route_main.contains_key(p));
            let has_next = next.is_some_and(|n| route_main.contains_key(n));
            let priority = match (has_prev, has_next) {
                (true, true) => 0,
                (true, false) | (false, true) => 1,
                (false, false) => 2,
            };
            // SQL 版は (優先度, stop_sequence) までしか順序を決めておらず、
            // 同点の行のどれが採られるかは PostgreSQL の実行計画任せだった。
            // 実際に決着が付かない停留所があり (例: 京王 1972 系統の 1298_00 は
            // 終点として現れる便と途中停車する便で next が食い違う)、
            // 取り込むたびに停車順が変わりうる。ここでは trip_id まで見て
            // 決め切る。
            let rank = (priority, times[i].stop_sequence, trip.trip_id.as_str());
            let key = (route_id, *parent);
            if variant_rank.get(&key).is_none_or(|current| rank < *current) {
                variant_rank.insert(key, rank);
                variant_only.insert(
                    key,
                    VariantOnly {
                        direction_id: trip.direction_id,
                        prev,
                        next,
                    },
                );
            }
        }
    }

    // 鎖をたどる深さの上限。変種だけの停留所は 1 回しか通らない (visited で保証)
    // ので、その件数 + 1 で足りる。
    let mut chain_limit: HashMap<&str, i32> = HashMap::new();
    for (route_id, _) in variant_only.keys() {
        *chain_limit.entry(route_id).or_insert(1) += 1;
    }

    // --- 停車順の推定 ---------------------------------------------------------
    let mut estimated: Vec<(&str, &str, f64)> = Vec::new();
    for ((route_id, parent), variant) in &variant_only {
        let (Some(main), Some(route_main), Some(max_seq)) = (
            main_trips.get(route_id),
            main_stops.get(route_id),
            main_max_seq.get(route_id),
        ) else {
            continue;
        };
        let max_depth = chain_limit.get(route_id).copied().unwrap_or(1);
        // 変種便の向きが代表便と揃っているか。ODPT が向きを持たない場合は
        // 揃っているものとして扱う。
        let same_direction = variant.direction_id.is_none()
            || (main.direction_id.is_some() && variant.direction_id == main.direction_id);

        let prev_seq = variant.prev.and_then(|p| route_main.get(p)).copied();
        let next_seq = variant.next.and_then(|n| route_main.get(n)).copied();
        let resolved_prev =
            resolve_chain(parent, route_id, &variant_only, route_main, max_depth, true);
        let resolved_next = resolve_chain(
            parent,
            route_id,
            &variant_only,
            route_main,
            max_depth,
            false,
        );

        let seq = match (prev_seq, next_seq) {
            (Some(prev), Some(next)) => (prev as f64 + next as f64) / 2.0,
            (Some(prev), None) => {
                if same_direction {
                    prev as f64 + 0.5
                } else {
                    prev as f64 - 0.5
                }
            }
            (None, Some(next)) => {
                if same_direction {
                    next as f64 - 0.5
                } else {
                    next as f64 + 0.5
                }
            }
            (None, None) => match (resolved_prev, resolved_next) {
                (Some((prev, prev_depth)), Some((next, next_depth))) => {
                    // 深さの差ぶんだけずらして、同じ位置に潰れないようにする。
                    (prev as f64 + next as f64) / 2.0 + (prev_depth - next_depth) as f64 * 0.01
                }
                (Some((prev, depth)), None) => {
                    if same_direction {
                        prev as f64 + 0.1 * depth as f64
                    } else {
                        prev as f64 - 0.1 * depth as f64
                    }
                }
                (None, Some((next, depth))) => {
                    if same_direction {
                        next as f64 - 0.1 * depth as f64
                    } else {
                        next as f64 + 0.1 * depth as f64
                    }
                }
                (None, None) => {
                    if variant.next.is_none() {
                        // 終点。向きが揃っていれば末尾、逆なら先頭。
                        if same_direction {
                            *max_seq as f64 + 0.5
                        } else {
                            0.5
                        }
                    } else if variant.prev.is_none() {
                        if same_direction {
                            0.5
                        } else {
                            *max_seq as f64 + 0.5
                        }
                    } else {
                        // 手掛かりが無いので末尾へ置く。
                        *max_seq as f64 + 9999.0
                    }
                }
            },
        };
        estimated.push((route_id, parent, seq));
    }

    // --- 連番へ振り直す -------------------------------------------------------
    let mut per_route: HashMap<&str, Vec<(f64, &str)>> = HashMap::new();
    for (route_id, stops) in &main_stops {
        for (parent, seq) in stops {
            per_route
                .entry(route_id)
                .or_default()
                .push((*seq as f64, parent));
        }
    }
    for (route_id, parent, seq) in &estimated {
        per_route.entry(route_id).or_default().push((*seq, parent));
    }

    // 系統ごとに推定値で並べ、1 始まりの連番へ振り直す。同値は停留所 ID で決着。
    let mut ordered: Vec<(&str, Vec<(f64, &str)>)> = per_route.into_iter().collect();
    ordered.sort_by_key(|(route_id, _)| *route_id);

    let mut map: StopRouteMap = HashMap::new();
    for (route_id, mut stops) in ordered {
        stops.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(b.1)));
        for (index, (_, parent)) in stops.iter().enumerate() {
            map.entry((*parent).to_string())
                .or_default()
                .push((route_id.to_string(), index as i32 + 1));
        }
    }
    map
}

/// `prev` / `next` の鎖をたどって、最初に代表便へ当たる位置と深さを返す。
fn resolve_chain<'a>(
    origin: &'a str,
    route_id: &'a str,
    variant_only: &HashMap<(&'a str, &'a str), VariantOnly<'a>>,
    route_main: &HashMap<&'a str, i32>,
    max_depth: i32,
    follow_prev: bool,
) -> Option<(i32, i32)> {
    let start = variant_only.get(&(route_id, origin))?;
    let step = |variant: &VariantOnly<'a>| {
        if follow_prev {
            variant.prev
        } else {
            variant.next
        }
    };
    let mut current = step(start)?;
    let mut depth = 1;
    let mut visited: HashSet<&str> = HashSet::from([origin]);
    loop {
        if let Some(seq) = route_main.get(current) {
            return Some((*seq, depth));
        }
        if depth >= max_depth {
            return None;
        }
        let next = variant_only.get(&(route_id, current)).and_then(step)?;
        if !visited.insert(current) {
            return None;
        }
        current = next;
        depth += 1;
    }
}

/// direction_id の優先順位。0 -> 未設定 -> それ以外。
fn direction_rank(direction_id: Option<i32>) -> u8 {
    match direction_id {
        Some(0) => 0,
        None => 1,
        _ => 2,
    }
}

/// `NULL` を最後に置く降順比較。
fn max_option(current: Option<f64>, candidate: Option<f64>) -> Option<f64> {
    match (current, candidate) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (Some(a), None) => Some(a),
        (None, b) => b,
    }
}

/// 距離の降順比較。`None` は最後。
fn dist_cmp(a: Option<f64>, b: Option<f64>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(a), Some(b)) => b.total_cmp(&a),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

fn shape_is_better(candidate: &ShapeRank, current: &ShapeRank) -> bool {
    current
        .0
        .cmp(&candidate.0)
        .then_with(|| dist_cmp(candidate.1, current.1))
        .then_with(|| candidate.2.cmp(&current.2))
        .then_with(|| candidate.3.cmp(current.3))
        .is_lt()
}

fn trip_is_better(candidate: &TripRank, current: &TripRank) -> bool {
    candidate
        .0
        .cmp(&current.0)
        .then_with(|| candidate.1.cmp(&current.1))
        .then_with(|| current.2.cmp(&candidate.2))
        .then_with(|| dist_cmp(candidate.3, current.3))
        .then_with(|| current.4.cmp(&candidate.4))
        .then_with(|| candidate.5.cmp(current.5))
        .is_lt()
}

/// 停留所を stations へ。物理停留所 1 件につき、通る系統の数だけ行を作る。
fn stops_to_stations(dataset: &mut Dataset, gtfs: &GtfsData, stop_route_map: &StopRouteMap) {
    // 親を持たない停留所だけが物理的な停留所。子は同じ停留所の別の乗り場。
    let stops: Vec<&Stop> = gtfs
        .stops
        .iter()
        .filter(|stop| stop.parent_station.is_none())
        .collect();
    // 誰かの親になっている停留所は、それ自体が 1 つの物理停留所として
    // 重複解消済みなので、まとめ処理から外す。
    let parent_ids: HashSet<&str> = gtfs
        .stops
        .iter()
        .filter_map(|stop| stop.parent_station.as_deref())
        .collect();

    let group_ids = build_station_g_cd_map(&stops, &parent_ids);
    let groups = group_ids.values().copied().collect::<HashSet<_>>().len();
    info!("バス停 {} 件を {groups} グループにまとめた", stops.len());

    let mut added = 0usize;
    for stop in &stops {
        let Some(routes) = stop_route_map.get(&stop.stop_id) else {
            continue; // どの系統にも乗らない停留所は取り込まない
        };
        let station_g_cd = group_ids
            .get(stop.stop_id.as_str())
            .copied()
            .unwrap_or_else(|| bus_station_g_cd(&stop.stop_id));
        // station_name_r はマクロン付き (Tōkyō)、station_name_rn は ASCII (Tokyo)。
        // 鉄道側と同じ使い分けにする。
        let station_name_rn = stop.stop_name_r.as_deref().map(strip_macrons);
        let station_name_k = stop
            .stop_name_k
            .as_deref()
            .map(hiragana_to_katakana)
            .unwrap_or_else(|| stop.stop_name.clone());

        for (route_id, stop_sequence) in routes {
            let mut row = dataset.stations.blank_row();
            let t = &dataset.stations;
            set(
                t,
                &mut row,
                "station_cd",
                int(bus_station_cd(&stop.stop_id, route_id)),
            );
            set(t, &mut row, "station_g_cd", int(station_g_cd));
            set(t, &mut row, "station_name", text(stop.stop_name.clone()));
            set(t, &mut row, "station_name_k", text(station_name_k.clone()));
            set(t, &mut row, "station_name_r", stop.stop_name_r.clone());
            set(t, &mut row, "station_name_rn", station_name_rn.clone());
            set(t, &mut row, "station_name_zh", stop.stop_name_zh.clone());
            set(t, &mut row, "station_name_ko", stop.stop_name_ko.clone());
            set(t, &mut row, "line_cd", int(bus_line_cd(route_id)));
            set(t, &mut row, "pref_cd", int(13));
            set(t, &mut row, "post", text(""));
            set(t, &mut row, "address", text(""));
            set(t, &mut row, "lon", text(stop.stop_lon.to_string()));
            set(t, &mut row, "lat", text(stop.stop_lat.to_string()));
            set(t, &mut row, "open_ymd", text(""));
            set(t, &mut row, "close_ymd", text(""));
            set(t, &mut row, "e_status", int(0));
            set(t, &mut row, "e_sort", int(*stop_sequence));
            set(t, &mut row, "transport_type", int(1));
            if dataset.stations.push(row) {
                added += 1;
            }
        }
    }
    info!("バス停 {} 件から駅 {added} 行を作った", stops.len());
}

/// 同じ物理停留所の上下線ポールをひとつの `station_g_cd` へまとめる。
///
/// 親子関係を持つフィード (都営) は既に 1 停留所 1 レコードなので、
/// 親として参照されている停留所はまとめない。それ以外のフィード
/// (西武・京王・東急コミュニティ) はポールごとに別レコードなので、
/// 名前が同じで近い停留所を 1 つにする。
///
/// まとめ方は決定的な貪欲法。名前ごとに `stop_id` 順に見て、
/// 「クラスタの全員が半径内にある」場合だけ加える。こうするとクラスタの
/// 直径が半径を超えず、遠くの同名停留所まで芋づる式に飲み込まない。
fn build_station_g_cd_map(stops: &[&Stop], parent_ids: &HashSet<&str>) -> HashMap<String, i32> {
    let mut map: HashMap<String, i32> = HashMap::with_capacity(stops.len());
    let mut by_name: HashMap<&str, Vec<usize>> = HashMap::new();
    for (index, stop) in stops.iter().enumerate() {
        if parent_ids.contains(stop.stop_id.as_str()) {
            map.insert(stop.stop_id.clone(), bus_station_g_cd(&stop.stop_id));
            continue;
        }
        by_name
            .entry(stop.stop_name.trim())
            .or_default()
            .push(index);
    }

    for indices in by_name.values() {
        let mut order = indices.clone();
        order.sort_by(|a, b| stops[*a].stop_id.cmp(&stops[*b].stop_id));

        let mut clusters: Vec<Vec<usize>> = Vec::new();
        for index in &order {
            let stop = stops[*index];
            let joined = clusters.iter_mut().find(|cluster| {
                cluster.iter().all(|other| {
                    let other = stops[*other];
                    haversine_distance(stop.stop_lat, stop.stop_lon, other.stop_lat, other.stop_lon)
                        <= BUS_STOP_GROUPING_RADIUS_METERS
                })
            });
            match joined {
                Some(cluster) => cluster.push(*index),
                None => clusters.push(vec![*index]),
            }
        }

        for cluster in &clusters {
            // 代表は stop_id が最小のもの。結果を安定させるため。
            let representative = cluster
                .iter()
                .map(|index| stops[*index].stop_id.as_str())
                .min()
                .expect("クラスタは空にならない");
            let g_cd = bus_station_g_cd(representative);
            for index in cluster {
                map.insert(stops[*index].stop_id.clone(), g_cd);
            }
        }
    }
    map
}

/// 1 つの (系統, shape) を 1 つの列車種別として登録する。
struct Variation<'a> {
    route_id: &'a str,
    shape_id: &'a str,
    trip_id: &'a str,
    direction_id: Option<i32>,
    trip_headsign: Option<&'a str>,
    route_short_name: Option<&'a str>,
    route_long_name: Option<&'a str>,
    route_long_name_r: Option<&'a str>,
    route_color: Option<&'a str>,
    stop_count: usize,
    first_stop_name: Option<&'a str>,
    first_stop_name_r: Option<&'a str>,
    last_stop_name: Option<&'a str>,
    last_stop_name_r: Option<&'a str>,
    is_loop: bool,
    /// 通る物理停留所を並べ替えて連結したもの。同じ集合の shape は
    /// 同じ路線の上下線とみなして 1 つの種別に畳む。
    stop_set_key: Option<String>,
}

/// (系統, shape) ごとに列車種別と停車駅を作る。
///
/// 1 つの shape が 1 つの運行パターンにあたる (池86 ならフルループ /
/// サンシャインシティ経由 / 短ターン)。鉄道の のぞみ / ひかり / こだま と
/// 同じように、クライアントはこれで運行パターンを切り替えられる。
fn trip_variations_to_types(dataset: &mut Dataset, gtfs: &GtfsData) {
    let stop_times_by_trip = gtfs.stop_times_by_trip();
    let parent_of = gtfs.parent_stop_ids();
    let stop_by_id: HashMap<&str, &Stop> = gtfs
        .stops
        .iter()
        .map(|stop| (stop.stop_id.as_str(), stop))
        .collect();
    let route_by_id: HashMap<&str, &super::model::Route> = gtfs
        .routes
        .iter()
        .map(|route| (route.route_id.as_str(), route))
        .collect();

    // (系統, shape) ごとに代表の便を 1 本。決着は trip_id の小さい順。
    let mut representative_trip: HashMap<(&str, &str), &super::model::Trip> = HashMap::new();
    for trip in &gtfs.trips {
        let Some(shape_id) = trip.shape_id.as_deref() else {
            continue;
        };
        representative_trip
            .entry((trip.route_id.as_str(), shape_id))
            .and_modify(|current| {
                if trip.trip_id < current.trip_id {
                    *current = trip;
                }
            })
            .or_insert(trip);
    }

    let mut variations: Vec<Variation> = Vec::new();
    for ((route_id, shape_id), trip) in &representative_trip {
        let Some(route) = route_by_id.get(route_id) else {
            continue;
        };
        let empty = Vec::new();
        let times = stop_times_by_trip
            .get(trip.trip_id.as_str())
            .unwrap_or(&empty);
        let first = times.first();
        let last = times.last();
        let name_of = |stop_id: &str| stop_by_id.get(stop_id).map(|stop| stop.stop_name.as_str());
        let roman_of = |stop_id: &str| {
            stop_by_id
                .get(stop_id)
                .and_then(|stop| stop.stop_name_r.as_deref())
        };
        let parent_of_time =
            |time: &&super::model::StopTime| parent_of.get(time.stop_id.as_str()).copied();

        let mut parents: Vec<&str> = times.iter().filter_map(parent_of_time).collect();
        parents.sort();
        parents.dedup();
        let stop_set_key = if parents.is_empty() {
            None
        } else {
            Some(parents.join("|"))
        };

        let first_parent = first.and_then(parent_of_time);
        let last_parent = last.and_then(parent_of_time);

        variations.push(Variation {
            route_id,
            shape_id,
            trip_id: trip.trip_id.as_str(),
            direction_id: trip.direction_id,
            trip_headsign: trip.trip_headsign.as_deref(),
            route_short_name: route.route_short_name.as_deref(),
            route_long_name: route.route_long_name.as_deref(),
            route_long_name_r: route.route_long_name_r.as_deref(),
            route_color: route.route_color.as_deref(),
            stop_count: times.len(),
            first_stop_name: first.and_then(|time| name_of(&time.stop_id)),
            first_stop_name_r: first.and_then(|time| roman_of(&time.stop_id)),
            last_stop_name: last.and_then(|time| name_of(&time.stop_id)),
            last_stop_name_r: last.and_then(|time| roman_of(&time.stop_id)),
            is_loop: first_parent.is_some() && first_parent == last_parent,
            stop_set_key,
        });
    }

    // 停留所数の多い shape が接尾辞なしの名前を取れるよう、この順で並べる。
    variations.sort_by(|a, b| {
        a.route_id
            .cmp(b.route_id)
            .then(b.stop_count.cmp(&a.stop_count))
            .then(a.shape_id.cmp(b.shape_id))
    });
    info!("バスの運行パターン {} 件 (畳む前)", variations.len());

    // 同じ系統内で同じ停留所集合を通る shape は、上下線とみなして 1 つに畳む。
    // 並びは stop_count の降順なので、各グループの先頭が代表になる。
    let mut group_of: HashMap<(&str, String), usize> = HashMap::new();
    let mut directions: Vec<Vec<Option<i32>>> = Vec::new();
    let mut representatives: Vec<&Variation> = Vec::new();
    for variation in &variations {
        let key = (
            variation.route_id,
            variation
                .stop_set_key
                .clone()
                .unwrap_or_else(|| format!("__shape:{}", variation.shape_id)),
        );
        match group_of.get(&key) {
            Some(index) => directions[*index].push(variation.direction_id),
            None => {
                group_of.insert(key, representatives.len());
                directions.push(vec![variation.direction_id]);
                representatives.push(variation);
            }
        }
    }
    info!("畳んだ結果 {} 種別", representatives.len());

    let via = pick_via_stops(
        &representatives,
        &stop_times_by_trip,
        &parent_of,
        &stop_by_id,
    );

    // 同じ名前になった種別は停留所数を付けて区別する。
    let mut name_counter: HashMap<(&str, String), i32> = HashMap::new();
    let mut sst_added = 0usize;
    for (index, variation) in representatives.iter().enumerate() {
        let type_cd = bus_type_cd(variation.route_id, variation.shape_id);
        let line_group_cd = bus_line_group_cd(variation.route_id, variation.shape_id);

        let is_bidirectional = directions[index].len() > 1;
        let direction = if is_bidirectional {
            0 // 上下がまとまったので Both
        } else {
            variation.direction_id.unwrap_or(0)
        };

        let base_name = build_type_name(variation, is_bidirectional, via.get(&index));
        let base_name_r = build_type_name_roman(variation, is_bidirectional, via.get(&index));

        let counter = name_counter
            .entry((variation.route_id, base_name.clone()))
            .or_insert(0);
        *counter += 1;
        let need_suffix = *counter > 1;
        let type_name = if need_suffix {
            format!("{base_name} ({}停)", variation.stop_count)
        } else {
            base_name
        };
        let type_name_r = match base_name_r {
            Some(roman) if need_suffix => format!("{roman} ({} stops)", variation.stop_count),
            Some(roman) => roman,
            None => String::new(),
        };

        let color = variation
            .route_color
            .map(|color| {
                if color.starts_with('#') {
                    color.to_string()
                } else {
                    format!("#{color}")
                }
            })
            .unwrap_or_else(|| "#000000".to_string());

        let mut row = dataset.types.blank_row();
        let t = &dataset.types;
        set(t, &mut row, "type_cd", int(type_cd));
        set(t, &mut row, "type_name", text(type_name.clone()));
        set(t, &mut row, "type_name_k", text(type_name));
        set(t, &mut row, "type_name_r", text(type_name_r));
        set(t, &mut row, "type_name_zh", text(""));
        set(t, &mut row, "type_name_ko", text(""));
        set(t, &mut row, "color", text(color));
        set(t, &mut row, "direction", int(direction));
        set(t, &mut row, "kind", int(BUS_ROUTE_KIND));
        set(t, &mut row, "priority", int(0));
        dataset.types.push(row);

        // 代表便の停車順に station_station_types を入れる。id が停車順序として
        // 読まれるため、並べてから入れること。
        let empty = Vec::new();
        let times = stop_times_by_trip.get(variation.trip_id).unwrap_or(&empty);
        let mut first_seq: HashMap<&str, i32> = HashMap::new();
        for time in times {
            let Some(parent) = parent_of.get(time.stop_id.as_str()).copied() else {
                continue;
            };
            first_seq
                .entry(parent)
                .and_modify(|seq| *seq = (*seq).min(time.stop_sequence))
                .or_insert(time.stop_sequence);
        }
        let mut stops: Vec<(&str, i32)> = first_seq.into_iter().collect();
        stops.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(b.0)));

        for (parent, _) in &stops {
            let mut row = dataset.sst.blank_row();
            let t = &dataset.sst;
            set(
                t,
                &mut row,
                "station_cd",
                int(bus_station_cd(parent, variation.route_id)),
            );
            set(t, &mut row, "type_cd", int(type_cd));
            set(t, &mut row, "line_group_cd", int(line_group_cd));
            set(t, &mut row, "pass", int(0));
            dataset.sst.push(row);
            sst_added += 1;
        }
    }

    info!(
        "バスの運行パターン {} 件を種別として取り込んだ (station_station_types {sst_added} 行)",
        representatives.len()
    );
}

/// 循環系統の名前に使う「経由地」を選ぶ。
///
/// 循環は始点と終点が同じなので、行先表示だけでは区別できない。同じ行先を持つ
/// 兄弟 shape が通らない停留所を優先し、無ければ行程の中間点を使う。
fn pick_via_stops<'a>(
    representatives: &[&'a Variation<'a>],
    stop_times_by_trip: &HashMap<&str, Vec<&super::model::StopTime>>,
    parent_of: &HashMap<&'a str, &'a str>,
    stop_by_id: &HashMap<&'a str, &'a Stop>,
) -> HashMap<usize, (String, Option<String>)> {
    // 循環する代表便の停車順 (物理停留所で重複解消済み)。
    let mut stops_per_trip: HashMap<usize, Vec<LoopStop>> = HashMap::new();
    for (index, variation) in representatives.iter().enumerate() {
        if !variation.is_loop {
            continue;
        }
        let Some(times) = stop_times_by_trip.get(variation.trip_id) else {
            continue;
        };
        let mut seen: HashSet<&str> = HashSet::new();
        let mut stops = Vec::new();
        for time in times {
            let Some(parent) = parent_of.get(time.stop_id.as_str()).copied() else {
                continue;
            };
            if !seen.insert(parent) {
                continue;
            }
            let Some(stop) = stop_by_id.get(time.stop_id.as_str()) else {
                continue;
            };
            stops.push((parent, stop.stop_name.as_str(), stop.stop_name_r.as_deref()));
        }
        stops_per_trip.insert(index, stops);
    }

    // (系統, 行先) ごとに兄弟 shape を集める。
    let mut loop_groups: HashMap<(&str, String), Vec<usize>> = HashMap::new();
    for (index, variation) in representatives.iter().enumerate() {
        if !variation.is_loop {
            continue;
        }
        if let Some(headsign) = headsign_of(variation) {
            loop_groups
                .entry((variation.route_id, headsign))
                .or_default()
                .push(index);
        }
    }

    let mut via = HashMap::new();
    for ((_, headsign), indices) in &loop_groups {
        for (position, index) in indices.iter().enumerate() {
            let Some(stops) = stops_per_trip.get(index).filter(|stops| stops.len() >= 3) else {
                continue;
            };

            // 兄弟 shape がどれも通らない停留所。
            let unique: Vec<&LoopStop> = stops
                .iter()
                .filter(|(parent, _, _)| {
                    indices.iter().enumerate().all(|(other_position, other)| {
                        if other_position == position {
                            return true;
                        }
                        stops_per_trip
                            .get(other)
                            .map(|other| other.iter().all(|(p, _, _)| p != parent))
                            .unwrap_or(true)
                    })
                })
                .collect();

            let middle = stops.len() / 2;
            let picked = if unique.is_empty() {
                let (_, name, roman) = stops[middle];
                Some((name, roman))
            } else {
                // 中間点に最も近いものを選ぶ。
                unique
                    .iter()
                    .min_by_key(|(parent, _, _)| {
                        let position =
                            stops.iter().position(|(p, _, _)| p == parent).unwrap_or(0) as i64;
                        (position - middle as i64).abs()
                    })
                    .map(|(_, name, roman)| (*name, *roman))
            };

            if let Some((name, roman)) = picked {
                // 行先と同じ名前では情報が増えないので使わない。
                if name != headsign {
                    via.insert(
                        *index,
                        (
                            name.to_string(),
                            roman.filter(|s| !s.is_empty()).map(str::to_string),
                        ),
                    );
                }
            }
        }
    }
    via
}

/// 行先の決め方。行先表示 -> 路線長名 -> 路線短名 -> 始発停留所名。
fn headsign_of(variation: &Variation) -> Option<String> {
    variation
        .trip_headsign
        .filter(|s| !s.is_empty())
        .or(variation.route_long_name.filter(|s| !s.is_empty()))
        .or(variation.route_short_name.filter(|s| !s.is_empty()))
        .or(variation.first_stop_name.filter(|s| !s.is_empty()))
        .map(str::to_string)
}

/// 種別名を組み立てる。
///
/// - 上下ペア: `<始発> ⇔ <行先>`
/// - 循環: `<行先>（<経由地>経由・循環）` (経由地が取れなければ `<行先> (循環)`)
/// - それ以外で始発と行先が違う: `<始発> → <行先>`
/// - どちらかしか無い: あるほう
fn build_type_name(
    variation: &Variation,
    is_bidirectional: bool,
    via: Option<&(String, Option<String>)>,
) -> String {
    let headsign = variation
        .trip_headsign
        .filter(|s| !s.is_empty())
        .or(variation.route_long_name.filter(|s| !s.is_empty()))
        .or(variation.route_short_name.filter(|s| !s.is_empty()));
    let first_stop = variation.first_stop_name.filter(|s| !s.is_empty());

    let loop_name = |label: &str| match via {
        Some((name, _)) => format!("{label}（{name}経由・循環）"),
        None => format!("{label} (循環)"),
    };

    if is_bidirectional && !variation.is_loop {
        return match (first_stop, headsign) {
            (Some(first), Some(head)) if first != head => format!("{first} ⇔ {head}"),
            (_, Some(head)) => head.to_string(),
            (Some(first), None) => first.to_string(),
            _ => format!("shape {}", variation.shape_id),
        };
    }

    match (variation.is_loop, first_stop, headsign) {
        (true, _, Some(head)) => loop_name(head),
        (true, Some(first), None) => loop_name(first),
        (false, Some(first), Some(head)) if first != head => format!("{first} → {head}"),
        (_, _, Some(head)) => head.to_string(),
        (_, Some(first), None) => first.to_string(),
        _ => format!("shape {}", variation.shape_id),
    }
}

/// 種別名のローマ字版。日本語版と同じ分岐をたどる。
///
/// 途中で必要なローマ字が欠けた場合は `None` を返す。日本語混じりの
/// 中途半端なローマ字名を作るより、空にしたほうがクライアントで扱いやすい。
fn build_type_name_roman(
    variation: &Variation,
    is_bidirectional: bool,
    via: Option<&(String, Option<String>)>,
) -> Option<String> {
    // trip_headsign にローマ字の対応列は無い。行先が終点の停留所名と一致すれば
    // その停留所のローマ字を借り、路線長名から来ていれば路線のローマ字を使う。
    let headsign_r: Option<&str> = {
        let japanese = variation.trip_headsign.filter(|s| !s.is_empty());
        let last_r = variation.last_stop_name_r.filter(|s| !s.is_empty());
        let first_r = variation.first_stop_name_r.filter(|s| !s.is_empty());
        match (japanese, variation.last_stop_name, last_r) {
            (Some(head), Some(last), Some(roman)) if head == last => Some(roman),
            _ => {
                if variation.is_loop && japanese.is_some() {
                    // 循環は始発 == 終点なので、始発のローマ字で代用できる。
                    first_r
                } else if japanese.is_none() && variation.route_long_name.is_some() {
                    variation.route_long_name_r.filter(|s| !s.is_empty())
                } else {
                    None
                }
            }
        }
    };
    let first_stop_r = variation.first_stop_name_r.filter(|s| !s.is_empty());

    let loop_name_r = |label: &str| -> Option<String> {
        if label.is_empty() {
            return None;
        }
        match via {
            Some((_, Some(roman))) => Some(format!("{label} via {roman} (Loop)")),
            // 経由地はあるがローマ字が無い場合は、名前ごと諦める。
            Some((_, None)) => None,
            None => Some(format!("{label} (Loop)")),
        }
    };

    if is_bidirectional && !variation.is_loop {
        return match (first_stop_r, headsign_r) {
            (Some(first), Some(head)) if first != head => Some(format!("{first} ⇔ {head}")),
            (_, Some(head)) => Some(head.to_string()),
            (Some(first), None) => Some(first.to_string()),
            _ => None,
        };
    }

    match (variation.is_loop, first_stop_r, headsign_r) {
        (true, _, Some(head)) => loop_name_r(head),
        (true, Some(first), None) => loop_name_r(first),
        (false, Some(first), Some(head)) if first != head => Some(format!("{first} → {head}")),
        (_, _, Some(head)) => Some(head.to_string()),
        (_, Some(first), None) => Some(first.to_string()),
        _ => None,
    }
}

/// 列名を指定して 1 セルを埋める。
fn set(table: &crate::table::Table, row: &mut [Cell], column: &str, value: Cell) {
    row[table.col(column)] = value;
}
