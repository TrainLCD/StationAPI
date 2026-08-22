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
    GetStationByCoordinatesRequest, GetStationsByNameRequest, GetTrainTypesByStationIdRequest,
    MultipleStationResponse, MultipleTrainTypeResponse, TransportType as GrpcTransportType,
};
use stationapi::use_case::interactor::query::QueryInteractor;
use stationapi::use_case::traits::query::QueryUseCase;

use repository::{
    MemCompanyRepository, MemLineRepository, MemStationRepository, MemTrainTypeRepository,
};

const COORDINATES_PATH: &str = "/app.trainlcd.grpc.StationAPI/GetStationsByCoordinates";
const BY_NAME_PATH: &str = "/app.trainlcd.grpc.StationAPI/GetStationsByName";
const TRAIN_TYPES_PATH: &str = "/app.trainlcd.grpc.StationAPI/GetTrainTypesByStationId";

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
