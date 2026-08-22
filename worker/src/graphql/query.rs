//! Query リゾルバ。公開スキーマの 18 クエリを提供する。
//!
//! 各リゾルバは UseCase 層を呼び、返ってきた domain エンティティを
//! proto へ変換してから GraphQL 型にする。proto を経由するのは、
//! IPA や TTS セグメントの計算が use_case の DTO 側にあるため。

use async_graphql::{Context, Object, Result as GqlResult};
use stationapi::domain::entity::gtfs::TransportTypeFilter;
use stationapi::proto;
use stationapi::use_case::traits::query::QueryUseCase;

use super::enums::TransportType as GqlTransportType;
use super::scalar::UInt32;
use super::types::*;
use crate::Interactor;

/// GraphQL の TransportType を UseCase のフィルタへ変換する。
///
/// gRPC 版は未指定を Rail (鉄道のみ) として扱うが、Worker 版は RailAndBus を
/// 既定とする。バスを含めた結果を既定で返したいため、意図的に挙動を変えている。
/// TransportTypeUnspecified が明示された場合も同じ扱いにする。
///
/// 座標検索の SQL は種別未指定のとき
/// `ORDER BY CASE WHEN $4 IS NULL THEN transport_type ELSE 0 END, 距離`
/// となるため、鉄道が先・バスが後に並んだうえで距離順になる。
fn to_filter(value: Option<GqlTransportType>) -> TransportTypeFilter {
    match value {
        Some(GqlTransportType::Rail) => TransportTypeFilter::Rail,
        Some(GqlTransportType::Bus) => TransportTypeFilter::Bus,
        _ => TransportTypeFilter::RailAndBus,
    }
}

/// GraphQL の Int は 32bit 符号付きなので負値が渡りうる。
/// `as u32` で丸めると -1 が 4294967295 になり、上限件数を引き出せてしまう。
fn to_limit(value: Option<i32>) -> Result<Option<u32>, async_graphql::Error> {
    match value {
        None => Ok(None),
        Some(v) if v < 0 => Err(async_graphql::Error::new(
            "limit には 0 以上を指定してください",
        )),
        Some(v) => Ok(Some(v as u32)),
    }
}

/// 識別子も同様に負値を弾く。丸めると別のレコードを指しうるため。
fn to_id(value: i32, name: &str) -> Result<u32, async_graphql::Error> {
    if value < 0 {
        return Err(async_graphql::Error::new(format!(
            "{name} には 0 以上を指定してください"
        )));
    }
    Ok(value as u32)
}

fn to_opt_id(value: Option<i32>, name: &str) -> Result<Option<u32>, async_graphql::Error> {
    value.map(|v| to_id(v, name)).transpose()
}

fn use_case<'a>(ctx: &Context<'a>) -> &'a Interactor {
    ctx.data_unchecked::<Interactor>()
}

/// domain エンティティ列を GraphQL の Station 列へ変換する
fn stations_to_gql(stations: Vec<stationapi::domain::entity::station::Station>) -> Vec<Station> {
    stations
        .into_iter()
        .map(|s| Station::from(proto::Station::from(s)))
        .collect()
}

pub struct QueryRoot;

#[Object(name = "Query")]
impl QueryRoot {
    async fn station(
        &self,
        ctx: &Context<'_>,
        id: i32,
        transport_type: Option<GqlTransportType>,
    ) -> GqlResult<Option<Station>> {
        let found = use_case(ctx)
            .find_station_by_id(to_id(id, "id")?, to_filter(transport_type))
            .await?;
        Ok(found.map(|s| Station::from(proto::Station::from(s))))
    }

    async fn stations(
        &self,
        ctx: &Context<'_>,
        ids: Vec<i32>,
        transport_type: Option<GqlTransportType>,
    ) -> GqlResult<Vec<Station>> {
        let ids: Vec<u32> = ids
            .into_iter()
            .map(|v| to_id(v, "ids"))
            .collect::<Result<_, _>>()?;
        let found = use_case(ctx)
            .get_stations_by_id_vec(&ids, to_filter(transport_type))
            .await?;
        Ok(stations_to_gql(found))
    }

    async fn stations_nearby(
        &self,
        ctx: &Context<'_>,
        latitude: f64,
        longitude: f64,
        limit: Option<i32>,
        transport_type: Option<GqlTransportType>,
    ) -> GqlResult<Vec<Station>> {
        let found = use_case(ctx)
            .get_stations_by_coordinates(
                latitude,
                longitude,
                to_limit(limit)?,
                to_filter(transport_type),
            )
            .await?;
        Ok(stations_to_gql(found))
    }

    async fn stations_by_name(
        &self,
        ctx: &Context<'_>,
        name: String,
        limit: Option<i32>,
        from_station_group_id: Option<i32>,
        transport_type: Option<GqlTransportType>,
    ) -> GqlResult<Vec<Station>> {
        let found = use_case(ctx)
            .get_stations_by_name(
                name,
                to_limit(limit)?,
                from_station_group_id.map(|v| v as u32),
                to_filter(transport_type),
            )
            .await?;
        Ok(stations_to_gql(found))
    }

    async fn station_group_stations(
        &self,
        ctx: &Context<'_>,
        group_id: i32,
        transport_type: Option<GqlTransportType>,
    ) -> GqlResult<Vec<Station>> {
        let found = use_case(ctx)
            .get_stations_by_group_id(to_id(group_id, "groupId")?, to_filter(transport_type))
            .await?;
        Ok(stations_to_gql(found))
    }

    async fn line_group_stations(
        &self,
        ctx: &Context<'_>,
        line_group_id: i32,
        // NOTE: 公開スキーマにはあるが UseCase 側に対応する引数が無い
        #[graphql(name = "directionId")] _direction_id: Option<i32>,
        transport_type: Option<GqlTransportType>,
    ) -> GqlResult<Vec<Station>> {
        let found = use_case(ctx)
            .get_stations_by_line_group_id(
                to_id(line_group_id, "lineGroupId")?,
                to_filter(transport_type),
            )
            .await?;
        Ok(stations_to_gql(found))
    }

    async fn line(&self, ctx: &Context<'_>, line_id: i32) -> GqlResult<Option<Line>> {
        let found = use_case(ctx)
            .find_line_by_id(to_id(line_id, "lineId")?)
            .await?;
        Ok(found.map(|l| Line::from(proto::Line::from(l))))
    }

    async fn lines(&self, ctx: &Context<'_>, line_ids: Vec<i32>) -> GqlResult<Vec<Line>> {
        let ids: Vec<u32> = line_ids
            .into_iter()
            .map(|v| to_id(v, "lineIds"))
            .collect::<Result<_, _>>()?;
        let found = use_case(ctx).get_lines_by_id_vec(&ids).await?;
        Ok(found
            .into_iter()
            .map(|l| Line::from(proto::Line::from(l)))
            .collect())
    }

    async fn lines_by_name(
        &self,
        ctx: &Context<'_>,
        name: String,
        limit: Option<i32>,
    ) -> GqlResult<Vec<Line>> {
        let found = use_case(ctx)
            .get_lines_by_name(name, limit.map(|v| v as u32))
            .await?;
        Ok(found
            .into_iter()
            .map(|l| Line::from(proto::Line::from(l)))
            .collect())
    }

    async fn line_stations(
        &self,
        ctx: &Context<'_>,
        line_id: i32,
        station_id: Option<i32>,
        direction_id: Option<i32>,
        transport_type: Option<GqlTransportType>,
    ) -> GqlResult<Vec<Station>> {
        let found = use_case(ctx)
            .get_stations_by_line_id(
                to_id(line_id, "lineId")?,
                to_opt_id(station_id, "stationId")?,
                to_opt_id(direction_id, "directionId")?,
                to_filter(transport_type),
            )
            .await?;
        Ok(stations_to_gql(found))
    }

    async fn line_list_stations(
        &self,
        ctx: &Context<'_>,
        line_ids: Vec<i32>,
        transport_type: Option<GqlTransportType>,
    ) -> GqlResult<Vec<Station>> {
        let ids: Vec<u32> = line_ids
            .into_iter()
            .map(|v| to_id(v, "lineIds"))
            .collect::<Result<_, _>>()?;
        let found = use_case(ctx)
            .get_stations_by_line_id_vec(&ids, to_filter(transport_type))
            .await?;
        Ok(stations_to_gql(found))
    }

    async fn line_group_list_stations(
        &self,
        ctx: &Context<'_>,
        line_group_ids: Vec<i32>,
        transport_type: Option<GqlTransportType>,
    ) -> GqlResult<Vec<Station>> {
        let ids: Vec<u32> = line_group_ids
            .into_iter()
            .map(|v| to_id(v, "lineGroupIds"))
            .collect::<Result<_, _>>()?;
        let found = use_case(ctx)
            .get_stations_by_line_group_id_vec(&ids, to_filter(transport_type))
            .await?;
        Ok(stations_to_gql(found))
    }

    async fn station_train_types(
        &self,
        ctx: &Context<'_>,
        station_id: i32,
    ) -> GqlResult<Vec<TrainType>> {
        let found = use_case(ctx)
            .get_train_types_by_station_id(to_id(station_id, "stationId")?)
            .await?;
        Ok(found
            .into_iter()
            .map(|t| TrainType::from(proto::TrainType::from(t)))
            .collect())
    }

    async fn routes(
        &self,
        ctx: &Context<'_>,
        from_station_group_id: i32,
        to_station_group_id: i32,
        via_line_id: Option<i32>,
        // NOTE: ページングは未対応。公開スキーマに合わせて引数だけ受ける
        #[graphql(name = "pageSize")] _page_size: Option<i32>,
        #[graphql(name = "pageToken")] _page_token: Option<String>,
    ) -> GqlResult<RoutePage> {
        let found = use_case(ctx)
            .get_routes(
                to_id(from_station_group_id, "fromStationGroupId")?,
                to_id(to_station_group_id, "toStationGroupId")?,
                to_opt_id(via_line_id, "viaLineId")?,
            )
            .await?;
        Ok(RoutePage {
            routes: Some(found.into_iter().map(Into::into).collect()),
            next_page_token: Some(String::new()),
        })
    }

    async fn route_types(
        &self,
        ctx: &Context<'_>,
        from_station_group_id: i32,
        to_station_group_id: i32,
        via_line_id: Option<i32>,
        #[graphql(name = "pageSize")] _page_size: Option<i32>,
        #[graphql(name = "pageToken")] _page_token: Option<String>,
    ) -> GqlResult<RouteTypePage> {
        let found = use_case(ctx)
            .get_train_types(
                to_id(from_station_group_id, "fromStationGroupId")?,
                to_id(to_station_group_id, "toStationGroupId")?,
                to_opt_id(via_line_id, "viaLineId")?,
            )
            .await?;
        Ok(RouteTypePage {
            train_types: Some(
                found
                    .into_iter()
                    .map(|t| TrainType::from(proto::TrainType::from(t)))
                    .collect(),
            ),
            next_page_token: Some(String::new()),
        })
    }

    async fn connected_routes(
        &self,
        ctx: &Context<'_>,
        from_station_group_id: i32,
        to_station_group_id: i32,
    ) -> GqlResult<Vec<Route>> {
        let found = use_case(ctx)
            .get_connected_routes(
                to_id(from_station_group_id, "fromStationGroupId")?,
                to_id(to_station_group_id, "toStationGroupId")?,
            )
            .await?;
        Ok(found.into_iter().map(Into::into).collect())
    }

    async fn estimate_arrival_times(
        &self,
        ctx: &Context<'_>,
        from_station_id: i32,
        to_station_id: i32,
        via_line_ids: Option<Vec<i32>>,
        direction_id: Option<i32>,
    ) -> GqlResult<EstimatedArrivalPage> {
        let via: Vec<u32> = via_line_ids
            .unwrap_or_default()
            .into_iter()
            .map(|v| to_id(v, "viaLineIds"))
            .collect::<Result<_, _>>()?;
        let stops = use_case(ctx)
            .estimate_route_arrival_times(
                to_id(from_station_id, "fromStationId")?,
                to_id(to_station_id, "toStationId")?,
                &via,
                to_opt_id(direction_id, "directionId")?,
            )
            .await?;

        // presentation 層と同じ畳み込み: line_group_cd が連続する区間を 1 ルートにまとめる
        let mut routes: Vec<EstimatedArrivalRoute> = Vec::new();
        for stop in &stops {
            let gql_stop = EstimatedArrivalStop {
                station_id: Some(stop.station_cd),
                station_group_id: Some(UInt32(stop.station_g_cd as u32)),
                cumulative_minutes: Some(stop.cumulative_minutes),
                stops_here: Some(stop.stops_here),
                departure_cumulative_minutes: Some(stop.departure_cumulative_minutes),
            };
            let route_id = stop.line_group_cd.unwrap_or(0);
            let merge = stop.line_group_cd.is_some()
                && routes.last().is_some_and(|r| r.id == Some(route_id));

            if merge {
                if let Some(last) = routes.last_mut() {
                    if let Some(list) = last.stops.as_mut() {
                        list.push(gql_stop);
                    }
                }
            } else {
                routes.push(EstimatedArrivalRoute {
                    id: Some(route_id),
                    stops: Some(vec![gql_stop]),
                });
            }
        }
        Ok(EstimatedArrivalPage {
            routes: Some(routes),
        })
    }

    async fn train_route(
        &self,
        ctx: &Context<'_>,
        from_station_id: i32,
        to_station_id: i32,
        line_group_id: Option<i32>,
    ) -> GqlResult<TrainRouteResponse> {
        let segments = use_case(ctx)
            .get_train_route(
                to_id(from_station_id, "fromStationId")?,
                to_id(to_station_id, "toStationId")?,
                to_opt_id(line_group_id, "lineGroupId")?,
            )
            .await?;
        Ok(TrainRouteResponse {
            segments: Some(segments.into_iter().map(Into::into).collect()),
        })
    }
}
