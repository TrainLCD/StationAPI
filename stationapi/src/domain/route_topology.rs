//! 乗換経路の到達判定に使う、所要時間を持たない系統網。
//!
//! [`crate::domain::route_search::RouteNetwork`] は所要時間の推定まで持つので、
//! 組み立てに全駅の `Station` と推定が要る (ネイティブで約 190ms)。行き先の
//! 検索 (`stationsByName`) で要るのは「どの系統がどの駅に止まるか」だけなので、
//! それだけを持つこの網を別に作る。`RouteNetwork` も内部に同じ網を持ち、
//! 系統の整え方 ([`trim_pattern`]) を共有するので、同じ系統から作れば同じ網になる。
//!
//! IO を持たない純粋ロジック。

use std::collections::{HashMap, HashSet};

use crate::domain::route_search::MAX_RIDES;

/// 系統の 1 駅。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RouteStop {
    pub station_cd: i32,
    pub station_group_id: u32,
    pub line_cd: i32,
    /// 乗降できる (通過ではない) か。
    pub stoppable: bool,
}

/// 系統の駅列を網に載せる形に整える。載せないなら偽を返す。
///
/// - 先頭駅が末尾にも重複格納された「閉じた」環状データは重複終端を除く
///   (estimate_route_arrival_times と同じ扱い)
/// - 乗降できる駅が 2 つ未満の系統は載せない
pub(crate) fn trim_pattern<T>(
    stops: &mut Vec<T>,
    station_cd: impl Fn(&T) -> i32,
    stoppable: impl Fn(&T) -> bool,
) -> bool {
    if stops.len() > 1 && station_cd(&stops[0]) == station_cd(&stops[stops.len() - 1]) {
        stops.pop();
    }
    stops.iter().filter(|stop| stoppable(stop)).count() >= 2
}

#[derive(Debug, PartialEq, Eq)]
struct TopologyPattern {
    /// 位置ごとの節点。環状でも一周ぶん。
    nodes: Vec<u32>,
    station_cds: Vec<i32>,
    line_cds: Vec<i32>,
    stoppable: Vec<bool>,
}

impl TopologyPattern {
    fn len(&self) -> usize {
        self.nodes.len()
    }
}

/// 所要時間を持たない系統網。
#[derive(Debug, Default, PartialEq, Eq)]
pub struct RouteTopology {
    patterns: Vec<TopologyPattern>,
    node_by_group: HashMap<u32, u32>,
    /// 節点 -> (パターン, 位置)。乗降できる位置だけ。
    stop_patterns: Vec<Vec<(u32, u32)>>,
    /// 節点が関節点か (取り除くと網が分かれるか)。[`Reachability::can_arrive`] が
    /// 厳密な判定を要る駅を見分けるのに使う。
    cut_nodes: Vec<bool>,
}

/// ある出発駅からの到達判定。[`RouteTopology::reachability`] で作る。
pub struct Reachability<'a> {
    topology: &'a RouteTopology,
    origin: Option<u32>,
    station_cds: HashSet<i32>,
}

impl Reachability<'_> {
    /// 駅 (`station_cd`、駅グループ `station_group_id`、路線 `line_cd`) に、その
    /// 路線の列車で乗車 [`MAX_RIDES`] 本以内に着けるか。
    ///
    /// 真なら、その路線を `via_line_id` にした [`RouteNetwork::search`](crate::domain::route_search::RouteNetwork::search) で経路が
    /// 見つかる。探索は目的地の駅グループで途中下車しないので、幅優先の到達判定
    /// だけでは「支線の根元の駅に、支線へ一度出てから戻って着く」ような経路を
    /// 数えてしまう (石橋阪大前に箕面線で着く、など)。これが起きるのは駅グループが
    /// 関節点のときだけなので、そのときに限り目的地で降りない探索で確かめる。
    pub fn can_arrive(&self, station_cd: i32, station_group_id: u32, line_cd: i32) -> bool {
        if !self.station_cds.contains(&station_cd) {
            return false;
        }
        let Some(&target) = self.topology.node_by_group.get(&station_group_id) else {
            return false;
        };
        // 出発駅グループ自身へは search が必ず 0 件を返す。到達集合には、別の駅から
        // 同じ系統に乗り直した分として出発駅グループの駅が入ることがある
        if self.origin == Some(target) {
            return false;
        }
        if !self.topology.cut_nodes[target as usize] {
            return true;
        }
        self.origin.is_some_and(|origin| {
            self.topology
                .arrives_without_stopover(origin, target, line_cd)
        })
    }
}

impl RouteTopology {
    /// 系統ごとの駅列 (運行順) から網を組み立てる。結果は `line_groups` の順序に
    /// 依存するので、呼び出し側は安定した順で渡すこと。
    pub fn build<I>(line_groups: I) -> Self
    where
        I: IntoIterator<Item = Vec<RouteStop>>,
    {
        let mut topology = RouteTopology::default();
        for stops in line_groups {
            topology.add_pattern(stops);
        }
        topology.finish();
        topology
    }

    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }

    /// 系統を 1 つ載せる。[`Self::finish`] を呼ぶまで到達判定には使えない。
    pub(crate) fn add_pattern(&mut self, mut stops: Vec<RouteStop>) {
        if !trim_pattern(&mut stops, |s| s.station_cd, |s| s.stoppable) {
            return;
        }
        let pattern_index = self.patterns.len() as u32;
        let mut nodes = Vec::with_capacity(stops.len());
        for (pos, stop) in stops.iter().enumerate() {
            let next = self.node_by_group.len() as u32;
            let node = *self
                .node_by_group
                .entry(stop.station_group_id)
                .or_insert(next);
            if node == next {
                self.stop_patterns.push(Vec::new());
            }
            if stop.stoppable {
                self.stop_patterns[node as usize].push((pattern_index, pos as u32));
            }
            nodes.push(node);
        }
        self.patterns.push(TopologyPattern {
            nodes,
            station_cds: stops.iter().map(|s| s.station_cd).collect(),
            line_cds: stops.iter().map(|s| s.line_cd).collect(),
            stoppable: stops.iter().map(|s| s.stoppable).collect(),
        });
    }

    /// 全系統を載せ終えたら呼ぶ。関節点を求める。
    pub(crate) fn finish(&mut self) {
        self.cut_nodes = self.find_cut_nodes();
    }

    /// 駅と系統を頂点、「系統がその駅に止まる」を辺とする二部グラフで、関節点に
    /// なる駅を求める (Tarjan、O(駅 + 系統 + 停車))。再帰すると長い系統で
    /// スタックが深くなるので、明示的なスタックで回す。
    fn find_cut_nodes(&self) -> Vec<bool> {
        let node_count = self.stop_patterns.len();
        let vertex_count = node_count + self.patterns.len();
        // 系統側の隣接 (乗降できる駅、重複なし)
        let pattern_nodes: Vec<Vec<u32>> = self
            .patterns
            .iter()
            .map(|pattern| {
                let mut nodes: Vec<u32> = (0..pattern.len())
                    .filter(|&stop| pattern.stoppable[stop])
                    .map(|stop| pattern.nodes[stop])
                    .collect();
                nodes.sort_unstable();
                nodes.dedup();
                nodes
            })
            .collect();
        let neighbor = |vertex: usize, index: usize| -> Option<usize> {
            if vertex < node_count {
                self.stop_patterns[vertex]
                    .get(index)
                    .map(|&(pattern, _)| node_count + pattern as usize)
            } else {
                pattern_nodes[vertex - node_count]
                    .get(index)
                    .map(|&node| node as usize)
            }
        };

        const UNVISITED: u32 = u32::MAX;
        let mut order = vec![UNVISITED; vertex_count];
        let mut low = vec![0u32; vertex_count];
        let mut cut = vec![false; node_count];
        let mut counter = 0u32;
        for root in 0..node_count {
            if order[root] != UNVISITED {
                continue;
            }
            order[root] = counter;
            low[root] = counter;
            counter += 1;
            let mut root_children = 0;
            // (頂点, 親, 次に見る隣接の添字)
            let mut stack: Vec<(usize, usize, usize)> = vec![(root, usize::MAX, 0)];
            while let Some(&mut (vertex, parent, ref mut next)) = stack.last_mut() {
                if let Some(child) = neighbor(vertex, *next) {
                    *next += 1;
                    if child == parent {
                        continue;
                    }
                    if order[child] == UNVISITED {
                        order[child] = counter;
                        low[child] = counter;
                        counter += 1;
                        if vertex == root {
                            root_children += 1;
                        }
                        stack.push((child, vertex, 0));
                    } else {
                        low[vertex] = low[vertex].min(order[child]);
                    }
                    continue;
                }
                stack.pop();
                if parent != usize::MAX {
                    low[parent] = low[parent].min(low[vertex]);
                    if parent != root && parent < node_count && low[vertex] >= order[parent] {
                        cut[parent] = true;
                    }
                }
            }
            cut[root] = root_children > 1;
        }
        cut
    }

    /// `origin` から、`target` の駅グループで途中下車せずに、`line_cd` の路線の
    /// 列車で `target` に着けるか。[`RouteNetwork::search`](crate::domain::route_search::RouteNetwork::search) と同じ制約の到達判定。
    fn arrives_without_stopover(&self, origin: u32, target: u32, line_cd: i32) -> bool {
        if origin == target {
            return false;
        }
        let mut reached_nodes = vec![false; self.stop_patterns.len()];
        let mut boarded_patterns = vec![false; self.patterns.len()];
        reached_nodes[origin as usize] = true;
        let mut frontier = vec![origin];
        for _ in 0..MAX_RIDES {
            let mut next = Vec::new();
            for &node in &frontier {
                for &(pattern_index, _) in &self.stop_patterns[node as usize] {
                    if std::mem::replace(&mut boarded_patterns[pattern_index as usize], true) {
                        continue;
                    }
                    let pattern = &self.patterns[pattern_index as usize];
                    for stop in 0..pattern.len() {
                        let stop_node = pattern.nodes[stop];
                        if !pattern.stoppable[stop] || stop_node == node {
                            continue;
                        }
                        if stop_node == target {
                            if pattern.line_cds[stop] == line_cd {
                                return true;
                            }
                            // 目的地では途中下車しない
                            continue;
                        }
                        if !std::mem::replace(&mut reached_nodes[stop_node as usize], true) {
                            next.push(stop_node);
                        }
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            frontier = next;
        }
        false
    }

    /// `from`(駅グループ ID)からの到達判定を作る。駅ごとの判定は
    /// [`Reachability::can_arrive`]。
    pub fn reachability(&self, from: u32) -> Reachability<'_> {
        Reachability {
            topology: self,
            origin: self.node_by_group.get(&from).copied(),
            station_cds: self.reachable_station_cds(from),
        }
    }

    /// `from` から乗車 [`MAX_RIDES`] 本以内で降りられる駅 (`station_cd`)。
    ///
    /// 駅は路線ごとの `station_cd` で返すので、「この駅にこの路線で着けるか」を
    /// 表す。所要時間は見ないので、系統を幅優先でたどるだけで済む。目的地で
    /// 途中下車する経路も数えるため、探索より広いことがある (厳密な判定は
    /// [`Reachability::can_arrive`])。
    pub(crate) fn reachable_station_cds(&self, from: u32) -> HashSet<i32> {
        let mut reachable = HashSet::new();
        let Some(&origin) = self.node_by_group.get(&from) else {
            return reachable;
        };
        // パターンに最初に乗った節点と、別の節点からも乗れたか。乗った駅で
        // そのまま降りることはできないので、乗車駅と同じ駅グループの駅は、別の
        // 節点からも乗れたときに初めて「降りられる駅」になる (日比谷線で中目黒に
        // 着いただけでは、東横線の中目黒に東横線で着いたことにならない)
        let mut first_board: Vec<Option<u32>> = vec![None; self.patterns.len()];
        let mut boarded_elsewhere = vec![false; self.patterns.len()];
        let mut reached_nodes = vec![false; self.stop_patterns.len()];
        reached_nodes[origin as usize] = true;
        let mut frontier = vec![origin];
        for _ in 0..MAX_RIDES {
            let mut next = Vec::new();
            for &node in &frontier {
                for &(pattern_index, _) in &self.stop_patterns[node as usize] {
                    let index = pattern_index as usize;
                    let pattern = &self.patterns[index];
                    // 初めて乗るなら乗車駅以外、別の節点から乗り直すなら最初の
                    // 乗車駅だけが新たに降りられる駅になる
                    let (boarded_at, first_time) = match first_board[index] {
                        None => {
                            first_board[index] = Some(node);
                            (node, true)
                        }
                        Some(first) if first != node && !boarded_elsewhere[index] => {
                            boarded_elsewhere[index] = true;
                            (first, false)
                        }
                        Some(_) => continue,
                    };
                    for stop in 0..pattern.len() {
                        let stop_node = pattern.nodes[stop];
                        if !pattern.stoppable[stop] || (stop_node == boarded_at) == first_time {
                            continue;
                        }
                        reachable.insert(pattern.station_cds[stop]);
                        if !std::mem::replace(&mut reached_nodes[stop_node as usize], true) {
                            next.push(stop_node);
                        }
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            frontier = next;
        }
        reachable
    }
}
