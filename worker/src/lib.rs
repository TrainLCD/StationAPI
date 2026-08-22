//! StationAPI を Cloudflare Workers 上で動かす PoC。
//!
//! sqlx / tonic を使わず、埋め込み CSV のインメモリ索引と自前の gRPC-Web フレーミングで
//! 既存クライアント互換の応答を返す。UseCase 層 (`QueryInteractor`) は一切変更せず、
//! repository トレイトの実装だけを差し替えている。

mod grpc_web;
mod index;
mod repository;

use prost::Message;
use worker::*;

use stationapi::domain::entity::gtfs::TransportTypeFilter;
use stationapi::proto::{
    GetLineByIdRequest, GetLinesByIdListRequest, GetStationByCoordinatesRequest,
    GetStationByGroupIdRequest, GetStationByIdListRequest, GetStationByIdRequest,
    GetLinesByNameRequest, GetStationByLineIdListRequest, GetStationByLineIdRequest, GetStationsByLineGroupIdListRequest,
    GetStationsByLineGroupIdRequest, GetStationsByNameRequest,
    EstimateArrivalTimesRequest, EstimatedArrivalResponse, EstimatedArrivalRoute,
    EstimatedArrivalStop, GetConnectedStationsRequest, GetRouteRequest, GetTrainRouteRequest,
    GetTrainTypesByStationIdRequest, MultipleLineResponse, MultipleStationResponse,
    MultipleTrainTypeResponse, RouteResponse, RouteTypeResponse, SingleLineResponse,
    SingleStationResponse, TrainRouteResponse, TransportType as GrpcTransportType,
};
use stationapi::use_case::interactor::query::QueryInteractor;
use stationapi::use_case::traits::query::QueryUseCase;

use repository::{
    MemCompanyRepository, MemLineRepository, MemStationRepository, MemTrainTypeRepository,
};

const COORDINATES_PATH: &str = "/app.trainlcd.grpc.StationAPI/GetStationsByCoordinates";
const BY_NAME_PATH: &str = "/app.trainlcd.grpc.StationAPI/GetStationsByName";
const TRAIN_TYPES_PATH: &str = "/app.trainlcd.grpc.StationAPI/GetTrainTypesByStationId";
const BY_ID_PATH: &str = "/app.trainlcd.grpc.StationAPI/GetStationById";
const BY_ID_LIST_PATH: &str = "/app.trainlcd.grpc.StationAPI/GetStationByIdList";
const BY_GROUP_ID_PATH: &str = "/app.trainlcd.grpc.StationAPI/GetStationsByGroupId";
const BY_LINE_GROUP_ID_PATH: &str = "/app.trainlcd.grpc.StationAPI/GetStationsByLineGroupId";
const BY_LINE_GROUP_ID_LIST_PATH: &str =
    "/app.trainlcd.grpc.StationAPI/GetStationsByLineGroupIdList";
const LINE_BY_ID_PATH: &str = "/app.trainlcd.grpc.StationAPI/GetLineById";
const LINES_BY_ID_LIST_PATH: &str = "/app.trainlcd.grpc.StationAPI/GetLinesByIdList";
const LINES_BY_NAME_PATH: &str = "/app.trainlcd.grpc.StationAPI/GetLinesByName";
const BY_LINE_ID_LIST_PATH: &str = "/app.trainlcd.grpc.StationAPI/GetStationsByLineIdList";
const ROUTES_PATH: &str = "/app.trainlcd.grpc.StationAPI/GetRoutes";
const ROUTE_TYPES_PATH: &str = "/app.trainlcd.grpc.StationAPI/GetRouteTypes";
const CONNECTED_ROUTES_PATH: &str = "/app.trainlcd.grpc.StationAPI/GetConnectedRoutes";
const TRAIN_ROUTE_PATH: &str = "/app.trainlcd.grpc.StationAPI/GetTrainRoute";
const ROUTES_MINIMAL_PATH: &str = "/app.trainlcd.grpc.StationAPI/GetRoutesMinimal";
const ESTIMATE_ARRIVAL_PATH: &str = "/app.trainlcd.grpc.StationAPI/EstimateArrivalTimes";
const BY_LINE_ID_PATH: &str = "/app.trainlcd.grpc.StationAPI/GetStationsByLineId";

type Interactor = QueryInteractor<
    MemStationRepository,
    MemLineRepository,
    MemTrainTypeRepository,
    MemCompanyRepository,
>;

/// repository は状態を持たない (索引は OnceLock 側にある) ので毎回生成して問題ない。
fn use_case() -> Interactor {
    QueryInteractor {
        station_repository: MemStationRepository,
        line_repository: MemLineRepository,
        train_type_repository: MemTrainTypeRepository,
        company_repository: MemCompanyRepository,
    }
}

/// presentation 層の convert_transport_type と同じ変換 (未指定は鉄道のみ)
fn convert_transport_type(proto_type: Option<i32>) -> TransportTypeFilter {
    match proto_type.and_then(|v| GrpcTransportType::try_from(v).ok()) {
        Some(GrpcTransportType::Rail) => TransportTypeFilter::Rail,
        Some(GrpcTransportType::Bus) => TransportTypeFilter::Bus,
        Some(GrpcTransportType::RailAndBus) => TransportTypeFilter::RailAndBus,
        _ => TransportTypeFilter::Rail,
    }
}

#[event(fetch)]
async fn fetch(req: Request, _env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    // Workers は 500 の本文を伏せるので、失敗理由をログに残す
    let result = route(req).await;
    if let Err(e) = &result {
        console_error!("request failed: {e}");
    }
    result
}

async fn route(req: Request) -> Result<Response> {
    let method = req.method();
    let path = req.path();

    if method == Method::Options {
        return grpc_web::preflight();
    }
    // 索引の件数確認とウォームアップ用
    if method == Method::Get && path == "/__health" {
        return Response::ok(format!(
            "stations={} lines={} companies={}",
            index::stations().len(),
            index::lines().len(),
            index::companies().len()
        ));
    }
    if method == Method::Post && path == COORDINATES_PATH {
        return handle_get_stations_by_coordinates(req).await;
    }
    if method == Method::Post && path == BY_NAME_PATH {
        return handle_get_stations_by_name(req).await;
    }
    if method == Method::Post && path == TRAIN_TYPES_PATH {
        return handle_get_train_types_by_station_id(req).await;
    }
    if method == Method::Post && path == BY_ID_PATH {
        return handle_get_station_by_id(req).await;
    }
    if method == Method::Post && path == BY_ID_LIST_PATH {
        return handle_get_station_by_id_list(req).await;
    }
    if method == Method::Post && path == BY_GROUP_ID_PATH {
        return handle_get_stations_by_group_id(req).await;
    }
    if method == Method::Post && path == BY_LINE_GROUP_ID_PATH {
        return handle_get_stations_by_line_group_id(req).await;
    }
    if method == Method::Post && path == BY_LINE_GROUP_ID_LIST_PATH {
        return handle_get_stations_by_line_group_id_list(req).await;
    }
    if method == Method::Post && path == LINE_BY_ID_PATH {
        return handle_get_line_by_id(req).await;
    }
    if method == Method::Post && path == LINES_BY_ID_LIST_PATH {
        return handle_get_lines_by_id_list(req).await;
    }
    if method == Method::Post && path == LINES_BY_NAME_PATH {
        return handle_get_lines_by_name(req).await;
    }
    if method == Method::Post && path == BY_LINE_ID_LIST_PATH {
        return handle_get_stations_by_line_id_list(req).await;
    }
    if method == Method::Post && path == ROUTES_PATH {
        return handle_get_routes(req).await;
    }
    if method == Method::Post && path == ROUTE_TYPES_PATH {
        return handle_get_route_types(req).await;
    }
    if method == Method::Post && path == CONNECTED_ROUTES_PATH {
        return handle_get_connected_routes(req).await;
    }
    if method == Method::Post && path == TRAIN_ROUTE_PATH {
        return handle_get_train_route(req).await;
    }
    if method == Method::Post && path == ROUTES_MINIMAL_PATH {
        return handle_get_routes_minimal(req).await;
    }
    if method == Method::Post && path == ESTIMATE_ARRIVAL_PATH {
        return handle_estimate_arrival_times(req).await;
    }
    if method == Method::Post && path == BY_LINE_ID_PATH {
        return handle_get_stations_by_line_id(req).await;
    }

    Response::error("Not Found", 404)
}

async fn handle_get_stations_by_coordinates(mut req: Request) -> Result<Response> {
    let body = req.bytes().await?;
    let payload = grpc_web::decode_frame(&body)?;
    let request = GetStationByCoordinatesRequest::decode(payload)
        .map_err(|e| Error::RustError(format!("protobuf decode failed: {e}")))?;

    let stations = use_case()
        .get_stations_by_coordinates(
            request.latitude,
            request.longitude,
            request.limit,
            convert_transport_type(request.transport_type),
        )
        .await
        .map_err(|e| Error::RustError(format!("{e}")))?;

    grpc_web::encode_response(&MultipleStationResponse {
        stations: stations.into_iter().map(Into::into).collect(),
    })
}

async fn handle_get_stations_by_name(mut req: Request) -> Result<Response> {
    let body = req.bytes().await?;
    let payload = grpc_web::decode_frame(&body)?;
    let request = GetStationsByNameRequest::decode(payload)
        .map_err(|e| Error::RustError(format!("protobuf decode failed: {e}")))?;

    let stations = use_case()
        .get_stations_by_name(
            request.station_name,
            request.limit,
            request.from_station_group_id,
            convert_transport_type(request.transport_type),
        )
        .await
        .map_err(|e| Error::RustError(format!("{e}")))?;

    grpc_web::encode_response(&MultipleStationResponse {
        stations: stations.into_iter().map(Into::into).collect(),
    })
}

async fn handle_get_train_types_by_station_id(mut req: Request) -> Result<Response> {
    let body = req.bytes().await?;
    let payload = grpc_web::decode_frame(&body)?;
    let request = GetTrainTypesByStationIdRequest::decode(payload)
        .map_err(|e| Error::RustError(format!("protobuf decode failed: {e}")))?;

    let train_types = use_case()
        .get_train_types_by_station_id(request.station_id)
        .await
        .map_err(|e| Error::RustError(format!("{e}")))?;

    grpc_web::encode_response(&MultipleTrainTypeResponse {
        train_types: train_types.into_iter().map(Into::into).collect(),
    })
}

/// リクエストボディを取り出して protobuf にデコードする共通処理
async fn decode_request<M: Message + Default>(req: &mut Request) -> Result<M> {
    let body = req.bytes().await?;
    let payload = grpc_web::decode_frame(&body)?;
    M::decode(payload).map_err(|e| Error::RustError(format!("protobuf decode failed: {e}")))
}

/// Workers は 500 の本文を "INTERNAL SERVER ERROR" に伏せるため、
/// 原因を追えるようログにも出す。
fn use_case_err(e: impl std::fmt::Display) -> Error {
    let message = format!("{e}");
    console_error!("use_case error: {message}");
    Error::RustError(message)
}

async fn handle_get_station_by_id(mut req: Request) -> Result<Response> {
    let request: GetStationByIdRequest = decode_request(&mut req).await?;
    let station = use_case()
        .find_station_by_id(request.id, convert_transport_type(request.transport_type))
        .await
        .map_err(use_case_err)?;

    grpc_web::encode_response(&SingleStationResponse {
        station: station.map(Into::into),
    })
}

async fn handle_get_station_by_id_list(mut req: Request) -> Result<Response> {
    let request: GetStationByIdListRequest = decode_request(&mut req).await?;
    let stations = use_case()
        .get_stations_by_id_vec(&request.ids, convert_transport_type(request.transport_type))
        .await
        .map_err(use_case_err)?;

    grpc_web::encode_response(&MultipleStationResponse {
        stations: stations.into_iter().map(Into::into).collect(),
    })
}

async fn handle_get_stations_by_group_id(mut req: Request) -> Result<Response> {
    let request: GetStationByGroupIdRequest = decode_request(&mut req).await?;
    let stations = use_case()
        .get_stations_by_group_id(
            request.group_id,
            convert_transport_type(request.transport_type),
        )
        .await
        .map_err(use_case_err)?;

    grpc_web::encode_response(&MultipleStationResponse {
        stations: stations.into_iter().map(Into::into).collect(),
    })
}

async fn handle_get_stations_by_line_group_id(mut req: Request) -> Result<Response> {
    let request: GetStationsByLineGroupIdRequest = decode_request(&mut req).await?;
    let stations = use_case()
        .get_stations_by_line_group_id(
            request.line_group_id,
            convert_transport_type(request.transport_type),
        )
        .await
        .map_err(use_case_err)?;

    grpc_web::encode_response(&MultipleStationResponse {
        stations: stations.into_iter().map(Into::into).collect(),
    })
}

async fn handle_get_stations_by_line_group_id_list(mut req: Request) -> Result<Response> {
    let request: GetStationsByLineGroupIdListRequest = decode_request(&mut req).await?;
    let stations = use_case()
        .get_stations_by_line_group_id_vec(
            &request.line_group_ids,
            convert_transport_type(request.transport_type),
        )
        .await
        .map_err(use_case_err)?;

    grpc_web::encode_response(&MultipleStationResponse {
        stations: stations.into_iter().map(Into::into).collect(),
    })
}

async fn handle_get_line_by_id(mut req: Request) -> Result<Response> {
    let request: GetLineByIdRequest = decode_request(&mut req).await?;
    let line = use_case()
        .find_line_by_id(request.line_id)
        .await
        .map_err(use_case_err)?;

    grpc_web::encode_response(&SingleLineResponse {
        line: line.map(Into::into),
    })
}

async fn handle_get_lines_by_id_list(mut req: Request) -> Result<Response> {
    let request: GetLinesByIdListRequest = decode_request(&mut req).await?;
    let lines = use_case()
        .get_lines_by_id_vec(&request.line_ids)
        .await
        .map_err(use_case_err)?;

    grpc_web::encode_response(&MultipleLineResponse {
        lines: lines.into_iter().map(Into::into).collect(),
    })
}

async fn handle_get_lines_by_name(mut req: Request) -> Result<Response> {
    let request: GetLinesByNameRequest = decode_request(&mut req).await?;
    let lines = use_case()
        .get_lines_by_name(request.line_name, request.limit)
        .await
        .map_err(use_case_err)?;

    grpc_web::encode_response(&MultipleLineResponse {
        lines: lines.into_iter().map(Into::into).collect(),
    })
}

async fn handle_get_stations_by_line_id_list(mut req: Request) -> Result<Response> {
    let request: GetStationByLineIdListRequest = decode_request(&mut req).await?;
    let stations = use_case()
        .get_stations_by_line_id_vec(
            &request.line_ids,
            convert_transport_type(request.transport_type),
        )
        .await
        .map_err(use_case_err)?;

    grpc_web::encode_response(&MultipleStationResponse {
        stations: stations.into_iter().map(Into::into).collect(),
    })
}

async fn handle_get_routes(mut req: Request) -> Result<Response> {
    let request: GetRouteRequest = decode_request(&mut req).await?;
    let routes = use_case()
        .get_routes(
            request.from_station_group_id,
            request.to_station_group_id,
            request.via_line_id,
        )
        .await
        .map_err(use_case_err)?;

    // NOTE: ページングは未対応 (next_page_token は空)
    grpc_web::encode_response(&RouteResponse {
        routes: routes.into_iter().map(Into::into).collect(),
        next_page_token: String::new(),
    })
}

async fn handle_get_route_types(mut req: Request) -> Result<Response> {
    let request: GetRouteRequest = decode_request(&mut req).await?;
    let train_types = use_case()
        .get_train_types(
            request.from_station_group_id,
            request.to_station_group_id,
            request.via_line_id,
        )
        .await
        .map_err(use_case_err)?;

    grpc_web::encode_response(&RouteTypeResponse {
        train_types: train_types.into_iter().map(Into::into).collect(),
        next_page_token: String::new(),
    })
}

async fn handle_get_connected_routes(mut req: Request) -> Result<Response> {
    let request: GetConnectedStationsRequest = decode_request(&mut req).await?;
    let routes = use_case()
        .get_connected_routes(request.from_station_group_id, request.to_station_group_id)
        .await
        .map_err(use_case_err)?;

    grpc_web::encode_response(&RouteResponse {
        routes: routes.into_iter().map(Into::into).collect(),
        next_page_token: String::new(),
    })
}

async fn handle_get_train_route(mut req: Request) -> Result<Response> {
    let request: GetTrainRouteRequest = decode_request(&mut req).await?;
    let segments = use_case()
        .get_train_route(
            request.from_station_id,
            request.to_station_id,
            request.line_group_id,
        )
        .await
        .map_err(use_case_err)?;

    grpc_web::encode_response(&TrainRouteResponse { segments })
}

async fn handle_get_routes_minimal(mut req: Request) -> Result<Response> {
    let request: GetRouteRequest = decode_request(&mut req).await?;
    // UseCase が RouteMinimalResponse をそのまま組み立てて返す
    let response = use_case()
        .get_routes_minimal(
            request.from_station_group_id,
            request.to_station_group_id,
            request.via_line_id,
        )
        .await
        .map_err(use_case_err)?;

    grpc_web::encode_response(&response)
}

async fn handle_estimate_arrival_times(mut req: Request) -> Result<Response> {
    let request: EstimateArrivalTimesRequest = decode_request(&mut req).await?;
    let stops = use_case()
        .estimate_route_arrival_times(
            request.from_station_id,
            request.to_station_id,
            &request.via_line_ids,
            request.direction_id,
        )
        .await
        .map_err(use_case_err)?;

    // presentation 層と同じ畳み込み: line_group_cd が連続する区間を 1 ルートにまとめる
    let mut routes: Vec<EstimatedArrivalRoute> = Vec::new();
    for stop in &stops {
        let proto_stop = EstimatedArrivalStop {
            station_id: stop.station_cd as u32,
            station_group_id: stop.station_g_cd as u32,
            cumulative_minutes: stop.cumulative_minutes,
            departure_cumulative_minutes: stop.departure_cumulative_minutes,
            stops_here: stop.stops_here,
        };
        let route_id = stop.line_group_cd.unwrap_or(0) as u32;
        let merge = stop.line_group_cd.is_some() && routes.last().is_some_and(|r| r.id == route_id);

        if merge {
            if let Some(last) = routes.last_mut() {
                last.stops.push(proto_stop);
            }
        } else {
            routes.push(EstimatedArrivalRoute {
                id: route_id,
                stops: vec![proto_stop],
            });
        }
    }

    grpc_web::encode_response(&EstimatedArrivalResponse {
        routes,
        next_page_token: String::new(),
    })
}

async fn handle_get_stations_by_line_id(mut req: Request) -> Result<Response> {
    let request: GetStationByLineIdRequest = decode_request(&mut req).await?;
    let stations = use_case()
        .get_stations_by_line_id(
            request.line_id,
            request.station_id,
            request.direction_id,
            convert_transport_type(request.transport_type),
        )
        .await
        .map_err(use_case_err)?;

    grpc_web::encode_response(&MultipleStationResponse {
        stations: stations.into_iter().map(Into::into).collect(),
    })
}
