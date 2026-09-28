//! 国土数値情報の鉄道データ (N02) の線路区間を、駅間の線路の長さを測るための
//! 無向グラフにする。
//!
//! 線路区間は LineString の集まりで、端点を共有する区間どうしがつながっている。
//! 頂点を座標で同一視してグラフにし、2 駅をそれぞれ近くの線路へ寄せて、その間の
//! 最短経路の長さを線路の長さとする。

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

use stationapi::domain::arrival_estimation::haversine_distance;

/// 駅を線路へ寄せるときに見る範囲 (メートル)。
///
/// 駅の座標は駅舎に置かれていることが多く、線路から数百メートル離れることがある
/// (湘南新宿ラインの武蔵小杉は南武線側の座標で、品鶴線から約 350m)。
pub const SNAP_RADIUS_METERS: f64 = 500.0;

/// 駅から線路までの距離に掛ける重み。
///
/// 範囲内の線路をすべて候補にして、「寄せた距離 × この重み + 線路上の距離」が
/// 最小になる組を選ぶ。最も近い線路だけに寄せると、大きな駅で同じ事業者の別路線
/// (本町の御堂筋線と四つ橋線など) に寄ってしまい、乗換駅をまわる遠回りが出る。
/// 重みを 1 以下にすると、相手の駅の近くまで寄せた方が安くなり、線路の長さが
/// 実際より短く出る。
const SNAP_PENALTY: f64 = 2.0;

/// 空間索引の升目 (度)。緯度 45 度でも経度方向に約 780m あり
/// [`SNAP_RADIUS_METERS`] より大きいので、周囲 9 升を見れば取りこぼさない。
const CELL_DEGREES: f64 = 0.01;

/// 頂点を同一視する座標の桁。N02 の座標は小数第 5 位 (約 1m) までのものが多い。
const VERTEX_SCALE: f64 = 1e5;

/// N02 の線路区間 1 本。
pub struct Section {
    /// 事業者名 (`N02_004`)。
    pub operator: String,
    /// `[経度, 緯度]` の並び。
    pub coordinates: Vec<[f64; 2]>,
}

struct Edge {
    a: u32,
    b: u32,
    operator: u16,
    length: f64,
}

/// 駅を線路へ寄せた位置。
struct Snap {
    edge: u32,
    /// 辺の上の位置。0 が `a`、1 が `b`。
    t: f64,
    /// 駅から寄せた位置までの距離 (メートル)。
    offset: f64,
}

pub struct RailNetwork {
    lat: Vec<f64>,
    lon: Vec<f64>,
    edges: Vec<Edge>,
    /// 頂点 -> 接する辺 (CSR)。頂点 `v` の辺は `adj[adj_start[v]..adj_start[v + 1]]`。
    adj_start: Vec<usize>,
    adj: Vec<u32>,
    /// 升目 -> その升目にかかる辺。
    grid: HashMap<(i32, i32), Vec<u32>>,
    operators: HashMap<String, u16>,
}

impl RailNetwork {
    pub fn build(sections: &[Section]) -> Self {
        let mut vertex_of: HashMap<(i64, i64), u32> = HashMap::new();
        let mut lat: Vec<f64> = Vec::new();
        let mut lon: Vec<f64> = Vec::new();
        let mut edges: Vec<Edge> = Vec::new();
        let mut operators: HashMap<String, u16> = HashMap::new();

        for section in sections {
            let next_operator = operators.len() as u16;
            let operator = *operators
                .entry(section.operator.clone())
                .or_insert(next_operator);
            let mut previous: Option<u32> = None;
            for &[x, y] in &section.coordinates {
                let key = (
                    (x * VERTEX_SCALE).round() as i64,
                    (y * VERTEX_SCALE).round() as i64,
                );
                let vertex = *vertex_of.entry(key).or_insert_with(|| {
                    lat.push(y);
                    lon.push(x);
                    (lat.len() - 1) as u32
                });
                if let Some(prev) = previous.filter(|&prev| prev != vertex) {
                    let length = haversine_distance(
                        lat[prev as usize],
                        lon[prev as usize],
                        lat[vertex as usize],
                        lon[vertex as usize],
                    );
                    edges.push(Edge {
                        a: prev,
                        b: vertex,
                        operator,
                        length,
                    });
                }
                previous = Some(vertex);
            }
        }

        let mut adj_start = vec![0usize; lat.len() + 1];
        for edge in &edges {
            adj_start[edge.a as usize + 1] += 1;
            adj_start[edge.b as usize + 1] += 1;
        }
        for i in 1..adj_start.len() {
            adj_start[i] += adj_start[i - 1];
        }
        let mut fill = adj_start.clone();
        let mut adj = vec![0u32; adj_start[lat.len()]];
        for (i, edge) in edges.iter().enumerate() {
            for v in [edge.a, edge.b] {
                adj[fill[v as usize]] = i as u32;
                fill[v as usize] += 1;
            }
        }

        let mut grid: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
        for (i, edge) in edges.iter().enumerate() {
            let (a, b) = (edge.a as usize, edge.b as usize);
            let (y0, y1) = (cell(lat[a].min(lat[b])), cell(lat[a].max(lat[b])));
            let (x0, x1) = (cell(lon[a].min(lon[b])), cell(lon[a].max(lon[b])));
            for y in y0..=y1 {
                for x in x0..=x1 {
                    grid.entry((y, x)).or_default().push(i as u32);
                }
            }
        }

        RailNetwork {
            lat,
            lon,
            edges,
            adj_start,
            adj,
            grid,
            operators,
        }
    }

    pub fn vertex_count(&self) -> usize {
        self.lat.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// 事業者名から事業者の番号を引く。N02 に無い事業者は `None`。
    pub fn operator_id(&self, name: &str) -> Option<u16> {
        self.operators.get(name).copied()
    }

    /// `from` と `to` のあいだの線路の長さ (メートル)。
    ///
    /// `operators` を渡すと、その事業者の線路だけを使う。線路上の距離が
    /// `max_length` を超える場合と、どちらかの駅の近くに線路が無い場合は `None`。
    pub fn track_length(
        &self,
        from: (f64, f64),
        to: (f64, f64),
        operators: Option<&HashSet<u16>>,
        max_length: f64,
    ) -> Option<f64> {
        let origin = self.snaps(from, operators);
        let destination = self.snaps(to, operators);
        if origin.is_empty() || destination.is_empty() {
            return None;
        }

        // (費用, 線路上の距離)。費用は寄せた距離の重みを含み、経路の選択にだけ使う。
        let mut best: Option<(f64, f64)> = None;

        // 同じ辺に寄せた場合は、辺の上の位置の差がそのまま長さになる。
        for o in &origin {
            for d in destination.iter().filter(|d| d.edge == o.edge) {
                let length = (o.t - d.t).abs() * self.edges[o.edge as usize].length;
                improve(
                    &mut best,
                    SNAP_PENALTY * (o.offset + d.offset) + length,
                    length,
                );
            }
        }

        let starts = self.snap_ends(&origin);
        let goals = self.snap_ends(&destination);

        let mut dist: HashMap<u32, f64> = HashMap::new();
        let mut heap = BinaryHeap::new();
        for (&vertex, &(cost, length)) in &starts {
            dist.insert(vertex, cost);
            heap.push(Label {
                cost,
                length,
                vertex,
            });
        }

        let cost_limit = max_length + 2.0 * SNAP_PENALTY * SNAP_RADIUS_METERS;
        while let Some(Label {
            cost,
            length,
            vertex,
        }) = heap.pop()
        {
            if dist.get(&vertex).is_some_and(|&d| cost > d) {
                continue;
            }
            if cost > cost_limit || best.is_some_and(|(best_cost, _)| cost >= best_cost) {
                break;
            }
            if let Some(&(goal_cost, goal_length)) = goals.get(&vertex) {
                improve(&mut best, cost + goal_cost, length + goal_length);
            }
            let v = vertex as usize;
            for &e in &self.adj[self.adj_start[v]..self.adj_start[v + 1]] {
                let edge = &self.edges[e as usize];
                if operators.is_some_and(|ops| !ops.contains(&edge.operator)) {
                    continue;
                }
                let next = if edge.a == vertex { edge.b } else { edge.a };
                let next_cost = cost + edge.length;
                if dist.get(&next).is_none_or(|&d| next_cost < d) {
                    dist.insert(next, next_cost);
                    heap.push(Label {
                        cost: next_cost,
                        length: length + edge.length,
                        vertex: next,
                    });
                }
            }
        }

        best.map(|(_, length)| length)
            .filter(|&length| length <= max_length)
    }

    /// 駅から [`SNAP_RADIUS_METERS`] 以内にある辺と、その辺の上で最も駅に近い位置。
    fn snaps(&self, (lat, lon): (f64, f64), operators: Option<&HashSet<u16>>) -> Vec<Snap> {
        let (y0, x0) = (cell(lat), cell(lon));
        let mut seen: HashSet<u32> = HashSet::new();
        let mut out = Vec::new();
        for y in y0 - 1..=y0 + 1 {
            for x in x0 - 1..=x0 + 1 {
                let Some(edges) = self.grid.get(&(y, x)) else {
                    continue;
                };
                for &e in edges {
                    if !seen.insert(e) {
                        continue;
                    }
                    if operators.is_some_and(|ops| !ops.contains(&self.edges[e as usize].operator))
                    {
                        continue;
                    }
                    let (offset, t) = self.project(lat, lon, e);
                    if offset <= SNAP_RADIUS_METERS {
                        out.push(Snap { edge: e, t, offset });
                    }
                }
            }
        }
        out
    }

    /// 寄せた位置から辺の両端までを、端点ごとに最も安いものだけ残す。
    /// 値は (費用, 線路上の距離)。
    fn snap_ends(&self, snaps: &[Snap]) -> HashMap<u32, (f64, f64)> {
        let mut ends: HashMap<u32, (f64, f64)> = HashMap::new();
        for snap in snaps {
            let edge = &self.edges[snap.edge as usize];
            for (vertex, length) in [
                (edge.a, snap.t * edge.length),
                (edge.b, (1.0 - snap.t) * edge.length),
            ] {
                let cost = SNAP_PENALTY * snap.offset + length;
                let entry = ends.entry(vertex).or_insert((f64::INFINITY, 0.0));
                if cost < entry.0 {
                    *entry = (cost, length);
                }
            }
        }
        ends
    }

    /// 駅を辺へ垂直に下ろした位置。駅間程度の範囲なので、経度を緯度の余弦で縮めた
    /// 平面で近似する。返り値は (駅からの距離 (メートル), 辺の上の位置)。
    fn project(&self, lat: f64, lon: f64, e: u32) -> (f64, f64) {
        let edge = &self.edges[e as usize];
        let (a, b) = (edge.a as usize, edge.b as usize);
        let k = lat.to_radians().cos();
        let (ax, ay) = (self.lon[a] * k, self.lat[a]);
        let (dx, dy) = (self.lon[b] * k - ax, self.lat[b] - ay);
        let len2 = dx * dx + dy * dy;
        let t = if len2 == 0.0 {
            0.0
        } else {
            (((lon * k - ax) * dx + (lat - ay) * dy) / len2).clamp(0.0, 1.0)
        };
        let (qlat, qlon) = (ay + t * dy, (ax + t * dx) / k);
        (haversine_distance(lat, lon, qlat, qlon), t)
    }
}

/// (費用, 線路上の距離) の候補を、費用が小さければ採る。
fn improve(best: &mut Option<(f64, f64)>, cost: f64, length: f64) {
    if best.is_none_or(|(best_cost, _)| cost < best_cost) {
        *best = Some((cost, length));
    }
}

fn cell(degrees: f64) -> i32 {
    (degrees / CELL_DEGREES).floor() as i32
}

/// Dijkstra の探索ラベル。`BinaryHeap` は最大ヒープなので費用の比較を逆にする。
/// 費用が同じときも順序が決まるよう、頂点と長さでも比べる。
struct Label {
    cost: f64,
    length: f64,
    vertex: u32,
}

impl PartialEq for Label {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Label {}

impl PartialOrd for Label {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Label {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .total_cmp(&self.cost)
            .then_with(|| other.vertex.cmp(&self.vertex))
            .then_with(|| other.length.total_cmp(&self.length))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section(operator: &str, coordinates: &[[f64; 2]]) -> Section {
        Section {
            operator: operator.to_string(),
            coordinates: coordinates.to_vec(),
        }
    }

    /// 経度 139.00 から東へ延びる線路。緯度 35 度で経度 0.01 度は約 911m。
    fn straight() -> Vec<Section> {
        vec![section(
            "A",
            &[
                [139.00, 35.0],
                [139.01, 35.0],
                [139.02, 35.0],
                [139.03, 35.0],
            ],
        )]
    }

    #[test]
    fn measures_along_the_track() {
        let network = RailNetwork::build(&straight());
        let length = network
            .track_length((35.0, 139.005), (35.0, 139.025), None, 10_000.0)
            .unwrap();
        let expected = haversine_distance(35.0, 139.005, 35.0, 139.025);
        assert!((length - expected).abs() < 1.0, "{length} vs {expected}");
    }

    #[test]
    fn follows_the_curve_instead_of_the_straight_line() {
        // 北へ 0.01 度 (約 1.1km) 迂回してから戻る線路。
        let sections = vec![section(
            "A",
            &[
                [139.00, 35.0],
                [139.00, 35.01],
                [139.01, 35.01],
                [139.01, 35.0],
            ],
        )];
        let network = RailNetwork::build(&sections);
        let length = network
            .track_length((35.0, 139.00), (35.0, 139.01), None, 10_000.0)
            .unwrap();
        let straight = haversine_distance(35.0, 139.00, 35.0, 139.01);
        assert!(length > 2.0 * straight, "{length} vs {straight}");
    }

    #[test]
    fn stations_far_from_any_track_have_no_length() {
        let network = RailNetwork::build(&straight());
        // 線路から北へ約 1.1km
        assert!(network
            .track_length((35.01, 139.005), (35.0, 139.025), None, 10_000.0)
            .is_none());
    }

    #[test]
    fn prefers_the_track_both_stations_share() {
        // 駅 X は線路 B のすぐ横にあるが、線路 A からも 200m ほどの位置にある。
        // B は Y の近くを通らず、A とは 3km 先でつながる。A に寄せて測るべき。
        let sections = vec![
            section("A", &[[139.000, 35.0], [139.010, 35.0]]),
            section(
                "B",
                &[
                    [139.000, 35.0018],
                    [139.030, 35.0018],
                    [139.030, 35.0],
                    [139.010, 35.0],
                ],
            ),
        ];
        let network = RailNetwork::build(&sections);
        let length = network
            .track_length((35.0018, 139.002), (35.0, 139.008), None, 10_000.0)
            .unwrap();
        let along_a = haversine_distance(35.0, 139.002, 35.0, 139.008);
        assert!((length - along_a).abs() < 1.0, "{length} vs {along_a}");
    }

    #[test]
    fn operator_filter_excludes_other_tracks() {
        let network = RailNetwork::build(&straight());
        let other: HashSet<u16> = [99].into_iter().collect();
        assert!(network
            .track_length((35.0, 139.005), (35.0, 139.025), Some(&other), 10_000.0)
            .is_none());
        let own: HashSet<u16> = [network.operator_id("A").unwrap()].into_iter().collect();
        assert!(network
            .track_length((35.0, 139.005), (35.0, 139.025), Some(&own), 10_000.0)
            .is_some());
    }

    #[test]
    fn gives_up_beyond_the_length_limit() {
        let network = RailNetwork::build(&straight());
        assert!(network
            .track_length((35.0, 139.0), (35.0, 139.03), None, 1_000.0)
            .is_none());
    }
}
