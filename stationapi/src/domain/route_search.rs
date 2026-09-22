//! 時刻表を持たない乗換経路探索。
//!
//! 系統(`line_group_cd`)ごとの停車駅列を「パターン」とし、駅グループ
//! (`station_g_cd`)を乗換の節点にして RAPTOR (Round-bAsed Public Transit
//! Optimized Router) で探索する。時刻表が無いので「どの列車に乗るか」は考えず、
//! 各パターンの駅間所要時間を [`crate::domain::arrival_estimation`] の推定値で
//! 与える(頻度ベースの RAPTOR)。
//!
//! - ラウンド k は「k 本目の列車に乗った時点」の最早到着を求める。k を 1 ずつ
//!   増やすので、各ラウンドの目的地到着は「乗車 k 本以内での最短」になり、
//!   所要時間と乗換回数のパレート最適解がそのまま得られる。
//! - 列車に乗るたびに、種別ごとの平均待ち時間 [`boarding_wait_seconds`] を
//!   評価値に加える。時刻表が無いと本数の少ない特急が「直通で速い」ことになり、
//!   山手線より成田エクスプレスを勧めてしまうため。乗換にはさらに乗換通路の
//!   徒歩 [`TRANSFER_WALK_SECONDS`] を加える。同じ駅グループ内でのみ乗り換え、
//!   駅グループを跨ぐ徒歩連絡は扱わない。
//! - 1 回の探索で走査するのは「前ラウンドで到着が改善した駅を通るパターン」
//!   だけで、計算量は O(乗車回数 × 触れたパターンの駅数)。駅と路線の全組み合わせを
//!   列挙しない。
//!
//! パレート解は最短と最少乗換しか含まないため、代替経路は「見つかった経路の
//! 区間を 1 つ選び、その区間の並行系統(快速/各停など)に乗ることを禁止して
//! 再探索する」ことで集める。
//!
//! IO を持たない純粋ロジックなので、すべて単体テストできる。

use std::collections::{HashMap, HashSet, VecDeque};

use crate::domain::arrival_estimation::{
    estimate_arrival_minutes_calibrated, is_circular_route, EstimationParams,
};
use crate::domain::entity::station::Station;
use crate::domain::route_topology::{trim_pattern, RouteStop, RouteTopology};

/// 乗換 1 回あたりの乗換通路の徒歩(秒)。
pub const TRANSFER_WALK_SECONDS: i32 = 3 * 60;
/// 1 経路で乗る列車の最大数(= 乗換 5 回まで)。
pub const MAX_RIDES: usize = 6;
/// 返す経路の最大数。
pub const MAX_JOURNEYS: usize = 6;
/// 代替経路を集めるための探索回数の上限(初回を含む)。
const MAX_SEARCH_RUNS: usize = 8;
/// 最良の経路に対し、代替経路として許容する評価値の倍率(分子/分母)と加算分。
/// これを超える遠回りは提示しない。
const ALTERNATIVE_SLACK_NUMERATOR: i64 = 115;
const ALTERNATIVE_SLACK_DENOMINATOR: i64 = 100;
const ALTERNATIVE_SLACK_SECONDS: i64 = 15 * 60;
/// 経路の順位付けで乗換 1 回ごとに上乗せする重み(秒)。所要時間の見込みには
/// 含めない。乗換アプリと同じく、わずかに速いだけで乗換の多い経路を下げる。
pub const TRANSFER_RANK_SECONDS: i64 = 5 * 60;
/// 代替経路に許す乗換回数の上乗せ。初回探索のパレート解の最多乗換回数を
/// これより上回る代替経路は出さない。
const ALTERNATIVE_EXTRA_TRANSFERS: usize = 1;

/// 列車に乗るときの平均待ち時間(秒)。運転間隔の半分の見込みで、時刻表が
/// 無いので種別(`TrainTypeKind`)から決める。
///
/// - 特急・新幹線 (LimitedExpress): 30 分間隔前後 → 15 分
/// - 急行・新快速級 (Express / HighSpeedRapid): 10 分間隔前後 → 5 分
/// - それ以外 (各停・快速など): 6 分間隔前後 → 3 分
pub fn boarding_wait_seconds(kind: Option<i32>) -> i32 {
    match kind {
        Some(4) => 15 * 60,
        Some(3) | Some(5) => 5 * 60,
        _ => 3 * 60,
    }
}

const UNREACHED: i64 = i64::MAX;

/// 経路の順位付けに使う値。評価値に乗換回数の重みを足す。
fn rank_score(cost: i64, legs: &[LegRef]) -> i64 {
    cost + TRANSFER_RANK_SECONDS * legs.len().saturating_sub(1) as i64
}

/// 1 系統(列車種別の停車パターン)。
#[derive(Clone, Debug)]
struct Pattern {
    line_group_id: u32,
    /// 位置ごとの節点。環状の場合は一周ぶん(閉じた終端を除く)。
    nodes: Vec<u32>,
    station_cds: Vec<i32>,
    line_cds: Vec<i32>,
    /// 乗降できる(= 通過ではない)か。
    stoppable: Vec<bool>,
    /// 展開位置ごとの到着・出発(秒)。線形なら `nodes.len()` 要素、環状なら
    /// 二周ぶん `2 * nodes.len() + 1` 要素で、位置 `q` の駅は `nodes[q % len]`。
    arrival: Vec<i32>,
    departure: Vec<i32>,
    circular: bool,
    /// 乗車時に評価値へ加える平均待ち時間(秒)。
    boarding_wait: i32,
}

impl Pattern {
    fn len(&self) -> usize {
        self.nodes.len()
    }

    /// 走査する展開位置の数。
    fn span(&self) -> usize {
        self.arrival.len()
    }
}

/// 乗車 1 回ぶん。`board` / `alight` はパターンの展開位置。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LegRef {
    pattern: u32,
    board: u32,
    alight: u32,
}

/// 探索結果の乗車区間。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JourneyLeg {
    pub line_group_id: u32,
    /// 乗車駅から降車駅までの駅(通過駅を含む)を進行順に並べたもの。
    /// 駅はこの系統が走る路線の駅 (`station_cd`) で、乗換駅でも系統ごとに異なる。
    pub station_cds: Vec<i32>,
    pub station_group_ids: Vec<u32>,
}

/// 探索結果の経路。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Journey {
    pub legs: Vec<JourneyLeg>,
    /// 推定所要時間(秒)。最初の列車の発車から到着まで。乗換の徒歩と、乗換先の
    /// 待ち時間の見込みを含む(最初の列車の待ち時間は含まない)。
    pub total_seconds: i32,
}

impl Journey {
    pub fn transfer_count(&self) -> usize {
        self.legs.len().saturating_sub(1)
    }
}

/// 探索用の系統網。一度組み立てれば読み取り専用で、リクエスト間で共有できる。
#[derive(Debug, Default)]
pub struct RouteNetwork {
    patterns: Vec<Pattern>,
    node_by_group: HashMap<u32, u32>,
    /// 節点 -> 駅グループ ID。
    node_groups: Vec<u32>,
    /// 節点 -> (パターン, 一周目の位置)。乗降できる位置だけ。
    stop_patterns: Vec<Vec<(u32, u32)>>,
    /// 同じ系統から作った、所要時間を持たない網。到達判定はこちらで行う。
    topology: RouteTopology,
}

impl RouteNetwork {
    /// 系統ごとの駅列から系統網を組み立てる。
    ///
    /// `line_groups` の各要素は 1 系統ぶんの駅を運行順(sst.id 順)に並べたもの。
    /// `sst_id` と `line_group_cd` を持たない駅は無視する。結果の決定性は
    /// `line_groups` の順序に依存するので、呼び出し側は安定した順で渡すこと。
    pub fn build<I>(line_groups: I, params: &EstimationParams) -> Self
    where
        I: IntoIterator<Item = Vec<Station>>,
    {
        let mut network = RouteNetwork::default();
        for stations in line_groups {
            network.add_pattern(stations, params);
        }
        network.topology.finish();
        network
    }

    /// 同じ系統から作った、所要時間を持たない網 (到達判定用)。
    pub fn topology(&self) -> &RouteTopology {
        &self.topology
    }

    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }

    fn node_of(&mut self, station_group_id: u32) -> u32 {
        let next = self.node_by_group.len() as u32;
        let node = *self.node_by_group.entry(station_group_id).or_insert(next);
        if node == next {
            self.node_groups.push(station_group_id);
            self.stop_patterns.push(Vec::new());
        }
        node
    }

    fn add_pattern(&mut self, stations: Vec<Station>, params: &EstimationParams) {
        let mut stations: Vec<Station> = stations
            .into_iter()
            .filter(|s| s.sst_id.is_some() && s.line_group_cd.is_some())
            .collect();
        let Some(line_group_id) = stations.first().and_then(|s| s.line_group_cd) else {
            return;
        };
        let is_stoppable =
            |s: &Station| s.pass != Some(1) && s.stop_condition != crate::model::StopCondition::Not;
        // 系統の整え方は到達判定の網と共有する (同じ系統から同じ網を作るため)
        if !trim_pattern(&mut stations, |s| s.station_cd, is_stoppable) {
            return;
        }
        let stoppable: Vec<bool> = stations.iter().map(is_stoppable).collect();
        self.topology.add_pattern(
            stations
                .iter()
                .map(|s| RouteStop {
                    station_cd: s.station_cd,
                    station_group_id: s.station_g_cd as u32,
                    line_cd: s.line_cd,
                    stoppable: is_stoppable(s),
                })
                .collect(),
        );

        let refs: Vec<&Station> = stations.iter().collect();
        let circular = is_circular_route(&refs);
        let estimated = if circular {
            // 二周ぶん + 始点に戻る 1 駅を並べ、継ぎ目を跨ぐ乗車も同じ配列で引く。
            let mut unrolled: Vec<&Station> = Vec::with_capacity(refs.len() * 2 + 1);
            unrolled.extend(refs.iter().copied());
            unrolled.extend(refs.iter().copied());
            unrolled.push(refs[0]);
            estimate_arrival_minutes_calibrated(&unrolled, &refs, params)
        } else {
            estimate_arrival_minutes_calibrated(&refs, &refs, params)
        };
        let to_seconds = |minutes: f64| -> i32 {
            if minutes.is_finite() {
                (minutes * 60.0).round() as i32
            } else {
                0
            }
        };
        let arrival: Vec<i32> = estimated
            .iter()
            .map(|e| to_seconds(e.cumulative_minutes))
            .collect();
        let departure: Vec<i32> = estimated
            .iter()
            .zip(&arrival)
            .map(|(e, &arr)| to_seconds(e.departure_cumulative_minutes).max(arr))
            .collect();

        let pattern_index = self.patterns.len() as u32;
        let nodes: Vec<u32> = stations
            .iter()
            .map(|s| self.node_of(s.station_g_cd as u32))
            .collect();
        for (pos, &node) in nodes.iter().enumerate() {
            if stoppable[pos] {
                self.stop_patterns[node as usize].push((pattern_index, pos as u32));
            }
        }
        self.patterns.push(Pattern {
            line_group_id: line_group_id as u32,
            nodes,
            station_cds: stations.iter().map(|s| s.station_cd).collect(),
            line_cds: stations.iter().map(|s| s.line_cd).collect(),
            stoppable,
            arrival,
            departure,
            circular,
            boarding_wait: boarding_wait_seconds(stations[0].kind),
        });
    }

    /// `from` から `to`(ともに駅グループ ID)への経路を、良い順に最大
    /// [`MAX_JOURNEYS`] 件返す。
    ///
    /// 順位は「評価値 + 乗換 1 回あたり [`TRANSFER_RANK_SECONDS`]」で決める。
    /// 初回探索のパレート解(乗車回数ごとの最良)は必ず含め、残りの枠を代替経路で
    /// 埋める。代替経路は、見つかった経路の区間を 1 つずつ禁止した再探索で集める
    /// (Yen の k 最短経路の簡略版)。最良の経路に比べて大きく遠回りなものと、
    /// 乗換の多すぎるものは捨てる。
    ///
    /// `via_line_id` を指定すると、目的地にその路線の駅で到着する経路 (最後の
    /// 区間がその路線を走る経路) だけを返す。
    pub fn search(&self, from: u32, to: u32, via_line_id: Option<i32>) -> Vec<Journey> {
        let (Some(&origin), Some(&target)) =
            (self.node_by_group.get(&from), self.node_by_group.get(&to))
        else {
            return Vec::new();
        };
        if origin == target {
            return Vec::new();
        }

        let arrival = Arrival {
            node: target as usize,
            line_cd: via_line_id,
        };
        // パレート解は最適解なので逆戻りの除外をかけない。かけると、逆戻りしか
        // 経路が無い駅が「行ける駅」(reachable_station_cds) なのに 0 件になる
        let pareto = self.raptor(origin, arrival, &HashSet::new(), UNREACHED);
        if pareto.is_empty() {
            return Vec::new();
        }
        let mut seen: HashSet<Vec<(u32, u32, i32, i32)>> = pareto
            .iter()
            .map(|(_, legs)| self.journey_key(legs))
            .collect();
        let best = pareto
            .iter()
            .map(|(cost, legs)| rank_score(*cost, legs))
            .min()
            .unwrap_or(0);
        let score_limit = best * ALTERNATIVE_SLACK_NUMERATOR / ALTERNATIVE_SLACK_DENOMINATOR
            + ALTERNATIVE_SLACK_SECONDS;
        let ride_limit = pareto.iter().map(|(_, legs)| legs.len()).max().unwrap_or(0)
            + ALTERNATIVE_EXTRA_TRANSFERS;
        let acceptable = |cost: i64, legs: &[LegRef]| {
            rank_score(cost, legs) <= score_limit && legs.len() <= ride_limit
        };

        // 再探索の禁止集合を幅優先で広げる。見つかった経路ごとに、区間を
        // 長い順に 1 つずつ禁止した子を積む
        let mut queue: VecDeque<HashSet<(u32, u32)>> = VecDeque::new();
        let push_children = |queue: &mut VecDeque<HashSet<(u32, u32)>>,
                             banned: &HashSet<(u32, u32)>,
                             legs: &[LegRef]| {
            for leg in self.legs_by_ride_time(legs) {
                let mut child = banned.clone();
                self.ban_leg(leg, &mut child);
                queue.push_back(child);
            }
        };
        let root = HashSet::new();
        for (_, legs) in &pareto {
            push_children(&mut queue, &root, legs);
        }

        let mut alternatives: Vec<(i64, Vec<LegRef>)> = Vec::new();
        let mut runs = 1;
        while let Some(banned) = queue.pop_front() {
            if runs >= MAX_SEARCH_RUNS || pareto.len() + alternatives.len() >= MAX_JOURNEYS {
                break;
            }
            runs += 1;
            // 許容幅を超える経路は捨てるので、それを目的地の上限にして枝を刈る
            for (cost, legs) in self.raptor(origin, arrival, &banned, score_limit + 1) {
                if !acceptable(cost, &legs)
                    || self.revisits_station_group(&legs)
                    || !seen.insert(self.journey_key(&legs))
                {
                    continue;
                }
                push_children(&mut queue, &banned, &legs);
                alternatives.push((cost, legs));
            }
        }

        let mut selected = pareto;
        alternatives.sort_by_key(|(cost, legs)| (rank_score(*cost, legs), legs.len()));
        selected.extend(
            alternatives
                .into_iter()
                .take(MAX_JOURNEYS.saturating_sub(selected.len())),
        );
        // 並びを決定的にするため、同じ順位なら乗換の少ない順、次に発見順
        selected.sort_by_key(|(cost, legs)| (rank_score(*cost, legs), legs.len()));
        selected.truncate(MAX_JOURNEYS);

        selected
            .into_iter()
            .map(|(cost, legs)| self.to_journey(cost, &legs))
            .collect()
    }

    /// 頻度ベースの RAPTOR を 1 回走らせ、目的地の評価値が改善したラウンドごとの
    /// 経路(パレート解)を乗車回数の少ない順に返す。
    ///
    /// 評価値は「乗車時間 + 乗車ごとの待ち時間 + 乗換の徒歩」(秒)。ラベルは
    /// 到着時刻ではなくこの評価値で、始点では 0。
    ///
    /// `banned` は「このパターンにこの節点から乗ってはならない」組。
    /// `bound` 以上の評価値でしか目的地に着けない経路は探さない
    /// (上限なしは [`UNREACHED`])。
    fn raptor(
        &self,
        origin: u32,
        arrival: Arrival,
        banned: &HashSet<(u32, u32)>,
        bound: i64,
    ) -> Vec<(i64, Vec<LegRef>)> {
        let target = arrival.node as u32;
        let node_count = self.stop_patterns.len();
        let mut labels: Vec<Vec<i64>> = Vec::with_capacity(MAX_RIDES + 1);
        let mut parents: Vec<Vec<Option<LegRef>>> = Vec::with_capacity(MAX_RIDES + 1);
        let mut best = vec![UNREACHED; node_count];
        let mut initial = vec![UNREACHED; node_count];
        initial[origin as usize] = 0;
        best[origin as usize] = 0;
        best[target as usize] = bound;
        labels.push(initial);
        parents.push(vec![None; node_count]);
        let mut marked: Vec<u32> = vec![origin];
        // パターンごとに、前ラウンドで改善した駅の一周目の位置の最小・最大。
        // ラウンドごとに触れたパターンだけ書き戻すので、確保は探索ごとに 1 回で済む
        let mut ranges: Vec<(u32, u32)> = vec![(u32::MAX, 0); self.patterns.len()];
        let mut touched: Vec<u32> = Vec::new();

        for round in 1..=MAX_RIDES {
            if marked.is_empty() {
                break;
            }
            for &node in &marked {
                for &(pattern, pos) in &self.stop_patterns[node as usize] {
                    let range = &mut ranges[pattern as usize];
                    if range.0 == u32::MAX {
                        touched.push(pattern);
                    }
                    range.0 = range.0.min(pos);
                    range.1 = range.1.max(pos);
                }
            }
            // パターン番号順に走査し、同着時の結果を決定的にする
            touched.sort_unstable();

            let previous = &labels[round - 1];
            let mut current = previous.clone();
            let mut round_parents: Vec<Option<LegRef>> = vec![None; node_count];
            let mut improved = vec![false; node_count];
            let transfer_walk = if round > 1 {
                i64::from(TRANSFER_WALK_SECONDS)
            } else {
                0
            };

            let mut state = RoundState {
                previous,
                current: &mut current,
                best: &mut best,
                parents: &mut round_parents,
                improved: &mut improved,
                arrival,
                transfer_walk,
                banned,
            };
            for &pattern_index in &touched {
                let (lo, hi) =
                    std::mem::replace(&mut ranges[pattern_index as usize], (u32::MAX, 0));
                let pattern = &self.patterns[pattern_index as usize];
                let (len, span) = (pattern.len(), pattern.span());
                // 前向きは最小の改善位置から末尾まで、後ろ向きは最大の改善位置から
                // 先頭まで。環状は二周目の同じ駅から戻り、継ぎ目を跨ぐ乗車も拾う。
                state.scan(pattern_index, pattern, true, lo as usize..span);
                let back_start = if pattern.circular {
                    hi as usize + len
                } else {
                    hi as usize
                };
                state.scan(
                    pattern_index,
                    pattern,
                    false,
                    (0..=back_start.min(span - 1)).rev(),
                );
            }

            touched.clear();
            marked = (0..node_count as u32)
                .filter(|&node| improved[node as usize])
                .collect();
            labels.push(current);
            parents.push(round_parents);
        }

        // 乗車回数を増やして評価値が下がっても、乗換の順位付けの重みを
        // 上回って良くならないならパレート解に入れない (1 分縮めるために
        // 何度も乗り換える経路を出さない)
        let mut results: Vec<(i64, Vec<LegRef>)> = Vec::new();
        let mut best_score = UNREACHED;
        for round in 1..labels.len() {
            let cost = labels[round][target as usize];
            if cost >= labels[round - 1][target as usize] {
                continue;
            }
            let score = cost + TRANSFER_RANK_SECONDS * (round as i64 - 1);
            if score >= best_score {
                continue;
            }
            if let Some(legs) = self.reconstruct(&parents, round, target, origin) {
                best_score = score;
                results.push((cost, legs));
            }
        }
        results
    }

    /// ラウンド `round` の `node` から親をたどって乗車区間を始点側から並べる。
    fn reconstruct(
        &self,
        parents: &[Vec<Option<LegRef>>],
        mut round: usize,
        mut node: u32,
        origin: u32,
    ) -> Option<Vec<LegRef>> {
        let mut legs = Vec::new();
        loop {
            // 前ラウンドから持ち越しただけの到着には親が無いので、設定された
            // ラウンドまで下る。
            while round > 0 && parents[round][node as usize].is_none() {
                round -= 1;
            }
            if round == 0 {
                break;
            }
            let leg = parents[round][node as usize]?;
            let pattern = &self.patterns[leg.pattern as usize];
            node = pattern.nodes[leg.board as usize % pattern.len()];
            legs.push(leg);
            round -= 1;
        }
        if node != origin {
            return None;
        }
        legs.reverse();
        Some(legs)
    }

    /// 区間を乗車時間の長い順に並べる (同じ長さなら先に乗る区間が先)。
    fn legs_by_ride_time<'a>(&self, legs: &'a [LegRef]) -> Vec<&'a LegRef> {
        let mut sorted: Vec<&LegRef> = legs.iter().collect();
        sorted.sort_by_key(|leg| {
            let pattern = &self.patterns[leg.pattern as usize];
            let (a, b) = (leg.board as usize, leg.alight as usize);
            std::cmp::Reverse(pattern.arrival[a.max(b)] - pattern.departure[a.min(b)])
        });
        sorted
    }

    /// 区間について、同じ乗車駅から同じ降車駅へ行ける並行系統 (快速・各停など)
    /// に、その区間のどの駅からも乗ることを禁止する。
    ///
    /// 乗車駅だけを禁止すると「別の列車で 1 駅進んでから同じ新幹線に乗る」
    /// ような、実質同じで乗換だけ増えた経路が代替経路として出てしまう。
    fn ban_leg(&self, main: &LegRef, banned: &mut HashSet<(u32, u32)>) {
        let pattern = &self.patterns[main.pattern as usize];
        let len = pattern.len();
        let board_node = pattern.nodes[main.board as usize % len];
        let alight_node = pattern.nodes[main.alight as usize % len];
        let (lo, hi) = (
            main.board.min(main.alight) as usize,
            main.board.max(main.alight) as usize,
        );
        let span_nodes: Vec<u32> = (lo..=hi)
            .map(|q| pattern.nodes[q % len])
            .filter(|&node| {
                self.stop_patterns[node as usize]
                    .iter()
                    .any(|&(p, _)| p == main.pattern)
            })
            .collect();
        for &(sibling, _) in &self.stop_patterns[board_node as usize] {
            let serves_alight = self.stop_patterns[alight_node as usize]
                .iter()
                .any(|&(p, _)| p == sibling);
            if !serves_alight {
                continue;
            }
            for &node in &span_nodes {
                banned.insert((sibling, node));
            }
        }
    }

    /// 別々の区間で同じ駅グループに停車するか。乗換駅 (前の区間の降車駅 = 次の
    /// 区間の乗車駅) は除く。
    ///
    /// 区間を禁止して再探索すると、「1 駅戻って同じ列車に乗り直す」逆戻りの経路が
    /// 代替経路として出てくるので捨てる。通過した駅へ戻るのは (急行で先の駅まで
    /// 行って戻るなど) 実際にある乗り方なので数えない。1 つの区間の中で同じ駅に
    /// 止まるのは実在する運行 (大江戸線の都庁前など) なので構わない。
    fn revisits_station_group(&self, legs: &[LegRef]) -> bool {
        let mut visited: HashSet<u32> = HashSet::new();
        for leg in legs {
            let pattern = &self.patterns[leg.pattern as usize];
            let len = pattern.len();
            let (board, alight) = (leg.board as usize, leg.alight as usize);
            let board_node = pattern.nodes[board % len];
            let mut leg_nodes: HashSet<u32> = HashSet::new();
            for q in board.min(alight)..=board.max(alight) {
                if pattern.stoppable[q % len] {
                    leg_nodes.insert(pattern.nodes[q % len]);
                }
            }
            if leg_nodes
                .iter()
                .any(|node| *node != board_node && visited.contains(node))
            {
                return true;
            }
            // 乗車駅は前の区間の降車駅なので、既に入っている
            visited.extend(leg_nodes);
        }
        false
    }

    /// 同一視する経路のキー。乗車・降車の駅グループと路線が同じなら、種別違い
    /// でも利用者から見て同じ経路とみなす。
    fn journey_key(&self, legs: &[LegRef]) -> Vec<(u32, u32, i32, i32)> {
        legs.iter()
            .map(|leg| {
                let pattern = &self.patterns[leg.pattern as usize];
                let board = leg.board as usize % pattern.len();
                let alight = leg.alight as usize % pattern.len();
                (
                    pattern.nodes[board],
                    pattern.nodes[alight],
                    pattern.line_cds[board],
                    pattern.line_cds[alight],
                )
            })
            .collect()
    }

    fn to_journey(&self, cost: i64, legs: &[LegRef]) -> Journey {
        // 評価値から、最初の列車を待つぶんだけを除いたものが所要時間
        let first_wait = legs
            .first()
            .map(|leg| i64::from(self.patterns[leg.pattern as usize].boarding_wait))
            .unwrap_or(0);
        let legs = legs
            .iter()
            .map(|leg| {
                let pattern = &self.patterns[leg.pattern as usize];
                let (board, alight) = (leg.board as usize, leg.alight as usize);
                let positions: Vec<usize> = if board <= alight {
                    (board..=alight).collect()
                } else {
                    (alight..=board).rev().collect()
                };
                JourneyLeg {
                    line_group_id: pattern.line_group_id,
                    station_cds: positions
                        .iter()
                        .map(|&q| pattern.station_cds[q % pattern.len()])
                        .collect(),
                    station_group_ids: positions
                        .iter()
                        .map(|&q| self.node_groups[pattern.nodes[q % pattern.len()] as usize])
                        .collect(),
                }
            })
            .collect();
        Journey {
            legs,
            total_seconds: i32::try_from(cost - first_wait).unwrap_or(i32::MAX),
        }
    }
}

/// 目的地の条件。
#[derive(Clone, Copy)]
struct Arrival {
    node: usize,
    /// 指定があれば、目的地にはこの路線の駅で着かなければならない。
    line_cd: Option<i32>,
}

/// 1 ラウンドぶんの作業領域。
struct RoundState<'a> {
    /// 前ラウンドの到着。乗車はこちらからだけ行う(同ラウンドの到着から乗ると
    /// 乗車回数が数えられなくなる)。
    previous: &'a [i64],
    current: &'a mut [i64],
    /// 全ラウンドを通した最良到着。これより遅い到着は記録しない。
    best: &'a mut [i64],
    parents: &'a mut [Option<LegRef>],
    improved: &'a mut [bool],
    arrival: Arrival,
    /// 乗換の徒歩。最初の乗車 (ラウンド 1) では 0。
    transfer_walk: i64,
    banned: &'a HashSet<(u32, u32)>,
}

impl RoundState<'_> {
    /// パターンを一方向に走査する。`positions` はパターンの展開位置を進行順に返す。
    ///
    /// 乗車中は「基準値 + 降車キー」で各駅の到着を求める。前向きなら
    /// 到着 = 前ラウンドの評価値 + 徒歩 + 待ち − 出発(乗車駅) + 到着(降車駅)、
    /// 後ろ向きは所要時間が対称であることを使い 出発と到着の役割を入れ替える。
    fn scan<I: Iterator<Item = usize>>(
        &mut self,
        pattern_index: u32,
        pattern: &Pattern,
        forward: bool,
        positions: I,
    ) {
        let len = pattern.len();
        // 乗車中なら (基準値, 乗車位置)
        let mut boarded: Option<(i64, usize)> = None;
        for q in positions {
            let stop = q % len;
            if pattern.circular {
                if let Some((_, board)) = boarded {
                    // 一周以上は乗らない
                    if q.abs_diff(board) >= len {
                        boarded = None;
                    }
                }
            }
            if !pattern.stoppable[stop] {
                continue;
            }
            let node = pattern.nodes[stop] as usize;
            if let Some((base, board)) = boarded {
                let alight_key = if forward {
                    i64::from(pattern.arrival[q])
                } else {
                    -i64::from(pattern.departure[q])
                };
                let arrival = base + alight_key;
                let accepted = node != self.arrival.node
                    || self
                        .arrival
                        .line_cd
                        .is_none_or(|line_cd| pattern.line_cds[stop] == line_cd);
                if accepted && arrival < self.best[node].min(self.best[self.arrival.node]) {
                    self.current[node] = arrival;
                    self.best[node] = arrival;
                    self.parents[node] = Some(LegRef {
                        pattern: pattern_index,
                        board: board as u32,
                        alight: q as u32,
                    });
                    self.improved[node] = true;
                }
            }
            let reached = self.previous[node];
            if reached == UNREACHED {
                continue;
            }
            let board_key = if forward {
                -i64::from(pattern.departure[q])
            } else {
                i64::from(pattern.arrival[q])
            };
            let candidate =
                reached + self.transfer_walk + i64::from(pattern.boarding_wait) + board_key;
            // 禁止の照合はハッシュを引くので、乗り直しが得になるときだけ行う
            if boarded.is_none_or(|(base, _)| candidate < base)
                && !self.banned.contains(&(pattern_index, node as u32))
            {
                boarded = Some((candidate, q));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entity::gtfs::TransportType;
    use crate::model::StopCondition;

    /// 1 分おきに東へ並ぶ駅。`group` が同じ駅は乗換できる。
    fn stop(line_group: i32, sst_id: i32, group: i32, lon_step: f64, lat_step: f64) -> Station {
        Station {
            station_cd: sst_id,
            station_g_cd: group,
            station_name: String::new(),
            station_name_k: String::new(),
            station_name_r: None,
            station_name_zh: None,
            station_name_ko: None,
            station_numbers: vec![],
            station_number1: None,
            station_number2: None,
            station_number3: None,
            station_number4: None,
            three_letter_code: None,
            line_cd: line_group,
            line: None,
            lines: vec![],
            pref_cd: 13,
            post: String::new(),
            address: String::new(),
            lon: 139.0 + lon_step * 0.01,
            lat: 35.0 + lat_step * 0.01,
            open_ymd: String::new(),
            close_ymd: String::new(),
            e_status: 0,
            e_sort: sst_id,
            stop_condition: StopCondition::All,
            distance: None,
            has_train_types: true,
            train_type: None,
            company_cd: Some(1),
            line_name: None,
            line_name_k: None,
            line_name_h: None,
            line_name_r: None,
            line_name_zh: None,
            line_name_ko: None,
            line_color_c: None,
            line_type: Some(2),
            line_symbol1: None,
            line_symbol2: None,
            line_symbol3: None,
            line_symbol4: None,
            line_symbol1_color: None,
            line_symbol2_color: None,
            line_symbol3_color: None,
            line_symbol4_color: None,
            line_symbol1_shape: None,
            line_symbol2_shape: None,
            line_symbol3_shape: None,
            line_symbol4_shape: None,
            average_distance: None,
            type_id: Some(1),
            sst_id: Some(sst_id),
            type_cd: Some(1),
            line_group_cd: Some(line_group),
            pass: None,
            type_name: None,
            type_name_k: None,
            type_name_r: None,
            type_name_zh: None,
            type_name_ko: None,
            color: None,
            direction: None,
            kind: None,
            transport_type: TransportType::Rail,
        }
    }

    /// 東西に一直線の系統。`groups` の駅を 1km 間隔で並べる。
    fn straight(line_group: i32, groups: &[(i32, f64, f64)]) -> Vec<Station> {
        groups
            .iter()
            .enumerate()
            .map(|(i, &(group, x, y))| stop(line_group, line_group * 100 + i as i32, group, x, y))
            .collect()
    }

    fn line_groups(journey: &Journey) -> Vec<u32> {
        journey.legs.iter().map(|leg| leg.line_group_id).collect()
    }

    /// 1 -2 -3 -4 の東西線 (100) と、3 から北へ 5 -6 の南北線 (200)。
    fn t_shaped() -> RouteNetwork {
        RouteNetwork::build(
            [
                straight(
                    100,
                    &[(1, 0.0, 0.0), (2, 1.0, 0.0), (3, 2.0, 0.0), (4, 3.0, 0.0)],
                ),
                straight(200, &[(3, 2.0, 0.0), (5, 2.0, 1.0), (6, 2.0, 2.0)]),
            ],
            &EstimationParams::default(),
        )
    }

    #[test]
    fn finds_direct_route_in_both_directions() {
        let network = t_shaped();
        let forward = network.search(1, 4, None);
        assert_eq!(forward.len(), 1);
        assert_eq!(line_groups(&forward[0]), vec![100]);
        assert_eq!(
            forward[0].legs[0].station_cds,
            vec![10000, 10001, 10002, 10003]
        );
        assert_eq!(
            forward[0].legs[0].station_group_ids,
            vec![1, 2, 3, 4],
            "the leg lists every station from boarding to alighting"
        );

        let backward = network.search(4, 1, None);
        assert_eq!(backward[0].legs[0].station_group_ids, vec![4, 3, 2, 1]);
        assert_eq!(forward[0].total_seconds, backward[0].total_seconds);
    }

    #[test]
    fn transfers_at_shared_station_group() {
        let network = t_shaped();
        let journeys = network.search(1, 6, None);
        assert_eq!(journeys.len(), 1);
        let journey = &journeys[0];
        assert_eq!(line_groups(journey), vec![100, 200]);
        assert_eq!(journey.transfer_count(), 1);
        assert_eq!(journey.legs[0].station_group_ids, vec![1, 2, 3]);
        assert_eq!(journey.legs[1].station_group_ids, vec![3, 5, 6]);

        let direct = network.search(1, 3, None)[0].total_seconds;
        assert!(
            journey.total_seconds >= direct + TRANSFER_WALK_SECONDS + boarding_wait_seconds(None),
            "a transfer adds the walk and the wait for the next train"
        );
    }

    #[test]
    fn unknown_or_identical_endpoints_return_nothing() {
        let network = t_shaped();
        assert!(network.search(1, 1, None).is_empty());
        assert!(network.search(1, 99, None).is_empty());
        assert!(network.search(99, 1, None).is_empty());
    }

    #[test]
    fn cannot_board_or_alight_at_passed_station() {
        let mut express = straight(
            100,
            &[(1, 0.0, 0.0), (2, 1.0, 0.0), (3, 2.0, 0.0), (4, 3.0, 0.0)],
        );
        express[1].pass = Some(1);
        let network = RouteNetwork::build([express], &EstimationParams::default());
        assert!(network.search(1, 2, None).is_empty());
        let through = network.search(1, 4, None);
        assert_eq!(
            through[0].legs[0].station_group_ids,
            vec![1, 2, 3, 4],
            "passed stations stay in the leg so clients can draw them"
        );
    }

    #[test]
    fn prefers_faster_route_and_keeps_fewest_transfers() {
        // 各停 (100) は 1 -> 9 を 8 駅かけて進む。快速 (200) は 1 と 9 にだけ
        // 止まるが、別系統 (300) で 9 から 10 へ乗り継ぐ必要がある。
        // 各停は 10 まで直通する。
        let mut local: Vec<(i32, f64, f64)> = (1..=10).map(|g| (g, g as f64 * 3.0, 0.0)).collect();
        local[9] = (10, 30.0, 0.0);
        let local = straight(100, &local);
        let rapid = straight(200, &[(1, 3.0, 0.0), (9, 27.0, 0.0)]);
        let feeder = straight(300, &[(9, 27.0, 0.0), (10, 30.0, 0.0)]);
        let network = RouteNetwork::build([local, rapid, feeder], &EstimationParams::default());

        let journeys = network.search(1, 10, None);
        let shapes: Vec<Vec<u32>> = journeys.iter().map(line_groups).collect();
        assert!(shapes.contains(&vec![100]), "the direct local must be kept");
        let score = |journey: &Journey| {
            i64::from(journey.total_seconds)
                + TRANSFER_RANK_SECONDS * journey.transfer_count() as i64
        };
        assert!(
            journeys.windows(2).all(|w| score(&w[0]) <= score(&w[1])),
            "journeys are ordered by estimated time weighted by transfers"
        );
    }

    #[test]
    fn collects_alternative_routes_through_other_stations() {
        // 1 から 4 へ、2 経由と 3 経由の二通り。
        let network = RouteNetwork::build(
            [
                straight(100, &[(1, 0.0, 0.0), (2, 1.0, 1.0)]),
                straight(200, &[(2, 1.0, 1.0), (4, 2.0, 0.0)]),
                straight(300, &[(1, 0.0, 0.0), (3, 1.0, -1.2)]),
                straight(400, &[(3, 1.0, -1.2), (4, 2.0, 0.0)]),
            ],
            &EstimationParams::default(),
        );
        let journeys = network.search(1, 4, None);
        let shapes: Vec<Vec<u32>> = journeys.iter().map(line_groups).collect();
        assert_eq!(shapes, vec![vec![100, 200], vec![300, 400]]);
    }

    #[test]
    fn via_line_restricts_the_line_arriving_at_the_destination() {
        // collects_alternative_routes_through_other_stations と同じ網。
        // テストの駅は line_cd = line_group_cd。
        let network = RouteNetwork::build(
            [
                straight(100, &[(1, 0.0, 0.0), (2, 1.0, 1.0)]),
                straight(200, &[(2, 1.0, 1.0), (4, 2.0, 0.0)]),
                straight(300, &[(1, 0.0, 0.0), (3, 1.0, -1.2)]),
                straight(400, &[(3, 1.0, -1.2), (4, 2.0, 0.0)]),
            ],
            &EstimationParams::default(),
        );
        let shapes =
            |via| -> Vec<Vec<u32>> { network.search(1, 4, via).iter().map(line_groups).collect() };
        assert_eq!(shapes(Some(200)), vec![vec![100, 200]]);
        assert_eq!(shapes(Some(400)), vec![vec![300, 400]]);
        assert!(shapes(Some(100)).is_empty(), "line 100 never reaches 4");
    }

    #[test]
    fn rejects_routes_that_backtrack_through_a_visited_station() {
        // 1 -> 2 -> 3 の各停 (100) と、2 から 4 へ行く支線 (200)。支線は 1 に
        // 止まらないので、1 から 4 へは 2 で乗り換える。代替経路のために 100 の
        // 1 -> 2 を禁止しても、「別の系統 (300) で 0 へ戻ってから 100 に乗り直す」
        // ような 1 を再訪する経路は出さない。
        let network = RouteNetwork::build(
            [
                straight(
                    100,
                    &[(0, -1.0, 0.0), (1, 0.0, 0.0), (2, 1.0, 0.0), (3, 2.0, 0.0)],
                ),
                straight(200, &[(2, 1.0, 0.0), (4, 1.0, 1.0)]),
                straight(300, &[(1, 0.0, 0.0), (0, -1.0, 0.0)]),
            ],
            &EstimationParams::default(),
        );
        let journeys = network.search(1, 4, None);
        assert_eq!(journeys.len(), 1);
        assert_eq!(line_groups(&journeys[0]), vec![100, 200]);
    }

    #[test]
    fn reachable_stations_follow_transfers_and_skip_passed_stations() {
        let mut express = straight(300, &[(3, 2.0, 0.0), (7, 3.0, 0.0), (8, 4.0, 0.0)]);
        express[1].pass = Some(1);
        let mut line_groups = vec![
            straight(100, &[(1, 0.0, 0.0), (2, 1.0, 0.0), (3, 2.0, 0.0)]),
            straight(200, &[(3, 2.0, 0.0), (5, 2.0, 1.0)]),
            express,
            // 1 からつながっていない系統
            straight(400, &[(9, 9.0, 9.0), (10, 9.5, 9.0)]),
        ];
        // 7 は急行 (300) が通過するが、別の路線 (500) の駅としてなら止まる
        line_groups.push(straight(500, &[(8, 4.0, 0.0), (7, 3.0, 0.0)]));
        let network = RouteNetwork::build(line_groups, &EstimationParams::default());

        let reachable = network.topology().reachable_station_cds(1);
        // straight() の station_cd は line_group * 100 + 位置
        for station_cd in [
            10000, 10001, 10002, 20000, 20001, 30000, 30002, 50000, 50001,
        ] {
            assert!(reachable.contains(&station_cd), "{station_cd} is reachable");
        }
        assert!(
            !reachable.contains(&30001),
            "the express passes 7, so 7 on line 300 is not a place to get off"
        );
        assert!(!reachable.contains(&40000) && !reachable.contains(&40001));
        assert!(network.topology().reachable_station_cds(99).is_empty());

        // 到達できると判定した駅は、その路線を via にした探索で経路が出る
        assert!(!network.search(1, 7, Some(500)).is_empty());
        assert!(network.search(1, 7, Some(300)).is_empty());
    }

    #[test]
    fn cannot_arrive_at_a_junction_on_the_branch_that_starts_there() {
        // 本線 (100) 1 - 2 - 3 と、3 から出る支線 (200) 3 - 4 (石橋阪大前と箕面線)。
        // 支線の 3 に支線の列車で着くには、3 を通って 4 へ出てから戻るしかない
        let network = RouteNetwork::build(
            [
                straight(100, &[(1, 0.0, 0.0), (2, 1.0, 0.0), (3, 2.0, 0.0)]),
                straight(200, &[(3, 2.0, 0.0), (4, 2.0, 1.0)]),
            ],
            &EstimationParams::default(),
        );
        let reachability = network.topology().reachability(1);
        // straight() の station_cd は line_group * 100 + 位置、line_cd は line_group
        assert!(reachability.can_arrive(10002, 3, 100));
        assert!(reachability.can_arrive(20001, 4, 200));
        assert!(
            !reachability.can_arrive(20000, 3, 200),
            "the branch's junction station is not reached by the branch itself"
        );
        assert!(network.search(1, 3, Some(200)).is_empty());
        assert!(
            !reachability.can_arrive(99999, 3, 100),
            "unknown stations are unreachable"
        );
    }

    #[test]
    fn reachability_matches_search_on_a_ring_with_branches() {
        // 環状 (100) に支線 (200, 300) が付き、支線同士は 5 で接する。どの駅も
        // 「着けると判定した駅には経路がある」ことを総当たりで確かめる
        let ring: Vec<(i32, f64, f64)> = (0..12)
            .map(|i| {
                let angle = i as f64 / 12.0 * std::f64::consts::TAU;
                (i + 1, angle.cos() * 3.0, angle.sin() * 3.0)
            })
            .collect();
        let line_groups = vec![
            straight(100, &ring),
            straight(200, &[(3, 1.5, 2.6), (20, 3.0, 5.0), (5, -1.5, 2.6)]),
            straight(300, &[(20, 3.0, 5.0), (21, 4.0, 6.0)]),
            straight(400, &[(9, 0.0, -3.0), (30, 0.0, -5.0)]),
        ];
        let stations: Vec<(i32, u32, i32)> = line_groups
            .iter()
            .flatten()
            .map(|s| (s.station_cd, s.station_g_cd as u32, s.line_cd))
            .collect();
        let network = RouteNetwork::build(line_groups, &EstimationParams::default());
        for origin in [1, 20, 21, 30] {
            let reachability = network.topology().reachability(origin);
            for &(station_cd, group, line_cd) in &stations {
                if group == origin {
                    continue;
                }
                assert_eq!(
                    reachability.can_arrive(station_cd, group, line_cd),
                    !network.search(origin, group, Some(line_cd)).is_empty(),
                    "origin {origin}, station {station_cd}"
                );
            }
        }
    }

    #[test]
    fn topology_built_from_route_stops_matches_the_one_inside_the_network() {
        // 閉じた環状 (先頭駅を末尾にも持つ)、通過駅、停車駅が 1 つしかない系統を含む
        let mut closed_ring = straight(
            100,
            &[(1, 0.0, 0.0), (2, 1.0, 0.0), (3, 1.0, 1.0), (4, 0.0, 1.0)],
        );
        let mut closing = closed_ring[0].clone();
        closing.sst_id = Some(10099);
        closed_ring.push(closing);
        let mut express = straight(200, &[(2, 1.0, 0.0), (5, 2.0, 0.0), (6, 3.0, 0.0)]);
        express[1].pass = Some(1);
        let mut single_stop = straight(300, &[(6, 3.0, 0.0), (7, 4.0, 0.0)]);
        single_stop[1].pass = Some(1);
        let line_groups = vec![closed_ring, express, single_stop];

        let stops: Vec<Vec<RouteStop>> = line_groups
            .iter()
            .map(|stations| {
                stations
                    .iter()
                    .map(|s| RouteStop {
                        station_cd: s.station_cd,
                        station_group_id: s.station_g_cd as u32,
                        line_cd: s.line_cd,
                        stoppable: s.pass != Some(1),
                    })
                    .collect()
            })
            .collect();
        let network = RouteNetwork::build(line_groups, &EstimationParams::default());
        let topology = RouteTopology::build(stops);
        assert_eq!(&topology, network.topology());
        assert_eq!(
            topology.pattern_count(),
            2,
            "the single-stop line group is dropped"
        );
    }

    #[test]
    fn reachable_stations_stop_after_the_ride_limit() {
        // 0 -> 1 -> ... -> 7 を 1 駅ずつ別の系統でつなぐ。乗車 6 本で 6 まで
        let line_groups: Vec<Vec<Station>> = (0..7)
            .map(|i| {
                straight(
                    100 + i,
                    &[(i, f64::from(i), 0.0), (i + 1, f64::from(i + 1), 0.0)],
                )
            })
            .collect();
        let network = RouteNetwork::build(line_groups, &EstimationParams::default());
        let reachable = network.topology().reachable_station_cds(0);
        assert!(
            reachable.contains(&(105 * 100 + 1)),
            "group 6 by the sixth ride"
        );
        assert!(
            !reachable.contains(&(106 * 100 + 1)),
            "group 7 needs a seventh ride"
        );
        assert!(network.search(0, 6, None).len() == 1);
        assert!(network.search(0, 7, None).is_empty());
    }

    #[test]
    fn collapses_parallel_train_types_into_one_route() {
        // 同じ停車駅の系統が 2 つ (種別違い) あっても同じ経路は 1 件にまとめる。
        let groups = [(1, 0.0, 0.0), (2, 1.0, 0.0), (3, 2.0, 0.0)];
        let network = RouteNetwork::build(
            [straight(100, &groups), straight(200, &groups)],
            &EstimationParams::default(),
        );
        assert_eq!(network.search(1, 3, None).len(), 1);
    }

    #[test]
    fn rides_across_the_seam_of_a_circular_line() {
        // 12 駅の環状線。格納順の末尾 (12) から先頭 (1) を跨いで 2 へ行く。
        let ring: Vec<(i32, f64, f64)> = (0..12)
            .map(|i| {
                let angle = i as f64 / 12.0 * std::f64::consts::TAU;
                (i + 1, angle.cos() * 3.0, angle.sin() * 3.0)
            })
            .collect();
        let network = RouteNetwork::build([straight(100, &ring)], &EstimationParams::default());
        let journeys = network.search(11, 2, None);
        assert_eq!(journeys.len(), 1);
        assert_eq!(
            journeys[0].legs[0].station_group_ids,
            vec![11, 12, 1, 2],
            "the short arc wraps across the stored seam"
        );
        let backward = network.search(2, 11, None);
        assert_eq!(backward[0].legs[0].station_group_ids, vec![2, 1, 12, 11]);
    }

    #[test]
    fn search_is_deterministic() {
        let network = RouteNetwork::build(
            [
                straight(100, &[(1, 0.0, 0.0), (2, 1.0, 1.0)]),
                straight(200, &[(2, 1.0, 1.0), (4, 2.0, 0.0)]),
                straight(300, &[(1, 0.0, 0.0), (3, 1.0, -1.0)]),
                straight(400, &[(3, 1.0, -1.0), (4, 2.0, 0.0)]),
            ],
            &EstimationParams::default(),
        );
        assert_eq!(network.search(1, 4, None), network.search(1, 4, None));
    }
}
