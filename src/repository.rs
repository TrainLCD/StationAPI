//! `stationapi::domain::repository` の各トレイトをインメモリ索引で実装する。
//!
//! これにより UseCase 層 (`QueryInteractor`) を一切変更せずに Workers 上で動かせる。
//! PoC では座標検索・名前検索の経路で呼ばれるメソッドだけを実装し、
//! 残りは明示的にエラーを返す (黙って空を返すと未実装が正常応答に見えるため)。

use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};

use stationapi::domain::arrival_estimation::EstimationParams;
use stationapi::domain::entity::company::Company;
use stationapi::domain::entity::gtfs::TransportType;
use stationapi::domain::entity::line::Line;
use stationapi::domain::entity::station::Station;
use stationapi::domain::entity::train_type::TrainType;
use stationapi::domain::error::DomainError;
use stationapi::domain::repository::company_repository::CompanyRepository;
use stationapi::domain::repository::line_repository::LineRepository;
use stationapi::domain::repository::station_repository::StationRepository;
use stationapi::domain::repository::train_type_repository::TrainTypeRepository;
use stationapi::domain::route_search::RouteNetwork;
use stationapi::domain::route_topology::{RouteStop, RouteTopology};
use stationapi::model::StopCondition;

use crate::index;

/// 有効な (e_status = 0) 路線だけを返す。
/// 大半の問い合わせは有効な路線しか対象にしないため、無効な路線を混ぜない。
fn active_line(line_cd: i32) -> Option<&'static Line> {
    index::line_by_cd(line_cd).filter(|l| l.e_status == 0)
}

/// 駅グループ ID 群に属する有効な駅を、路線の属性を埋めた Station として返す。
/// 無効な路線の駅は除く。
fn stations_of_groups(group_ids: &[u32]) -> Vec<Station> {
    let mut out = Vec::new();
    for &gid in group_ids {
        for record in index::stations_by_group(gid as i32) {
            if record.e_status != 0 {
                continue;
            }
            // 路線が引けない駅は返さない
            let Some(line) = index::line_by_cd(record.line_cd) else {
                continue;
            };
            if line.e_status != 0 {
                continue;
            }
            out.push(record.to_entity(Some(line)));
        }
    }
    out
}

/// 系統 (station_station_types) と列車種別 (types) の内容を Station に反映する。
fn apply_train_type(station: &mut Station, sst: &index::SstRecord, ty: &index::TypeRecord) {
    station.sst_id = Some(sst.id);
    station.type_cd = Some(sst.type_cd);
    station.line_group_cd = sst.line_group_cd;
    station.pass = sst.pass;
    station.type_id = Some(ty.id);
    station.type_name = Some(ty.type_name.clone());
    station.type_name_k = Some(ty.type_name_k.clone());
    station.type_name_r = ty.type_name_r.clone();
    station.type_name_zh = ty.type_name_zh.clone();
    station.type_name_ko = ty.type_name_ko.clone();
    station.color = Some(ty.color.clone());
    station.direction = ty.direction;
    station.kind = ty.kind;
    station.has_train_types = sst.line_group_cd.is_some();
    station.stop_condition = match sst.pass.unwrap_or(0) {
        1 => StopCondition::Not,
        2 => StopCondition::Partial,
        3 => StopCondition::Weekday,
        4 => StopCondition::Holiday,
        5 => StopCondition::PartialStop,
        _ => StopCondition::All,
    };
}

/// その駅が属する系統のうち先頭 (= 最小の sst.id) を 1 つだけ反映する。
fn apply_first_train_type(station: &mut Station) {
    let Some(sst) = index::sst_by_station(station.station_cd).next() else {
        return;
    };
    match index::type_by_cd(sst.type_cd) {
        Some(ty) => apply_train_type(station, sst, ty),
        // 種別が引けなくても系統の情報は入れる
        None => {
            station.sst_id = Some(sst.id);
            station.type_cd = Some(sst.type_cd);
            station.line_group_cd = sst.line_group_cd;
            station.pass = sst.pass;
            station.has_train_types = sst.line_group_cd.is_some();
        }
    }
}

/// 系統の停車駅の行を sst.id 昇順で返す。駅・路線・種別のいずれかが引けない行と、
/// 無効な駅・路線の行は落とす。
///
/// 経路探索の網 (`RouteNetwork`) と到達判定の網 (`RouteTopology`) はどちらも
/// この行から作るので、同じ系統からは同じ網になる。
fn line_group_rows(
    group_id: i32,
) -> impl Iterator<
    Item = (
        &'static index::SstRecord,
        &'static index::StationRecord,
        &'static Line,
        &'static index::TypeRecord,
    ),
> {
    index::sst_by_group(group_id).filter_map(|sst| {
        let record = index::station_by_cd(sst.station_cd).filter(|r| r.e_status == 0)?;
        let line = index::line_by_cd(record.line_cd).filter(|l| l.e_status == 0)?;
        // 種別が引けない系統は落とす
        let ty = index::type_by_cd(sst.type_cd)?;
        Some((sst, record, line, ty))
    })
}

/// 指定した系統の停車駅を返す。並びは指定された系統の順、各系統内は sst.id 昇順。
/// 駅・路線・種別のいずれかが引けない行は落とす。
fn stations_of_line_groups(group_ids: &[u32]) -> Vec<Station> {
    let mut out = Vec::new();
    for &group_id in group_ids {
        for (sst, record, line, ty) in line_group_rows(group_id as i32) {
            let mut station = record.to_entity(Some(line));
            apply_train_type(&mut station, sst, ty);
            out.push(station);
        }
    }
    out
}

/// 鉄道の系統 (line_group_cd の昇順)。バスは探索の対象にしない。
///
/// 系統の種別判定は先頭の駅で行う (GTFS 由来のバス系統は数が多いので、行を
/// 作ってから捨てると組み立てが遅くなる)。
fn rail_line_group_cds() -> impl Iterator<Item = i32> {
    index::line_group_cds().into_iter().filter(|&group| {
        index::sst_by_group(group)
            .find_map(|sst| index::station_by_cd(sst.station_cd))
            .and_then(|record| index::line_by_cd(record.line_cd))
            .is_some_and(|line| line.transport_type == TransportType::Rail)
    })
}

/// 乗換経路探索 (`connectedRoutes`) 用の系統網。全系統の駅と所要時間の推定から
/// 組み立てるので、最初に `connectedRoutes` が呼ばれたときに一度だけ作り、
/// isolate の寿命の間使い回す。
static ROUTE_NETWORK: OnceLock<Arc<RouteNetwork>> = OnceLock::new();

fn route_network() -> &'static Arc<RouteNetwork> {
    ROUTE_NETWORK.get_or_init(|| Arc::new(build_route_network()))
}

fn build_route_network() -> RouteNetwork {
    RouteNetwork::build(
        rail_line_group_cds().map(|group| stations_of_line_groups(&[group as u32])),
        &EstimationParams::default(),
    )
}

/// 行き先の検索 (`stationsByName`) で乗換の到達判定に使う、所要時間を持たない
/// 系統網。`Station` も所要時間の推定も要らないので、`ROUTE_NETWORK` よりずっと
/// 速く組み立てられる。`ROUTE_NETWORK` の中の網と同じものになる (テストで確認)。
static ROUTE_TOPOLOGY: OnceLock<RouteTopology> = OnceLock::new();

fn route_topology() -> &'static RouteTopology {
    ROUTE_TOPOLOGY.get_or_init(build_route_topology)
}

fn build_route_topology() -> RouteTopology {
    RouteTopology::build(rail_line_group_cds().map(|group| {
        line_group_rows(group)
            .map(|(sst, record, _, _)| RouteStop {
                station_cd: record.station_cd,
                station_group_id: record.station_g_cd as u32,
                line_cd: record.line_cd,
                // RouteNetwork は pass と stop_condition で判定する。apply_train_type は
                // pass == 1 のときだけ stop_condition を Not にするので同じ結果になる
                stoppable: sst.pass != Some(1),
            })
            .collect()
    }))
}

// ---------------------------------------------------------------- 駅

#[derive(Clone, Default)]
pub struct MemStationRepository;

#[async_trait]
impl StationRepository for MemStationRepository {
    async fn get_route_network(&self) -> Result<Arc<RouteNetwork>, DomainError> {
        Ok(Arc::clone(route_network()))
    }

    async fn get_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        limit: Option<u32>,
        transport_type: Option<TransportType>,
    ) -> Result<Vec<Station>, DomainError> {
        // 未指定なら 1 件
        let limit = limit.unwrap_or(1).min(1_000) as usize;
        let want = transport_type.map(|t| t as i32);
        Ok(index::nearest(latitude, longitude, limit, want)
            .into_iter()
            .map(|(record, distance_km)| {
                // NOTE: 座標検索は路線の有効・無効を見ない
                let mut station = record.to_entity(index::line_by_cd(record.line_cd));
                station.distance = Some(distance_km * 1000.0);
                // has_train_types 用に系統を 1 件だけ引く
                station.line_group_cd = index::first_line_group_cd(record.station_cd);
                station.has_train_types = station.line_group_cd.is_some();
                station
            })
            .collect())
    }

    /// 名前の部分一致に加えて、`from_station_group_id` が指定された場合は
    /// 「その駅から行けるか」で絞り込む。条件は次のいずれか。
    ///
    /// - 出発駅と同じ系統に、通過ではない停車として含まれる (分岐 A)
    /// - 出発駅か目的駅のどちらかが系統を持たず、かつ同じ路線にある (分岐 B)
    /// - 鉄道で、乗り換えればその駅の路線の列車で着ける (分岐 C。
    ///   `connectedRoutes(viaLineId = その駅の路線)` で経路が出る駅)
    ///
    /// `from_station_group_id` が無ければ絞り込みは掛からない。
    /// 件数の上限は絞り込みの後に効くため、切るのは最後。
    async fn get_by_name(
        &self,
        station_name: String,
        limit: Option<u32>,
        from_station_group_id: Option<u32>,
        transport_type: Option<TransportType>,
    ) -> Result<Vec<Station>, DomainError> {
        // 未指定なら実質全件
        let limit = limit.unwrap_or(u32::MAX).min(10_000) as usize;
        let want = transport_type.map(|t| t as i32);
        let hits = index::search_by_name(&station_name, want);

        let Some(group_id) = from_station_group_id else {
            return Ok(hits
                .into_iter()
                .take(limit)
                .map(|record| record.to_entity(index::line_by_cd(record.line_cd)))
                .collect());
        };

        // 出発駅グループ側をまとめる。
        // - from_groups: 出発駅が属する系統 (分岐 A 用)
        // - from_line_cds: 出発駅の路線 (分岐 B 用)
        // - lines_without_types: 系統を持たない出発駅の路線 (分岐 B 用)
        let mut from_groups: HashSet<i32> = HashSet::new();
        let mut from_line_cds: HashSet<i32> = HashSet::new();
        let mut lines_without_types: HashSet<i32> = HashSet::new();
        for from in index::stations_by_group(group_id as i32).filter(|s| s.e_status == 0) {
            from_line_cds.insert(from.line_cd);
            let mut has_sst = false;
            for sst in index::sst_by_station(from.station_cd) {
                has_sst = true;
                if let Some(group) = sst.line_group_cd {
                    from_groups.insert(group);
                }
            }
            if !has_sst {
                lines_without_types.insert(from.line_cd);
            }
        }

        // 乗換で行ける駅の判定。connectedRoutes と同じ系統から作った、所要時間を
        // 持たない網を使う。乗換が要る駅が出たときに一度だけ作る。網は鉄道だけ
        let rail_wanted = want.is_none_or(|t| t == TransportType::Rail as i32);
        let mut reachability = None;

        let mut out = Vec::new();
        for record in hits {
            let mut dst_has_sst = false;
            // 分岐 A: 出発駅と同じ系統に、通過ではない停車として含まれる
            let mut shared_group = None;
            for sst in index::sst_by_station(record.station_cd) {
                dst_has_sst = true;
                if shared_group.is_none() && sst.pass != Some(1) {
                    shared_group = sst.line_group_cd.filter(|g| from_groups.contains(g));
                }
            }
            // 分岐 B: 出発駅か目的駅のどちらかが系統を持たず、かつ同じ路線
            let same_line = if dst_has_sst {
                lines_without_types.contains(&record.line_cd)
            } else {
                from_line_cds.contains(&record.line_cd)
            };
            // 分岐 C: 乗り換えれば、この駅にこの路線の列車で着ける
            // (connectedRoutes(viaLineId = この駅の路線) で経路が出る)。
            // 共有する系統は無いので line_group_cd は空、has_train_types は false
            if shared_group.is_none() && !same_line {
                if !rail_wanted {
                    continue;
                }
                let reachability =
                    reachability.get_or_insert_with(|| route_topology().reachability(group_id));
                if !reachability.can_arrive(
                    record.station_cd,
                    record.station_g_cd as u32,
                    record.line_cd,
                ) {
                    continue;
                }
            }

            let mut station = record.to_entity(index::line_by_cd(record.line_cd));
            // has_train_types には出発駅と共有している系統を使う
            station.line_group_cd = shared_group;
            station.has_train_types = shared_group.is_some();
            out.push(station);
            if out.len() >= limit {
                break;
            }
        }
        Ok(out)
    }

    /// 駅に紐づく種別ごとに 1 件返す。種別を持たない駅は種別なしで 1 件返す。
    async fn get_by_station_group_id_vec(
        &self,
        station_group_id_vec: &[u32],
    ) -> Result<Vec<Station>, DomainError> {
        let mut out = Vec::new();
        for &group_id in station_group_id_vec {
            for record in index::stations_by_group(group_id as i32) {
                if record.e_status != 0 {
                    continue;
                }
                let Some(line) = index::line_by_cd(record.line_cd) else {
                    continue;
                };
                if line.e_status != 0 {
                    continue;
                }

                // 種別を持つ駅は系統の数だけ行が出る
                let mut matched = false;
                for sst in index::sst_by_station(record.station_cd) {
                    let Some(ty) = index::type_by_cd(sst.type_cd) else {
                        continue;
                    };
                    let mut station = record.to_entity(Some(line));
                    apply_train_type(&mut station, sst, ty);
                    out.push(station);
                    matched = true;
                }
                if !matched {
                    out.push(record.to_entity(Some(line)));
                }
            }
        }
        Ok(out)
    }

    /// 種別は付けず、has_train_types 用に系統を 1 件だけ引く。
    /// ここを埋めないと lines[].station.hasTrainTypes が常に false になる。
    async fn get_by_station_group_id_vec_no_types(
        &self,
        station_group_id_vec: &[u32],
    ) -> Result<Vec<Station>, DomainError> {
        let mut out = stations_of_groups(station_group_id_vec);
        for station in out.iter_mut() {
            station.line_group_cd = index::first_line_group_cd(station.station_cd);
            station.has_train_types = station.line_group_cd.is_some();
        }
        Ok(out)
    }

    async fn get_by_station_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Station>, DomainError> {
        self.get_by_station_group_id_vec(&[station_group_id]).await
    }

    /// 系統と種別を 1 件だけ反映する。埋めないと hasTrainTypes が常に false になる。
    async fn find_by_id(&self, id: u32) -> Result<Option<Station>, DomainError> {
        Ok(index::station_by_cd(id as i32)
            .filter(|r| r.e_status == 0)
            .filter(|r| active_line(r.line_cd).is_some())
            .map(|r| {
                let mut station = r.to_entity(active_line(r.line_cd));
                apply_first_train_type(&mut station);
                station
            }))
    }

    /// 種別は付けず、has_train_types 用に系統を 1 件だけ引く。並びは指定された ID の順。
    async fn get_by_id_vec(&self, ids: &[u32]) -> Result<Vec<Station>, DomainError> {
        Ok(ids
            .iter()
            .filter_map(|&id| index::station_by_cd(id as i32))
            .filter(|r| r.e_status == 0)
            .filter(|r| active_line(r.line_cd).is_some())
            .map(|r| {
                let mut station = r.to_entity(active_line(r.line_cd));
                station.line_group_cd = index::first_line_group_cd(r.station_cd);
                station.has_train_types = station.line_group_cd.is_some();
                station
            })
            .collect())
    }

    /// 各座標につき半径以内のバス停を近い順に見て、有効な路線を持つものだけを
    /// N 件まで採る。上限を先に掛けると、路線を引けないバス停や廃止路線の
    /// バス停が枠を埋めたぶんだけ件数が減るため、絞り込みを先に行う。
    /// 並びは指定された座標の順、その中では距離順。
    async fn get_bus_stops_near_stations(
        &self,
        coords: &[(u32, f64, f64)],
        limit_per_station: u32,
        radius_meters: f64,
    ) -> Result<Vec<(u32, Station)>, DomainError> {
        let want = TransportType::Bus as i32;
        let limit = limit_per_station as usize;
        let radius_km = radius_meters / 1000.0;
        let mut out = Vec::new();

        for &(source_g_cd, lat, lon) in coords {
            let hits = index::within_radius(lat, lon, radius_km, want)
                .into_iter()
                .filter_map(|(record, _distance)| {
                    let line = index::line_by_cd(record.line_cd).filter(|l| l.e_status == 0)?;
                    let mut station = record.to_entity(Some(line));
                    station.line_group_cd = index::first_line_group_cd(record.station_cd);
                    station.has_train_types = station.line_group_cd.is_some();
                    Some((source_g_cd, station))
                })
                .take(limit);
            out.extend(hits);
        }
        Ok(out)
    }

    /// 1. その路線 (station_id 指定時はその駅) に紐づく系統を priority 降順で 1 件選ぶ
    /// 2. その系統の停車駅を sst.id 順で返す
    /// 3. 空なら路線の全駅を e_sort, station_cd 順で返す
    ///
    /// direction_id が 1 か 2 のときは並び順を反転する。
    async fn get_by_line_id(
        &self,
        line_id: u32,
        station_id: Option<u32>,
        direction_id: Option<u32>,
    ) -> Result<Vec<Station>, DomainError> {
        let reverse = matches!(direction_id, Some(1) | Some(2));

        // priority が最大の系統を 1 件選ぶ
        let mut candidates: Vec<(i32, i32)> = Vec::new(); // (priority, line_group_cd)
        for seed in index::stations_by_line(line_id as i32) {
            if let Some(target) = station_id {
                if seed.station_cd != target as i32 {
                    continue;
                }
            }
            for sst in index::sst_by_station(seed.station_cd) {
                let Some(ty) = index::type_by_cd(sst.type_cd) else {
                    continue;
                };
                let prioritized = ty.priority > 0 && sst.pass != Some(1);
                // (priority > 0 かつ通過しない) か、そうでなければ kind が 0/1 のもの
                if !prioritized && !matches!(ty.kind, Some(0) | Some(1)) {
                    continue;
                }
                if let Some(group) = sst.line_group_cd {
                    candidates.push((ty.priority, group));
                }
            }
        }
        // priority の降順
        candidates.sort_by_key(|(priority, _)| std::cmp::Reverse(*priority));

        if let Some(&(_, target_group)) = candidates.first() {
            let mut typed: Vec<(i32, &index::StationRecord, &index::SstRecord)> = Vec::new();
            for sst in index::sst_by_group(target_group) {
                let Some(record) = index::station_by_cd(sst.station_cd) else {
                    continue;
                };
                if record.e_status != 0 {
                    continue;
                }
                if index::type_by_cd(sst.type_cd).is_none() {
                    continue;
                }
                let Some(line) = index::line_by_cd(record.line_cd) else {
                    continue;
                };
                if line.e_status != 0 {
                    continue;
                }
                typed.push((sst.id, record, sst));
            }
            if !typed.is_empty() {
                typed.sort_by_key(|(id, _, _)| *id);
                if reverse {
                    typed.reverse();
                }
                return Ok(typed
                    .into_iter()
                    .map(|(_, record, sst)| {
                        let mut station = record.to_entity(active_line(record.line_cd));
                        if let Some(ty) = index::type_by_cd(sst.type_cd) {
                            apply_train_type(&mut station, sst, ty);
                        }
                        station
                    })
                    .collect());
            }
        }

        // フォールバック: 種別を持たない路線として全駅を返す
        let Some(line) = index::line_by_cd(line_id as i32) else {
            return Ok(Vec::new());
        };
        if line.e_status != 0 {
            return Ok(Vec::new());
        }
        let mut records: Vec<&index::StationRecord> = index::stations_by_line(line_id as i32)
            .filter(|r| r.e_status == 0)
            .collect();
        records.sort_by(|a, b| {
            a.e_sort
                .cmp(&b.e_sort)
                .then_with(|| a.station_cd.cmp(&b.station_cd))
        });
        if reverse {
            records.reverse();
        }
        Ok(records
            .into_iter()
            .map(|record| {
                let mut station = record.to_entity(Some(line));
                station.line_group_cd = index::first_line_group_cd(record.station_cd);
                station.has_train_types = station.line_group_cd.is_some();
                station
            })
            .collect())
    }
    /// 指定された路線の有効な駅を返す。並びは指定された路線の順、
    /// その中では e_sort, station_cd の昇順。
    async fn get_by_line_id_vec(&self, line_ids: &[u32]) -> Result<Vec<Station>, DomainError> {
        let mut out = Vec::new();
        for &line_id in line_ids {
            let Some(line) = index::line_by_cd(line_id as i32) else {
                continue;
            };
            if line.e_status != 0 {
                continue;
            }
            let mut records: Vec<_> = index::stations_by_line(line_id as i32)
                .filter(|s| s.e_status == 0)
                .collect();
            records.sort_by(|a, b| {
                a.e_sort
                    .cmp(&b.e_sort)
                    .then_with(|| a.station_cd.cmp(&b.station_cd))
            });
            for record in records {
                let mut station = record.to_entity(Some(line));
                // has_train_types 用に系統を 1 件だけ引く
                station.line_group_cd = index::first_line_group_cd(record.station_cd);
                station.has_train_types = station.line_group_cd.is_some();
                out.push(station);
            }
        }
        Ok(out)
    }
    /// 指定路線の駅が属する駅グループの全駅 (他路線の駅も含む) を返す。
    /// 並べ替えは UseCase 側が行う。
    async fn get_by_line_id_vec_with_group_stations(
        &self,
        line_ids: &[u32],
    ) -> Result<Vec<Station>, DomainError> {
        let mut group_ids: Vec<i32> = Vec::new();
        let mut seen: HashSet<i32> = HashSet::new();
        for &line_id in line_ids {
            for record in index::stations_by_line(line_id as i32) {
                if record.e_status == 0 && seen.insert(record.station_g_cd) {
                    group_ids.push(record.station_g_cd);
                }
            }
        }

        let mut out = Vec::new();
        for group_id in group_ids {
            for record in index::stations_by_group(group_id) {
                if record.e_status != 0 {
                    continue;
                }
                let Some(line) = index::line_by_cd(record.line_cd) else {
                    continue;
                };
                if line.e_status != 0 {
                    continue;
                }
                let mut station = record.to_entity(Some(line));
                station.line_group_cd = index::first_line_group_cd(record.station_cd);
                station.has_train_types = station.line_group_cd.is_some();
                out.push(station);
            }
        }
        Ok(out)
    }
    async fn get_by_line_group_id(&self, line_group_id: u32) -> Result<Vec<Station>, DomainError> {
        Ok(stations_of_line_groups(&[line_group_id]))
    }

    async fn get_by_line_group_id_vec(
        &self,
        line_group_ids: &[u32],
    ) -> Result<Vec<Station>, DomainError> {
        Ok(stations_of_line_groups(line_group_ids))
    }
    /// 発着の双方に停車する系統の停車駅を、路線をまたいだまま sst.id 順で返す。
    ///
    /// 1. 発着の駅グループそれぞれで、有効な駅 (e_status == 0) の
    ///    通過ではない停車を持つ系統を集める
    /// 2. 双方に共通する系統だけを残す (via 指定時はさらに路線で絞る)
    /// 3. その系統の停車駅のうち、有効な駅・路線で種別を引けるものを sst.id 順で返す
    ///
    /// 呼び出し側 (`get_routes` / `get_train_types`) は系統に属する停車駅しか使わない。
    /// 系統に属さない駅は経路候補を構成しないので、ここでは集めない。
    async fn get_route_stops(
        &self,
        from_station_id: u32,
        to_station_id: u32,
        via_line_ids: &[u32],
    ) -> Result<Vec<Station>, DomainError> {
        // 双方の駅に通過ではない停車を持つ系統
        let stopping_groups = |group_id: u32| -> HashSet<i32> {
            index::stations_by_group(group_id as i32)
                .filter(|s| s.e_status == 0)
                .flat_map(|s| index::sst_by_station(s.station_cd))
                .filter(|sst| sst.pass != Some(1))
                .filter_map(|sst| sst.line_group_cd)
                .collect()
        };
        let from_stopping = stopping_groups(from_station_id);
        let to_stopping = stopping_groups(to_station_id);

        let mut stops: Vec<(&index::SstRecord, &index::StationRecord, &index::TypeRecord)> =
            Vec::new();
        for group in from_stopping.intersection(&to_stopping) {
            for sst in index::sst_by_group(*group) {
                let Some(record) = index::station_by_cd(sst.station_cd) else {
                    continue;
                };
                if record.e_status != 0 {
                    continue;
                }
                if !via_line_ids.is_empty() && !via_line_ids.contains(&(record.line_cd as u32)) {
                    continue;
                }
                let Some(line) = index::line_by_cd(record.line_cd) else {
                    continue;
                };
                if line.e_status != 0 {
                    continue;
                }
                // 種別が引けない系統は落とす
                let Some(train_type) = index::type_by_cd(sst.type_cd) else {
                    continue;
                };
                stops.push((sst, record, train_type));
            }
        }
        stops.sort_by_key(|(sst, _, _)| sst.id);

        Ok(stops
            .into_iter()
            .map(|(sst, record, train_type)| {
                let mut station = record.to_entity(index::line_by_cd(record.line_cd));
                apply_train_type(&mut station, sst, train_type);
                station
            })
            .collect())
    }

    async fn get_route_stops_by_station_cd(
        &self,
        from_station_cd: u32,
        to_station_cd: u32,
        via_line_ids: &[u32],
        direction_id: Option<u32>,
    ) -> Result<Vec<Station>, DomainError> {
        let reverse = direction_id == Some(1);
        let via_ok =
            |line_cd: i32| via_line_ids.is_empty() || via_line_ids.contains(&(line_cd as u32));

        let (Some(from_record), Some(to_record)) = (
            index::station_by_cd(from_station_cd as i32),
            index::station_by_cd(to_station_cd as i32),
        ) else {
            return Ok(Vec::new());
        };

        // 双方の駅に通過ではない停車を持つ系統
        let groups_of = |station_cd: i32| -> HashSet<i32> {
            index::sst_by_station(station_cd)
                .filter(|sst| sst.pass != Some(1))
                .filter_map(|sst| sst.line_group_cd)
                .collect()
        };
        let common_groups: Vec<i32> = groups_of(from_station_cd as i32)
            .intersection(&groups_of(to_station_cd as i32))
            .copied()
            .collect();

        let mut excluded: HashSet<i32> = HashSet::new();
        for group in &common_groups {
            for sst in index::sst_by_group(*group) {
                excluded.insert(sst.station_cd);
            }
        }

        // --- untyped: common_lines 上で種別経路に含まれない駅 ---
        // station_cd は一意なので common_lines は「両駅が同じ line_cd を持つか」に帰着する
        let mut untyped: Vec<&index::StationRecord> = Vec::new();
        if from_record.e_status == 0
            && to_record.e_status == 0
            && from_record.line_cd == to_record.line_cd
            && via_ok(from_record.line_cd)
            && active_line(from_record.line_cd).is_some()
        {
            untyped.extend(
                index::stations_by_line(from_record.line_cd)
                    .filter(|r| r.e_status == 0 && !excluded.contains(&r.station_cd)),
            );
        }
        untyped.sort_by(|a, b| {
            a.e_sort
                .cmp(&b.e_sort)
                .then_with(|| a.station_cd.cmp(&b.station_cd))
        });
        if reverse {
            untyped.reverse();
        }

        // --- 種別経路に含まれる駅 (種別と路線の両方が引けるもの) ---
        let mut typed: Vec<(i32, &index::StationRecord, &index::SstRecord)> = Vec::new();
        for group in &common_groups {
            for sst in index::sst_by_group(*group) {
                let Some(record) = index::station_by_cd(sst.station_cd) else {
                    continue;
                };
                if record.e_status != 0 || !via_ok(record.line_cd) {
                    continue;
                }
                if index::type_by_cd(sst.type_cd).is_none() {
                    continue;
                }
                let Some(line) = index::line_by_cd(record.line_cd) else {
                    continue;
                };
                if line.e_status != 0 {
                    continue;
                }
                typed.push((sst.id, record, sst));
            }
        }
        typed.sort_by_key(|(id, _, _)| *id);
        if reverse {
            typed.reverse();
        }

        // 種別なしの駅の後に種別ありの駅を連結する
        let mut out: Vec<Station> = untyped
            .into_iter()
            .map(|record| record.to_entity(active_line(record.line_cd)))
            .collect();
        for (_, record, sst) in typed {
            let mut station = record.to_entity(active_line(record.line_cd));
            if let Some(ty) = index::type_by_cd(sst.type_cd) {
                apply_train_type(&mut station, sst, ty);
            }
            out.push(station);
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------- 路線

#[derive(Clone, Default)]
pub struct MemLineRepository;

/// - 系統を 1 つも持たない駅 -> 通す
/// - 系統を持つ駅 -> 停車する系統が 1 つでもあれば通す
fn passes_stop_condition(station_cd: i32) -> bool {
    let mut has_group = false;
    for sst in index::sst_by_station(station_cd) {
        if sst.line_group_cd.is_some() {
            has_group = true;
            if sst.pass != Some(1) {
                return true;
            }
        }
    }
    !has_group
}

/// 駅グループに属する各駅の所属路線を、駅の識別子付きで返す。
/// UseCase 層は `line.station_g_cd` で駅に紐付けるため、ここを埋める必要がある。
fn lines_of_groups(group_ids: &[u32]) -> Vec<Line> {
    let mut out = Vec::new();
    for &gid in group_ids {
        for record in index::stations_by_group(gid as i32) {
            if record.e_status != 0 {
                continue;
            }
            let Some(line) = index::line_by_cd(record.line_cd) else {
                continue;
            };
            // 無効化された路線は返さない (例: 成田エクスプレスは e_status = 3)
            if line.e_status != 0 {
                continue;
            }
            if !passes_stop_condition(record.station_cd) {
                continue;
            }
            let mut line = line.clone();
            line.station_cd = Some(record.station_cd);
            line.station_g_cd = Some(record.station_g_cd);
            index::apply_line_alias(&mut line, record.station_cd);
            out.push(line);
        }
    }
    out
}

#[async_trait]
impl LineRepository for MemLineRepository {
    /// 通過のみの系統しか持たない駅を除く
    async fn get_by_station_group_id_vec(
        &self,
        station_group_id_vec: &[u32],
    ) -> Result<Vec<Line>, DomainError> {
        Ok(lines_of_groups(station_group_id_vec))
    }

    /// no_types 版が省くのはネストした駅に載せる種別情報だけで、返す路線の
    /// 集合は types 版と同じ。通過のみの系統しか持たない駅をここで通すと、
    /// 中央線(快速) の代々木のように停車しない路線が lines に出てしまう。
    async fn get_by_station_group_id_vec_no_types(
        &self,
        station_group_id_vec: &[u32],
    ) -> Result<Vec<Line>, DomainError> {
        Ok(lines_of_groups(station_group_id_vec))
    }

    async fn get_by_station_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Line>, DomainError> {
        self.get_by_station_group_id_vec(&[station_group_id]).await
    }

    /// 無効な路線は返さない
    async fn find_by_id(&self, id: u32) -> Result<Option<Line>, DomainError> {
        Ok(active_line(id as i32).cloned())
    }

    /// 無効な路線は ID を指定されても返さない。
    /// 並びは指定された ID の順で、同じ ID を複数渡されても 1 件だけ返す。
    async fn get_by_ids(&self, ids: &[u32]) -> Result<Vec<Line>, DomainError> {
        let mut seen = HashSet::with_capacity(ids.len());
        Ok(ids
            .iter()
            .filter(|id| seen.insert(**id))
            .filter_map(|&id| active_line(id as i32))
            .cloned()
            .collect())
    }

    /// 停車する系統があれば line_group_cd / type_cd を埋める。無ければ未設定のまま。
    /// 無効な路線も返す。
    async fn find_by_station_id(&self, station_id: u32) -> Result<Option<Line>, DomainError> {
        let Some(record) = index::station_by_cd(station_id as i32) else {
            return Ok(None);
        };
        let Some(line) = index::line_by_cd(record.line_cd) else {
            return Ok(None);
        };
        let mut line = line.clone();
        line.station_cd = Some(record.station_cd);
        line.station_g_cd = Some(record.station_g_cd);
        index::apply_line_alias(&mut line, record.station_cd);
        if let Some(sst) = index::sst_by_station(record.station_cd).find(|s| s.pass != Some(1)) {
            line.line_group_cd = sst.line_group_cd;
            line.type_cd = Some(sst.type_cd);
        }
        Ok(Some(line))
    }

    async fn get_by_line_group_id(&self, line_group_id: u32) -> Result<Vec<Line>, DomainError> {
        self.get_by_line_group_id_vec(&[line_group_id]).await
    }

    /// 指定した系統に停車する有効な駅の、有効な所属路線を返す。
    async fn get_by_line_group_id_vec(
        &self,
        line_group_id_vec: &[u32],
    ) -> Result<Vec<Line>, DomainError> {
        let mut out = Vec::new();
        for &group_id in line_group_id_vec {
            for sst in index::sst_by_group(group_id as i32) {
                if sst.pass == Some(1) {
                    continue;
                }
                let Some(station) = index::station_by_cd(sst.station_cd) else {
                    continue;
                };
                if station.e_status != 0 {
                    continue;
                }
                // l.line_cd = s.line_cd AND l.e_status = 0
                let Some(line) = index::line_by_cd(station.line_cd) else {
                    continue;
                };
                if line.e_status != 0 {
                    continue;
                }
                let mut line = line.clone();
                line.line_group_cd = sst.line_group_cd;
                line.type_cd = Some(sst.type_cd);
                line.station_cd = Some(station.station_cd);
                line.station_g_cd = Some(station.station_g_cd);
                index::apply_line_alias(&mut line, station.station_cd);
                out.push(line);
            }
        }
        Ok(out)
    }
    /// `get_by_line_group_id_vec` とほぼ同じだが、(sst.id, line_cd) で重複を除き
    /// その順に並べる。
    async fn get_by_line_group_id_vec_for_routes(
        &self,
        line_group_id_vec: &[u32],
    ) -> Result<Vec<Line>, DomainError> {
        // (sst.id, line_cd, Line) を並べ替えてから重複を落とす
        let mut rows: Vec<(i32, i32, Line)> = Vec::new();
        for &group_id in line_group_id_vec {
            for sst in index::sst_by_group(group_id as i32) {
                if sst.pass == Some(1) {
                    continue;
                }
                let Some(station) = index::station_by_cd(sst.station_cd) else {
                    continue;
                };
                if station.e_status != 0 {
                    continue;
                }
                let Some(line) = index::line_by_cd(station.line_cd) else {
                    continue;
                };
                if line.e_status != 0 {
                    continue;
                }
                let mut line = line.clone();
                line.line_group_cd = sst.line_group_cd;
                line.type_cd = Some(sst.type_cd);
                line.station_cd = Some(station.station_cd);
                line.station_g_cd = Some(station.station_g_cd);
                index::apply_line_alias(&mut line, station.station_cd);
                rows.push((sst.id, line.line_cd, line));
            }
        }
        rows.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        rows.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
        Ok(rows.into_iter().map(|(_, _, line)| line).collect())
    }
    /// 駅名検索と違い正規化は行わず、全列に同じ部分一致を当てる。
    /// line_name_rn も大小を区別する。並びは CSV の順のまま。
    async fn get_by_name(
        &self,
        line_name: String,
        limit: Option<u32>,
    ) -> Result<Vec<Line>, DomainError> {
        let limit = limit.unwrap_or(1) as usize;
        let mut out = Vec::new();
        for line in index::lines() {
            if out.len() >= limit {
                break;
            }
            if line.e_status != 0 {
                continue;
            }
            let hit = line.line_name.contains(&line_name)
                || index::line_name_rn(line.line_cd).is_some_and(|v| v.contains(&line_name))
                || line.line_name_k.contains(&line_name)
                || line
                    .line_name_zh
                    .as_deref()
                    .is_some_and(|v| v.contains(&line_name))
                || line
                    .line_name_ko
                    .as_deref()
                    .is_some_and(|v| v.contains(&line_name));
            if hit {
                out.push(line.clone());
            }
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------- 事業者

#[derive(Clone, Default)]
pub struct MemCompanyRepository;

#[async_trait]
impl CompanyRepository for MemCompanyRepository {
    /// 事業者は 179 件と少ないが、駅ごとの付帯情報を組み立てるたびに呼ばれる。
    /// `id_vec.contains` のままだと 1 回の呼び出しで (事業者数 × 要求 ID 数) の
    /// 比較になるため、集合に入れてから引く。
    async fn find_by_id_vec(&self, id_vec: &[u32]) -> Result<Vec<Company>, DomainError> {
        let wanted: HashSet<u32> = id_vec.iter().copied().collect();
        Ok(index::companies()
            .iter()
            .filter(|c| wanted.contains(&(c.company_cd as u32)))
            .cloned()
            .collect())
    }
}

// ---------------------------------------------------------------- 列車種別

/// 列車種別。types.csv と station_station_types.csv を索引から引く。
#[derive(Clone, Default)]
pub struct MemTrainTypeRepository;

/// SstRecord + TypeRecord から TrainType を組み立てる。
fn build_train_type(sst: &index::SstRecord, ty: &index::TypeRecord) -> TrainType {
    TrainType {
        id: Some(sst.id),
        station_cd: Some(sst.station_cd),
        type_cd: Some(sst.type_cd),
        line_group_cd: sst.line_group_cd,
        pass: sst.pass,
        type_name: ty.type_name.clone(),
        type_name_k: ty.type_name_k.clone(),
        type_name_r: ty.type_name_r.clone(),
        type_name_zh: ty.type_name_zh.clone(),
        type_name_ko: ty.type_name_ko.clone(),
        color: ty.color.clone(),
        direction: ty.direction,
        line: None,
        lines: vec![],
        kind: ty.kind,
    }
}

/// 共通条件: 駅が有効で、通過駅 (pass = 1) ではないこと。
fn sst_is_stop(sst: &index::SstRecord) -> bool {
    if sst.pass == Some(1) {
        return false;
    }
    index::station_by_cd(sst.station_cd).is_some_and(|s| s.e_status == 0)
}

/// priority の降順、次に sst.id の昇順で並べる。
///
/// priority は各駅停車を先頭に出すためのもの (普通・各駅停車・快速・
/// アクセス特急だけが 1 以上)。種別を一覧として見せる場面で使う。
fn sort_by_priority_then_id(items: &mut [(TrainType, i32)]) {
    items.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| a.0.id.unwrap_or(0).cmp(&b.0.id.unwrap_or(0)))
    });
}

#[async_trait]
impl TrainTypeRepository for MemTrainTypeRepository {
    /// line_group_id が None のときは結果が空になる。
    async fn get_types_by_station_id_vec(
        &self,
        station_id_vec: &[u32],
        line_group_id: Option<u32>,
    ) -> Result<Vec<TrainType>, DomainError> {
        let Some(target_group) = line_group_id.map(|v| v as i32) else {
            return Ok(Vec::new());
        };

        let mut scored: Vec<(TrainType, i32)> = Vec::new();
        for &station_id in station_id_vec {
            for sst in index::sst_by_station(station_id as i32) {
                if sst.line_group_cd != Some(target_group) || !sst_is_stop(sst) {
                    continue;
                }
                let Some(ty) = index::type_by_cd(sst.type_cd) else {
                    continue;
                };
                scored.push((build_train_type(sst, ty), ty.priority));
            }
        }
        sort_by_priority_then_id(&mut scored);
        Ok(scored.into_iter().map(|(t, _)| t).collect())
    }

    /// line_group_id が指定されればその系統に限定し、無ければ駅の全種別を返す。
    async fn get_by_station_id_vec(
        &self,
        station_id_vec: &[u32],
        line_group_id: Option<u32>,
    ) -> Result<Vec<TrainType>, DomainError> {
        let target_group = line_group_id.map(|v| v as i32);

        let mut out: Vec<TrainType> = Vec::new();
        for &station_id in station_id_vec {
            for sst in index::sst_by_station(station_id as i32) {
                if let Some(group) = target_group {
                    if sst.line_group_cd != Some(group) {
                        continue;
                    }
                }
                if !sst_is_stop(sst) {
                    continue;
                }
                let Some(ty) = index::type_by_cd(sst.type_cd) else {
                    continue;
                };
                out.push(build_train_type(sst, ty));
            }
        }
        out.sort_by_key(|t| t.id.unwrap_or(0));
        Ok(out)
    }

    async fn get_by_station_id(&self, station_id: u32) -> Result<Vec<TrainType>, DomainError> {
        let mut out: Vec<TrainType> = index::sst_by_station(station_id as i32)
            .filter(|sst| sst_is_stop(sst))
            .filter_map(|sst| index::type_by_cd(sst.type_cd).map(|ty| build_train_type(sst, ty)))
            .collect();
        out.sort_by_key(|t| t.id.unwrap_or(0));
        Ok(out)
    }

    async fn get_by_line_group_id(
        &self,
        line_group_id: u32,
    ) -> Result<Vec<TrainType>, DomainError> {
        self.get_by_line_group_id_vec(&[line_group_id]).await
    }

    async fn get_by_line_group_id_vec(
        &self,
        line_group_id_vec: &[u32],
    ) -> Result<Vec<TrainType>, DomainError> {
        let targets: Vec<i32> = line_group_id_vec.iter().map(|&v| v as i32).collect();
        let mut out: Vec<TrainType> = index::ssts()
            .iter()
            .filter(|sst| sst.line_group_cd.is_some_and(|g| targets.contains(&g)))
            .filter(|sst| sst_is_stop(sst))
            .filter_map(|sst| index::type_by_cd(sst.type_cd).map(|ty| build_train_type(sst, ty)))
            .collect();
        out.sort_by_key(|t| t.id.unwrap_or(0));
        Ok(out)
    }

    async fn find_by_line_group_id_and_line_id(
        &self,
        line_group_id: u32,
        line_id: u32,
    ) -> Result<Option<TrainType>, DomainError> {
        let target_line = line_id as i32;
        // その系統に属し、指定路線の有効な駅にあたる行のうち sst.id が最小のもの。
        // 通過駅かどうかは見ない。
        // 全 SST の走査ではなく、系統の索引から辿る (sst.id 昇順で返る)
        Ok(index::sst_by_group(line_group_id as i32)
            .filter(|sst| {
                index::station_by_cd(sst.station_cd)
                    .is_some_and(|s| s.line_cd == target_line && s.e_status == 0)
            })
            .find_map(|sst| index::type_by_cd(sst.type_cd).map(|ty| build_train_type(sst, ty))))
    }

    async fn find_by_line_group_id_and_line_id_vec(
        &self,
        pairs: &[(u32, u32)],
    ) -> Result<HashMap<(u32, u32), TrainType>, DomainError> {
        let mut out = HashMap::with_capacity(pairs.len());
        for &(line_group_id, line_id) in pairs {
            if let Some(tt) = self
                .find_by_line_group_id_and_line_id(line_group_id, line_id)
                .await?
            {
                out.insert((line_group_id, line_id), tt);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stationapi::domain::route_search::{self, Journey, JourneySort};
    use stationapi::model;

    const TOKYO: u32 = 1130101;
    const SHIBUYA: u32 = 1130205;
    const MITAKA: u32 = 1131105;
    const NAKA_MEGURO: u32 = 2600103;

    /// repository の実装は await しない (索引を引くだけ) ので、1 回 poll すれば終わる
    fn block_on<F: std::future::Future>(future: F) -> F::Output {
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        match std::pin::pin!(future).poll(&mut context) {
            std::task::Poll::Ready(value) => value,
            std::task::Poll::Pending => panic!("repository futures complete without waiting"),
        }
    }

    fn names_by_line(from: u32, name: &str) -> Vec<(String, i32, bool)> {
        block_on(MemStationRepository.get_by_name(name.to_string(), Some(100), Some(from), None))
            .unwrap()
            .into_iter()
            .map(|s| (s.station_name, s.line_cd, s.has_train_types))
            .collect()
    }

    /// 探索結果の経路を estimateArrivalTimes / trainRoute の legs にする
    /// (connectedRoutes が区間ごとに返す乗降駅と、探索が選んだ系統)
    fn journey_legs(journey: &Journey) -> Vec<model::RouteLegRequest> {
        journey
            .legs
            .iter()
            .map(|leg| model::RouteLegRequest {
                line_group_id: leg.line_group_id,
                from_station_id: leg.station_cds[0] as u32,
                to_station_id: *leg.station_cds.last().unwrap() as u32,
            })
            .collect()
    }

    fn station_ids_of(eta: &[stationapi::domain::arrival_estimation::EstimatedStop]) -> Vec<i32> {
        eta.iter().map(|stop| stop.station_cd).collect()
    }

    #[test]
    fn connected_route_eta_and_train_route_follow_the_legs() {
        use stationapi::use_case::traits::query::QueryUseCase;
        let interactor = crate::interactor();
        // 大宮 → 新大阪 (はやぶさ → 東京 → のぞみ など) と、山手線の継ぎ目を
        // 跨ぎうる東京 → 渋谷
        for (from, to) in [(1131906, 1160213), (TOKYO, SHIBUYA)] {
            let journeys = route_network().search(from, to, None);
            assert!(!journeys.is_empty());
            for journey in &journeys {
                let legs = journey_legs(journey);
                let eta =
                    block_on(interactor.estimate_connected_route_arrival_times(&legs)).unwrap();
                let train_route = block_on(interactor.get_connected_train_route(&legs)).unwrap();

                // 同じ区間を同じ弧で切り出す
                let eta_ids = station_ids_of(&eta);
                let train_route_ids: Vec<i32> = train_route
                    .iter()
                    .map(|segment| segment.station.as_ref().unwrap().id as i32)
                    .collect();
                assert_eq!(eta_ids, train_route_ids);
                // 探索の区間と同じ駅を通る
                let journey_ids: Vec<i32> = journey
                    .legs
                    .iter()
                    .flat_map(|leg| leg.station_cds.iter().copied())
                    .collect();
                assert_eq!(eta_ids, journey_ids);
                // 累積は減らず、最後は探索の所要時間と (ほぼ) 一致する
                assert!(eta.windows(2).all(|pair| {
                    pair[1].cumulative_minutes >= pair[0].departure_cumulative_minutes - 1e-9
                }));
                let last = eta.last().unwrap().cumulative_minutes;
                let expected = f64::from(journey.total_seconds) / 60.0;
                assert!(
                    (last - expected).abs() < 1.0,
                    "eta {last} vs search {expected}"
                );
                // 乗換では徒歩の後に乗換先の列車を待ち、走行区間は距離 0 から始まる。
                // 乗車駅の行は、前の区間の降車駅の行のすぐ後
                let mut board = 0;
                for leg in &journey.legs[..journey.legs.len() - 1] {
                    board += leg.station_cds.len();
                    assert!(
                        eta[board].departure_cumulative_minutes > eta[board].cumulative_minutes
                    );
                    assert!(train_route[board].distance_from_previous == 0.0);
                }
            }
        }
    }

    #[test]
    fn connected_route_legs_accept_any_train_type_of_the_leg() {
        use stationapi::use_case::traits::query::QueryUseCase;
        let interactor = crate::interactor();
        // 区間の乗降駅は中央線 (快速) の三鷹 (1131220) と新宿 (1131211) だが、
        // 中央・総武線の各停 (系統 585) を選んでも、同じ駅グループにある各停の駅で
        // 切り出す
        let legs = [model::RouteLegRequest {
            line_group_id: 585,
            from_station_id: 1131220,
            to_station_id: 1131211,
        }];
        let eta = block_on(interactor.estimate_connected_route_arrival_times(&legs)).unwrap();
        let group_of = |station_cd: i32| index::station_by_cd(station_cd).unwrap().station_g_cd;
        let (first, last) = (eta[0].station_cd, eta.last().unwrap().station_cd);
        assert_eq!((group_of(first), group_of(last)), (MITAKA as i32, 1130208));
        assert!(
            first != 1131220 && last != 1131211,
            "the local's own stations"
        );
        let train_route = block_on(interactor.get_connected_train_route(&legs)).unwrap();
        assert_eq!(train_route.len(), eta.len());

        // connectedRoutes の区間の trainTypes は、どれを選んでも区間の乗降駅で使える
        let routes = block_on(interactor.get_connected_routes(
            MITAKA,
            NAKA_MEGURO,
            None,
            JourneySort::Recommended,
        ))
        .unwrap();
        for route in &routes {
            // stationGroupIds は乗車駅の駅グループから降車駅の駅グループまでの並び
            for leg in &route.legs {
                assert!(leg.station_group_ids.len() >= 2);
                assert_eq!(
                    (
                        leg.station_group_ids.first().copied(),
                        leg.station_group_ids.last().copied()
                    ),
                    (
                        Some(leg.from_station.group_id),
                        Some(leg.to_station.group_id)
                    )
                );
            }
            let choices = route
                .legs
                .iter()
                .map(|leg| leg.train_types.len())
                .max()
                .unwrap();
            for choice in 0..choices {
                let legs: Vec<model::RouteLegRequest> = route
                    .legs
                    .iter()
                    .map(|leg| model::RouteLegRequest {
                        line_group_id: leg.train_types[choice.min(leg.train_types.len() - 1)]
                            .group_id,
                        from_station_id: leg.from_station.id,
                        to_station_id: leg.to_station.id,
                    })
                    .collect();
                let eta =
                    block_on(interactor.estimate_connected_route_arrival_times(&legs)).unwrap();
                let train_route = block_on(interactor.get_connected_train_route(&legs)).unwrap();
                assert_eq!(eta.len(), train_route.len());
            }
        }
    }

    #[test]
    fn connected_route_sort_only_reorders_the_recommended_routes() {
        use stationapi::use_case::traits::query::QueryUseCase;
        let interactor = crate::interactor();
        // 三鷹 → 中目黒、東京 → 渋谷、大宮 → 新大阪
        for (from, to) in [(MITAKA, NAKA_MEGURO), (TOKYO, SHIBUYA), (1131906, 1160213)] {
            let recommended = route_network().search(from, to, None);
            assert!(!recommended.is_empty());
            let mut by_arrival = recommended.clone();
            route_search::sort_journeys(&mut by_arrival, JourneySort::ArrivalTime);
            assert!(by_arrival
                .windows(2)
                .all(|w| (w[0].total_seconds, w[0].transfer_count())
                    <= (w[1].total_seconds, w[1].transfer_count())));
            let mut by_transfers = recommended.clone();
            route_search::sort_journeys(&mut by_transfers, JourneySort::TransferCount);
            assert!(by_transfers
                .windows(2)
                .all(|w| (w[0].transfer_count(), w[0].total_seconds)
                    <= (w[1].transfer_count(), w[1].total_seconds)));

            // connectedRoutes も同じ集合を並べ替えるだけ
            let routes =
                |sort| block_on(interactor.get_connected_routes(from, to, None, sort)).unwrap();
            let base = routes(JourneySort::Recommended);
            for sort in [JourneySort::ArrivalTime, JourneySort::TransferCount] {
                let sorted = routes(sort);
                assert_eq!(sorted.len(), base.len());
                assert!(sorted.iter().all(|route| base.contains(route)));
            }
            let transfers: Vec<usize> = routes(JourneySort::TransferCount)
                .iter()
                .map(|route| route.legs.len())
                .collect();
            assert!(transfers.windows(2).all(|w| w[0] <= w[1]), "{transfers:?}");
        }
    }

    #[test]
    fn connected_route_rejects_legs_that_do_not_connect() {
        use stationapi::use_case::traits::query::QueryUseCase;
        let interactor = crate::interactor();
        let routes = block_on(interactor.get_connected_routes(
            MITAKA,
            NAKA_MEGURO,
            None,
            JourneySort::Recommended,
        ))
        .unwrap();
        let mut legs: Vec<model::RouteLegRequest> = routes[0]
            .legs
            .iter()
            .map(|leg| model::RouteLegRequest {
                line_group_id: leg.train_types[0].group_id,
                from_station_id: leg.from_station.id,
                to_station_id: leg.to_station.id,
            })
            .collect();
        assert!(legs.len() > 1);
        // 2 区間目を飛ばすと、1 区間目の降車駅と 3 区間目の乗車駅がつながらない
        legs.remove(1);
        let error = block_on(interactor.estimate_connected_route_arrival_times(&legs))
            .unwrap_err()
            .to_string();
        assert!(error.contains("区間がつながっていません"), "{error}");
        assert!(block_on(interactor.get_connected_train_route(&legs)).is_err());
        assert!(block_on(interactor.get_connected_train_route(&[])).is_err());

        // connectedRoutes が返しうる乗車回数 (MAX_RIDES) を超える区間は断る。
        // つながった区間 (三鷹と新宿を中央線快速で往復) でも受け付けない
        let back_and_forth: Vec<model::RouteLegRequest> = (0..=route_search::MAX_RIDES)
            .map(|index| {
                let (from, to) = if index % 2 == 0 {
                    (1131220, 1131211)
                } else {
                    (1131211, 1131220)
                };
                model::RouteLegRequest {
                    line_group_id: 20,
                    from_station_id: from,
                    to_station_id: to,
                }
            })
            .collect();
        let error = block_on(interactor.estimate_connected_route_arrival_times(&back_and_forth))
            .unwrap_err()
            .to_string();
        assert!(error.contains("区間までにしてください"), "{error}");
        assert!(block_on(interactor.get_connected_train_route(&back_and_forth)).is_err());
        // 上限ちょうどは受け付ける
        let at_limit = &back_and_forth[..route_search::MAX_RIDES];
        assert!(block_on(interactor.get_connected_train_route(at_limit)).is_ok());
    }

    #[test]
    fn route_topology_matches_the_topology_inside_the_route_network() {
        // 行き先の検索は軽い網、connectedRoutes は所要時間つきの網を使う。
        // 両者がずれると「行ける」と返した駅で経路が出なくなる
        assert_eq!(route_topology(), route_network().topology());
    }

    #[test]
    fn stations_by_name_includes_stations_reached_by_transfer() {
        // 三鷹から中目黒へは直通の系統が無いが、乗り換えれば東横線でも日比谷線でも着く
        let found = names_by_line(MITAKA, "中目黒");
        assert!(found.contains(&("中目黒".to_string(), 26001, false)));
        assert!(found.contains(&("中目黒".to_string(), 28003, false)));

        // 返した駅には、その路線を viaLineId にした connectedRoutes で経路がある
        let network = route_network();
        for (_, line_cd, _) in &found {
            assert!(!network
                .search(MITAKA, NAKA_MEGURO, Some(*line_cd))
                .is_empty());
        }
    }

    #[test]
    fn stations_by_name_skips_a_branch_junction_reached_only_by_backtracking() {
        // 石橋阪大前は宝塚線 (34002) と、そこから出る箕面線 (34007) の駅。
        // 箕面線の石橋阪大前に箕面線で着くには、一度箕面線へ出て戻るしかない
        let found = names_by_line(MITAKA, "石橋阪大前");
        let lines: Vec<i32> = found.iter().map(|(_, line_cd, _)| *line_cd).collect();
        assert_eq!(lines, vec![34002]);
    }

    #[test]
    fn stations_by_name_keeps_direct_stations_marked_as_sharing_a_line_group() {
        // 東京から品川は山手線などで直通なので、共有する系統が付く
        let found = names_by_line(TOKYO, "品川");
        assert!(found
            .iter()
            .any(|(name, _, has_train_types)| name == "品川" && *has_train_types));
    }

    #[test]
    fn route_network_finds_direct_and_transfer_routes_in_real_data() {
        let network = build_route_network();
        assert!(network.pattern_count() > 0);

        // 山手線で乗り換えずに行ける
        let journeys = network.search(TOKYO, SHIBUYA, None);
        assert!(journeys.iter().any(|journey| journey.transfer_count() == 0));

        // 直通の系統が無く、乗換が要る (旧実装は探索の上限に先に達して 0 件だった)
        let journeys = network.search(MITAKA, NAKA_MEGURO, None);
        assert!(!journeys.is_empty());
        assert!(journeys.iter().all(|journey| journey.transfer_count() > 0));
        for journey in &journeys {
            assert_eq!(journey.legs[0].station_group_ids[0], MITAKA);
            assert_eq!(
                journey.legs.last().unwrap().station_group_ids.last(),
                Some(&NAKA_MEGURO)
            );
            // 区間はつながっている
            for pair in journey.legs.windows(2) {
                assert_eq!(
                    pair[0].station_group_ids.last(),
                    pair[1].station_group_ids.first()
                );
            }
        }
    }
}
