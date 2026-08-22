//! `stationapi::domain::repository` の各トレイトをインメモリ索引で実装する。
//!
//! これにより UseCase 層 (`QueryInteractor`) を一切変更せずに Workers 上で動かせる。
//! PoC では座標検索・名前検索の経路で呼ばれるメソッドだけを実装し、
//! 残りは明示的にエラーを返す (黙って空を返すと未実装が正常応答に見えるため)。

use async_trait::async_trait;
use std::collections::HashMap;

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

use crate::index;

fn todo_err(what: &'static str) -> DomainError {
    DomainError::Unexpected(format!("{what}: PoC では未実装"))
}

/// 駅グループ ID 群に属する有効な駅を、路線を JOIN した Station として返す。
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
            out.push(record.to_entity(Some(line)));
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
        _transport_type: Option<TransportType>,
    ) -> Result<Vec<Station>, DomainError> {
        // 既存 SQL の LIMIT $3 と同じく未指定なら 1 件
        let limit = limit.unwrap_or(1).min(1_000) as usize;
        Ok(index::nearest(latitude, longitude, limit)
            .into_iter()
            .map(|(record, distance_km)| {
                let mut station = record.to_entity(index::line_by_cd(record.line_cd));
                station.distance = Some(distance_km * 1000.0);
                station
            })
            .collect())
    }

    async fn get_by_name(
        &self,
        station_name: String,
        limit: Option<u32>,
        _from_station_group_id: Option<u32>,
        _transport_type: Option<TransportType>,
    ) -> Result<Vec<Station>, DomainError> {
        // NOTE: 既存は limit 未指定で LIMIT NULL (全件)。PoC でも実質全件を許す。
        let limit = limit.unwrap_or(u32::MAX).min(10_000) as usize;
        Ok(index::search_by_name(&station_name, limit)
            .into_iter()
            .map(|record| record.to_entity(index::line_by_cd(record.line_cd)))
            .collect())
    }

    async fn get_by_station_group_id_vec(
        &self,
        station_group_id_vec: &[u32],
    ) -> Result<Vec<Station>, DomainError> {
        Ok(stations_of_groups(station_group_id_vec))
    }

    // 列車種別を未実装のため、types を JOIN する版としない版は同じ結果になる
    async fn get_by_station_group_id_vec_no_types(
        &self,
        station_group_id_vec: &[u32],
    ) -> Result<Vec<Station>, DomainError> {
        Ok(stations_of_groups(station_group_id_vec))
    }

    async fn get_by_station_group_id(
        &self,
        station_group_id: u32,
    ) -> Result<Vec<Station>, DomainError> {
        Ok(stations_of_groups(&[station_group_id]))
    }

    async fn find_by_id(&self, id: u32) -> Result<Option<Station>, DomainError> {
        Ok(index::station_by_cd(id as i32)
            .filter(|r| r.e_status == 0)
            .map(|r| r.to_entity(index::line_by_cd(r.line_cd))))
    }

    async fn get_by_id_vec(&self, ids: &[u32]) -> Result<Vec<Station>, DomainError> {
        Ok(ids
            .iter()
            .filter_map(|&id| index::station_by_cd(id as i32))
            .filter(|r| r.e_status == 0)
            .map(|r| r.to_entity(index::line_by_cd(r.line_cd)))
            .collect())
    }

    /// バス機能は方針どおり後回し。鉄道のみのデータでは常に空。
    async fn get_bus_stops_near_stations(
        &self,
        _coords: &[(u32, f64, f64)],
        _limit_per_station: u32,
    ) -> Result<Vec<(u32, Station)>, DomainError> {
        Ok(Vec::new())
    }

    async fn get_by_line_id(
        &self,
        _line_id: u32,
        _station_id: Option<u32>,
        _direction_id: Option<u32>,
    ) -> Result<Vec<Station>, DomainError> {
        Err(todo_err("get_by_line_id"))
    }
    async fn get_by_line_id_vec(&self, _line_ids: &[u32]) -> Result<Vec<Station>, DomainError> {
        Err(todo_err("get_by_line_id_vec"))
    }
    async fn get_by_line_id_vec_with_group_stations(
        &self,
        _line_ids: &[u32],
    ) -> Result<Vec<Station>, DomainError> {
        Err(todo_err("get_by_line_id_vec_with_group_stations"))
    }
    async fn get_by_line_group_id(&self, _line_group_id: u32) -> Result<Vec<Station>, DomainError> {
        Err(todo_err("get_by_line_group_id"))
    }
    async fn get_by_line_group_id_vec(
        &self,
        _line_group_ids: &[u32],
    ) -> Result<Vec<Station>, DomainError> {
        Err(todo_err("get_by_line_group_id_vec"))
    }
    async fn get_route_stops(
        &self,
        _from_station_id: u32,
        _to_station_id: u32,
        _via_line_ids: &[u32],
    ) -> Result<Vec<Station>, DomainError> {
        Err(todo_err("get_route_stops"))
    }
    async fn get_route_stops_by_station_cd(
        &self,
        _from_station_cd: u32,
        _to_station_cd: u32,
        _via_line_ids: &[u32],
        _direction_id: Option<u32>,
    ) -> Result<Vec<Station>, DomainError> {
        Err(todo_err("get_route_stops_by_station_cd"))
    }
}

// ---------------------------------------------------------------- 路線

#[derive(Clone, Default)]
pub struct MemLineRepository;

/// 駅グループに属する各駅の所属路線を、JOIN 結果と同じく駅の識別子付きで返す。
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
            let mut line = line.clone();
            line.station_cd = Some(record.station_cd);
            line.station_g_cd = Some(record.station_g_cd);
            out.push(line);
        }
    }
    out
}

#[async_trait]
impl LineRepository for MemLineRepository {
    async fn get_by_station_group_id_vec(
        &self,
        station_group_id_vec: &[u32],
    ) -> Result<Vec<Line>, DomainError> {
        Ok(lines_of_groups(station_group_id_vec))
    }

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
        Ok(lines_of_groups(&[station_group_id]))
    }

    async fn find_by_id(&self, id: u32) -> Result<Option<Line>, DomainError> {
        Ok(index::line_by_cd(id as i32).cloned())
    }

    async fn get_by_ids(&self, ids: &[u32]) -> Result<Vec<Line>, DomainError> {
        Ok(ids
            .iter()
            .filter_map(|&id| index::line_by_cd(id as i32))
            .cloned()
            .collect())
    }

    async fn find_by_station_id(&self, station_id: u32) -> Result<Option<Line>, DomainError> {
        Ok(index::station_by_cd(station_id as i32)
            .and_then(|r| index::line_by_cd(r.line_cd))
            .cloned())
    }

    async fn get_by_line_group_id(&self, _line_group_id: u32) -> Result<Vec<Line>, DomainError> {
        Err(todo_err("LineRepository::get_by_line_group_id"))
    }
    async fn get_by_line_group_id_vec(
        &self,
        _line_group_id_vec: &[u32],
    ) -> Result<Vec<Line>, DomainError> {
        Err(todo_err("LineRepository::get_by_line_group_id_vec"))
    }
    async fn get_by_line_group_id_vec_for_routes(
        &self,
        _line_group_id_vec: &[u32],
    ) -> Result<Vec<Line>, DomainError> {
        Err(todo_err("get_by_line_group_id_vec_for_routes"))
    }
    async fn get_by_name(
        &self,
        _line_name: String,
        _limit: Option<u32>,
    ) -> Result<Vec<Line>, DomainError> {
        Err(todo_err("LineRepository::get_by_name"))
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

/// 列車種別は types.csv と station_station_types.csv の取り込みが必要なため次段階。
/// 空を返すと UseCase 層は「種別なし」として扱い、駅・路線・事業者の付与は正常に動く。
#[derive(Clone, Default)]
pub struct MemTrainTypeRepository;

#[async_trait]
impl TrainTypeRepository for MemTrainTypeRepository {
    async fn get_types_by_station_id_vec(
        &self,
        _station_id_vec: &[u32],
        _line_group_id: Option<u32>,
    ) -> Result<Vec<TrainType>, DomainError> {
        Ok(Vec::new())
    }
    async fn get_by_station_id_vec(
        &self,
        _station_id_vec: &[u32],
        _line_group_id: Option<u32>,
    ) -> Result<Vec<TrainType>, DomainError> {
        Ok(Vec::new())
    }
    async fn get_by_station_id(&self, _station_id: u32) -> Result<Vec<TrainType>, DomainError> {
        Ok(Vec::new())
    }
    async fn get_by_line_group_id(
        &self,
        _line_group_id: u32,
    ) -> Result<Vec<TrainType>, DomainError> {
        Ok(Vec::new())
    }
    async fn get_by_line_group_id_vec(
        &self,
        _line_group_id_vec: &[u32],
    ) -> Result<Vec<TrainType>, DomainError> {
        Ok(Vec::new())
    }
    async fn find_by_line_group_id_and_line_id(
        &self,
        _line_group_id: u32,
        _line_id: u32,
    ) -> Result<Option<TrainType>, DomainError> {
        Ok(None)
    }
    async fn find_by_line_group_id_and_line_id_vec(
        &self,
        _pairs: &[(u32, u32)],
    ) -> Result<HashMap<(u32, u32), TrainType>, DomainError> {
        Ok(HashMap::new())
    }
}
