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
use stationapi::proto::StopCondition;

use crate::index;

/// 有効な (e_status = 0) 路線だけを返す。
/// 多くの SQL が `l.e_status = 0` を条件に持つため、無効な路線を混ぜないようにする。
fn active_line(line_cd: i32) -> Option<&'static Line> {
    index::line_by_cd(line_cd).filter(|l| l.e_status == 0)
}

/// 駅グループ ID 群に属する有効な駅を、路線を JOIN した Station として返す。
/// 対応する SQL は `JOIN lines l ... AND l.e_status = 0` なので無効な路線は除く。
fn stations_of_groups(group_ids: &[u32]) -> Vec<Station> {
    let mut out = Vec::new();
    for &gid in group_ids {
        for record in index::stations_by_group(gid as i32) {
            if record.e_status != 0 {
                continue;
            }
            // stations JOIN lines (INNER) 相当
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

/// `JOIN station_station_types sst` と `JOIN types t` の結果を Station に反映する。
/// 既存の `StationRow -> Station` 変換と同じ対応付け。
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

/// 既存 SQL:
/// `FROM stations s JOIN lines l ON l.line_cd = s.line_cd AND l.e_status = 0`
/// `JOIN sst ON sst.line_group_cd IN (..) AND sst.station_cd = s.station_cd`
/// `JOIN types t ON t.type_cd = sst.type_cd WHERE s.e_status = 0`
/// `ORDER BY CASE sst.line_group_cd .. END, sst.id`
///
/// 入力順に走査し、各グループ内は sst.id 昇順なので ORDER BY と一致する。
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
            // JOIN types なので種別が引けない sst は落ちる
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
        // 既存 SQL の LIMIT $3 と同じく未指定なら 1 件
        let limit = limit.unwrap_or(1).min(1_000) as usize;
        let want = transport_type.map(|t| t as i32);
        Ok(index::nearest(latitude, longitude, limit, want)
            .into_iter()
            .map(|(record, distance_km)| {
                // NOTE: 座標検索の SQL は JOIN lines のみで l.e_status を見ないため、
                // ここでは無効な路線も除外しない
                let mut station = record.to_entity(index::line_by_cd(record.line_cd));
                station.distance = Some(distance_km * 1000.0);
                // 既存 SQL は has_train_types 用に line_group_cd をサブクエリで引く
                station.line_group_cd = index::first_line_group_cd(record.station_cd);
                station.has_train_types = station.line_group_cd.is_some();
                station
            })
            .collect())
    }

    async fn get_by_name(
        &self,
        station_name: String,
        limit: Option<u32>,
        _from_station_group_id: Option<u32>,
        transport_type: Option<TransportType>,
    ) -> Result<Vec<Station>, DomainError> {
        // NOTE: 既存は limit 未指定で LIMIT NULL (全件)。PoC でも実質全件を許す。
        let limit = limit.unwrap_or(u32::MAX).min(10_000) as usize;
        let want = transport_type.map(|t| t as i32);
        Ok(index::search_by_name(&station_name, limit, want)
            .into_iter()
            .map(|record| record.to_entity(index::line_by_cd(record.line_cd)))
            .collect())
    }

    /// 既存 SQL は `LEFT JOIN station_station_types sst` と `LEFT JOIN types t` を行い、
    /// 駅に紐づく種別ごとに行を返す。種別を持たない駅は sst 側が NULL の 1 行になる。
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

                // LEFT JOIN なので、種別を持つ駅は sst の数だけ行が出る
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

    /// 既存 SQL は types を JOIN しない代わりに、has_train_types 用の
    /// `line_group_cd` をサブクエリで 1 件だけ引く。
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

    async fn find_by_id(&self, id: u32) -> Result<Option<Station>, DomainError> {
        Ok(index::station_by_cd(id as i32)
            .filter(|r| r.e_status == 0)
            .filter(|r| active_line(r.line_cd).is_some())
            .map(|r| r.to_entity(active_line(r.line_cd))))
    }

    async fn get_by_id_vec(&self, ids: &[u32]) -> Result<Vec<Station>, DomainError> {
        Ok(ids
            .iter()
            .filter_map(|&id| index::station_by_cd(id as i32))
            .filter(|r| r.e_status == 0)
            .filter(|r| active_line(r.line_cd).is_some())
            .map(|r| r.to_entity(active_line(r.line_cd)))
            .collect())
    }

    /// 既存 SQL:
    /// 各座標につき LATERAL で `transport_type = Bus` の最寄り N 件を取り、
    /// そのあと `JOIN lines ... AND l.e_status = 0` で絞る。
    /// 先に路線で絞ると件数が変わるため、この順序を保つ。
    /// 並びは `ORDER BY ic.source_g_cd, 距離`。
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

    /// 既存実装 (`get_by_line_id_with_train_type`) の写し。
    ///
    /// 1. target_line_group: その路線 (station_id 指定時はその駅) に紐づく
    ///    line_group_cd を priority 降順で 1 件選ぶ
    /// 2. その系統の停車駅を `ORDER BY sst.id` で返す
    /// 3. 空なら種別なし (`get_by_line_id_without_train_types`) へフォールバックし、
    ///    `ORDER BY s.e_sort, s.station_cd` で路線の全駅を返す
    ///
    /// direction_id が 1 か 2 のときは並び順を反転する。
    async fn get_by_line_id(
        &self,
        line_id: u32,
        station_id: Option<u32>,
        direction_id: Option<u32>,
    ) -> Result<Vec<Station>, DomainError> {
        let reverse = matches!(direction_id, Some(1) | Some(2));

        // target_line_group: ORDER BY t.priority DESC LIMIT 1
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
        // ORDER BY t.priority DESC
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
    /// 既存 SQL:
    /// `WHERE l.line_cd IN (..) AND s.e_status = 0 AND l.e_status = 0`
    /// `ORDER BY CASE l.line_cd .. END, s.e_sort ASC, s.station_cd ASC`
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
                // has_train_types 用サブクエリ相当
                station.line_group_cd = index::first_line_group_cd(record.station_cd);
                station.has_train_types = station.line_group_cd.is_some();
                out.push(station);
            }
        }
        Ok(out)
    }
    /// 既存 SQL:
    /// `WHERE s.station_g_cd IN (SELECT DISTINCT s2.station_g_cd FROM stations s2`
    /// `  WHERE s2.line_cd IN (..) AND s2.e_status = 0)`
    /// `AND s.e_status = 0 AND l.e_status = 0`
    ///
    /// 指定路線の駅が属する駅グループの全駅 (他路線の駅も含む) を返す。
    /// ORDER BY は無く、並べ替えは UseCase 側が行う。
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
    /// 既存 SQL (CTE 構成) を素直に写したもの:
    /// - common_lines: from と to の両方に有効な駅がある line_cd (via 指定時はさらに絞る)
    /// - sst_cte: from と to の双方に pass <> 1 で停車する line_group_cd に属する sst
    /// - 最終的に `LEFT JOIN sst_cte ... WHERE sst.line_group_cd IS NULL` で、
    ///   その種別経路に含まれない駅 (= 各駅停車として扱う駅) だけを残す
    ///
    /// from_cte / to_cte には e_status 条件が無い点も SQL に合わせている。
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

        // ORDER BY sta.e_sort, sta.station_cd
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
    /// 既存実装は 2 本のクエリを実行して結果を連結する。
    /// 1. untyped: 種別経路に含まれない駅 (`sst.line_group_cd IS NULL`)、
    ///    `ORDER BY sta.e_sort, sta.station_cd`
    /// 2. typed: 種別経路に含まれる駅 (`JOIN types` が INNER)、`ORDER BY sst.id`
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

        // --- typed: 種別経路に含まれる駅 (types と lines を INNER JOIN) ---
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

        // untyped の後に typed を連結する (既存の rows.append と同じ順序)
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

/// 既存 SQL の
/// `AND ((sst.line_group_cd IS NOT NULL AND sst.pass <> 1) OR sst.line_group_cd IS NULL)`
/// に対応する。LEFT JOIN なので、
/// - 系統を1つも持たない駅 -> 通す (sst 側が NULL)
/// - 系統を持つ駅 -> 停車する系統が1つでもあれば通す
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

/// 駅グループに属する各駅の所属路線を、JOIN 結果と同じく駅の識別子付きで返す。
/// UseCase 層は `line.station_g_cd` で駅に紐付けるため、ここを埋める必要がある。
/// `require_stop` は types を JOIN する版 (通過のみの系統を除く) かどうか。
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
            // 既存 SQL は WHERE l.e_status = 0。無効化された路線は返さない
            // (例: 成田エクスプレスは e_status = 3)
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
    /// SQL は sst を LEFT JOIN し、通過のみの系統しか持たない駅を除く
    async fn get_by_station_group_id_vec(
        &self,
        station_group_id_vec: &[u32],
    ) -> Result<Vec<Line>, DomainError> {
        Ok(lines_of_groups_inner(station_group_id_vec, true))
    }

    /// no_types 版は sst を JOIN しないので通過条件は掛からない
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

    /// 既存 SQL は WHERE l.e_status = 0
    async fn find_by_id(&self, id: u32) -> Result<Option<Line>, DomainError> {
        Ok(active_line(id as i32).cloned())
    }

    /// NOTE: get_by_ids / find_by_station_id の SQL には l.e_status 条件が無いため、
    /// 無効な路線も返す (呼び出し側が ID を指定しているため)
    async fn get_by_ids(&self, ids: &[u32]) -> Result<Vec<Line>, DomainError> {
        Ok(ids
            .iter()
            .filter_map(|&id| index::line_by_cd(id as i32))
            .cloned()
            .collect())
    }

    /// 既存 SQL は sst を `LEFT JOIN ... AND sst.pass <> 1` して
    /// line_group_cd / type_cd を返す。停車する系統が無ければ NULL のまま。
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

    /// 既存 SQL:
    /// `FROM lines l JOIN sst ON sst.line_group_cd IN (..) AND sst.pass <> 1`
    /// `JOIN stations s ON s.station_cd = sst.station_cd AND s.e_status = 0`
    /// `WHERE l.line_cd = s.line_cd AND l.e_status = 0`
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
    /// `get_by_line_group_id_vec` とほぼ同じ結合だが、
    /// `DISTINCT ON (sst.id, l.line_cd)` と `ORDER BY sst.id, l.line_cd` が付く。
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
    /// 既存 SQL は駅名検索と違い正規化を行わず、全列に同じ `%入力%` を当てる。
    /// line_name_rn も ILIKE ではなく LIKE (大小を区別する)。
    /// ORDER BY が無いためスキャン順 = CSV の並び順で LIMIT が効く。
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

/// 既存 SQL の共通条件: 駅が有効で、通過駅 (pass = 1) ではないこと。
fn sst_is_stop(sst: &index::SstRecord) -> bool {
    if sst.pass == Some(1) {
        return false;
    }
    index::station_by_cd(sst.station_cd).is_some_and(|s| s.e_status == 0)
}

/// `ORDER BY t.priority DESC, sst.id` 相当で並べる。
fn sort_by_priority_then_id(items: &mut [(TrainType, i32)]) {
    items.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| a.0.id.unwrap_or(0).cmp(&b.0.id.unwrap_or(0)))
    });
}

#[async_trait]
impl TrainTypeRepository for MemTrainTypeRepository {
    /// 既存 SQL は `sst.line_group_cd = $N` を要求するため、
    /// line_group_id が None のとき (= NULL 比較) は結果が空になる。
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

    /// 既存 SQL は `ORDER BY sst.id` のみで priority を見ない
    /// (`get_by_station_id_vec` 側は `priority DESC, sst.id` なので分けている)。
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
        // 全 SST の走査ではなく、系統の索引から辿る (sst.id 昇順で返る)
        Ok(index::sst_by_group(line_group_id as i32)
            .filter(|sst| sst_is_stop(sst))
            .find(|sst| {
                index::station_by_cd(sst.station_cd).is_some_and(|s| s.line_cd == target_line)
            })
            .and_then(|sst| index::type_by_cd(sst.type_cd).map(|ty| build_train_type(sst, ty))))
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
