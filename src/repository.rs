//! `stationapi::domain::repository` の各トレイトをインメモリ索引で実装する。
//!
//! これにより UseCase 層 (`QueryInteractor`) を一切変更せずに Workers 上で動かせる。
//! PoC では座標検索・名前検索の経路で呼ばれるメソッドだけを実装し、
//! 残りは明示的にエラーを返す (黙って空を返すと未実装が正常応答に見えるため)。

use async_trait::async_trait;
use std::collections::{HashMap, HashSet};

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

/// 指定した系統の停車駅を返す。並びは指定された系統の順、各系統内は sst.id 昇順。
/// 駅・路線・種別のいずれかが引けない行は落とす。
fn stations_of_line_groups(group_ids: &[u32]) -> Vec<Station> {
    let mut out = Vec::new();
    for &group_id in group_ids {
        for sst in index::sst_by_group(group_id as i32) {
            let Some(record) = index::station_by_cd(sst.station_cd) else {
                continue;
            };
            if record.e_status != 0 {
                continue;
            }
            let Some(line) = index::line_by_cd(record.line_cd) else {
                continue;
            };
            if line.e_status != 0 {
                continue;
            }
            // 種別が引けない系統は落とす
            let Some(ty) = index::type_by_cd(sst.type_cd) else {
                continue;
            };
            let mut station = record.to_entity(Some(line));
            apply_train_type(&mut station, sst, ty);
            out.push(station);
        }
    }
    out
}

// ---------------------------------------------------------------- 駅

#[derive(Clone, Default)]
pub struct MemStationRepository;

#[async_trait]
impl StationRepository for MemStationRepository {
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
    /// 「その駅から乗り換えなしで行けるか」で絞り込む。条件は次のどちらか。
    ///
    /// - 出発駅と同じ系統に、通過ではない停車として含まれる
    /// - 出発駅か目的駅のどちらかが系統を持たず、かつ同じ路線にある
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
            if shared_group.is_none() && !same_line {
                continue;
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

    /// 各座標につきバス停の最寄り N 件を取り、そのあと有効な路線を持つものだけに絞る。
    /// 先に路線で絞ると件数が変わるため、この順序を保つ。
    /// 並びは指定された座標の順、その中では距離順。
    async fn get_bus_stops_near_stations(
        &self,
        coords: &[(u32, f64, f64)],
        limit_per_station: u32,
    ) -> Result<Vec<(u32, Station)>, DomainError> {
        let want = Some(TransportType::Bus as i32);
        let limit = limit_per_station as usize;
        let mut out = Vec::new();

        for &(source_g_cd, lat, lon) in coords {
            for (record, _distance) in index::nearest_without_line_join(lat, lon, limit, want) {
                let Some(line) = index::line_by_cd(record.line_cd) else {
                    continue;
                };
                if line.e_status != 0 {
                    continue;
                }
                let mut station = record.to_entity(Some(line));
                station.line_group_cd = index::first_line_group_cd(record.station_cd);
                station.has_train_types = station.line_group_cd.is_some();
                out.push((source_g_cd, station));
            }
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
    /// 1. from と to の両方に有効な駅がある路線を集める (via 指定時はさらに絞る)
    /// 2. from と to の双方に停車する系統を集める
    /// 3. その系統に含まれない駅 (= 各駅停車として扱う駅) だけを残す
    ///
    /// 出発・到着の駅グループを引く段階では e_status を見ない。
    async fn get_route_stops(
        &self,
        from_station_id: u32,
        to_station_id: u32,
        via_line_ids: &[u32],
    ) -> Result<Vec<Station>, DomainError> {
        // common_lines (e_status = 0 が条件、via 指定があれば限定)
        let from_lines: HashSet<i32> = index::stations_by_group(from_station_id as i32)
            .filter(|s| s.e_status == 0)
            .filter(|s| via_line_ids.is_empty() || via_line_ids.contains(&(s.line_cd as u32)))
            .map(|s| s.line_cd)
            .collect();
        let to_lines: HashSet<i32> = index::stations_by_group(to_station_id as i32)
            .filter(|s| s.e_status == 0)
            .map(|s| s.line_cd)
            .collect();
        let common_lines: Vec<i32> = from_lines.intersection(&to_lines).copied().collect();
        if common_lines.is_empty() {
            return Ok(Vec::new());
        }

        // sst_cte_c1 / sst_cte_c2 (from_cte / to_cte は e_status を見ない)
        let groups_of = |group_id: u32| -> HashSet<i32> {
            index::stations_by_group(group_id as i32)
                .flat_map(|s| index::sst_by_station(s.station_cd))
                .filter(|sst| sst.pass != Some(1))
                .filter_map(|sst| sst.line_group_cd)
                .collect()
        };
        let from_groups = groups_of(from_station_id);
        let to_groups = groups_of(to_station_id);

        // sst_cte に現れる station_cd = 除外対象 (sst.line_group_cd IS NULL の否定)
        let mut excluded: HashSet<i32> = HashSet::new();
        for group in from_groups.intersection(&to_groups) {
            for sst in index::sst_by_group(*group) {
                excluded.insert(sst.station_cd);
            }
        }

        let mut records: Vec<&index::StationRecord> = Vec::new();
        for line_cd in common_lines {
            let Some(line) = index::line_by_cd(line_cd) else {
                continue;
            };
            if line.e_status != 0 {
                continue;
            }
            for record in index::stations_by_line(line_cd) {
                if record.e_status != 0 || excluded.contains(&record.station_cd) {
                    continue;
                }
                records.push(record);
            }
        }

        // e_sort, station_cd の昇順
        records.sort_by(|a, b| {
            a.e_sort
                .cmp(&b.e_sort)
                .then_with(|| a.station_cd.cmp(&b.station_cd))
        });

        Ok(records
            .into_iter()
            .map(|record| record.to_entity(index::line_by_cd(record.line_cd)))
            .collect())
    }
    /// 2 つの結果を連結して返す。
    /// 1. 種別経路に含まれない駅。e_sort, station_cd の昇順
    /// 2. 種別経路に含まれる駅。sst.id の昇順
    ///
    /// direction_id = 1 のときは両方の並び順を反転する。
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

        // sst_cte: 双方の駅に pass <> 1 で停車する line_group_cd
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
/// `require_stop` は通過のみの系統しか持たない駅を除くかどうか。
fn lines_of_groups_inner(group_ids: &[u32], require_stop: bool) -> Vec<Line> {
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
            if require_stop && !passes_stop_condition(record.station_cd) {
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
        Ok(lines_of_groups_inner(station_group_id_vec, true))
    }

    /// no_types 版は通過条件を掛けない
    async fn get_by_station_group_id_vec_no_types(
        &self,
        station_group_id_vec: &[u32],
    ) -> Result<Vec<Line>, DomainError> {
        Ok(lines_of_groups_inner(station_group_id_vec, false))
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

    /// 無効な路線は ID を指定されても返さない。並びは指定された ID の順。
    async fn get_by_ids(&self, ids: &[u32]) -> Result<Vec<Line>, DomainError> {
        Ok(ids
            .iter()
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
    async fn find_by_id_vec(&self, id_vec: &[u32]) -> Result<Vec<Company>, DomainError> {
        Ok(index::companies()
            .iter()
            .filter(|c| id_vec.contains(&(c.company_cd as u32)))
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

        let mut scored: Vec<(TrainType, i32)> = Vec::new();
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
                scored.push((build_train_type(sst, ty), ty.priority));
            }
        }
        sort_by_priority_then_id(&mut scored);
        Ok(scored.into_iter().map(|(t, _)| t).collect())
    }

    /// 並びは sst.id のみで priority を見ない
    /// (`get_by_station_id_vec` 側は priority 降順なので分けている)。
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
        let mut scored: Vec<(TrainType, i32)> = index::ssts()
            .iter()
            .filter(|sst| sst.line_group_cd.is_some_and(|g| targets.contains(&g)))
            .filter(|sst| sst_is_stop(sst))
            .filter_map(|sst| {
                index::type_by_cd(sst.type_cd).map(|ty| (build_train_type(sst, ty), ty.priority))
            })
            .collect();
        sort_by_priority_then_id(&mut scored);
        Ok(scored.into_iter().map(|(t, _)| t).collect())
    }

    async fn get_line_group_ids_by_station_group_ids(
        &self,
        station_group_ids: &[u32],
    ) -> Result<HashMap<u32, Vec<u32>>, DomainError> {
        let mut out: HashMap<u32, Vec<u32>> = HashMap::new();
        for &group_id in station_group_ids {
            let mut groups: Vec<u32> = Vec::new();
            for record in index::stations_by_group(group_id as i32) {
                if record.e_status != 0 {
                    continue;
                }
                for sst in index::sst_by_station(record.station_cd) {
                    if !sst_is_stop(sst) {
                        continue;
                    }
                    if let Some(lg) = sst.line_group_cd {
                        let lg = lg as u32;
                        if !groups.contains(&lg) {
                            groups.push(lg);
                        }
                    }
                }
            }
            if !groups.is_empty() {
                out.insert(group_id, groups);
            }
        }
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
