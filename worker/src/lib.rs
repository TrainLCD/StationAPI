//! StationAPI を Cloudflare Workers 上で動かす PoC。
//! sqlx / tonic を使わず、埋め込み CSV のインメモリ索引と自前の gRPC-Web フレーミングで
//! 既存クライアント互換の応答を返す。

mod grpc_web;
mod index;

use prost::Message;
use worker::*;

use stationapi::proto::{
    GetStationByCoordinatesRequest, GetStationsByNameRequest, MultipleStationResponse,
    Station as GrpcStation,
};

const COORDINATES_PATH: &str = "/app.trainlcd.grpc.StationAPI/GetStationsByCoordinates";
const BY_NAME_PATH: &str = "/app.trainlcd.grpc.StationAPI/GetStationsByName";

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
        return Response::ok(format!("stations={}", index::stations().len()));
    }
    if method == Method::Post && path == COORDINATES_PATH {
        return handle_get_stations_by_coordinates(req).await;
    }
    if method == Method::Post && path == BY_NAME_PATH {
        return handle_get_stations_by_name(req).await;
    }

    Response::error("Not Found", 404)
}

async fn handle_get_stations_by_coordinates(mut req: Request) -> Result<Response> {
    let body = req.bytes().await?;
    let payload = grpc_web::decode_frame(&body)?;
    let request = GetStationByCoordinatesRequest::decode(payload)
        .map_err(|e| Error::RustError(format!("protobuf decode failed: {e}")))?;

    // 既存 SQL の LIMIT $3 と同じく未指定なら 1 件
    let limit = request.limit.unwrap_or(1).clamp(1, 100) as usize;
    let found = index::nearest(request.latitude, request.longitude, limit);

    let stations = found
        .into_iter()
        .map(|(record, distance_km)| to_proto_station(record, Some(distance_km)))
        .collect();

    grpc_web::encode_response(&MultipleStationResponse { stations })
}

async fn handle_get_stations_by_name(mut req: Request) -> Result<Response> {
    let body = req.bytes().await?;
    let payload = grpc_web::decode_frame(&body)?;
    let request = GetStationsByNameRequest::decode(payload)
        .map_err(|e| Error::RustError(format!("protobuf decode failed: {e}")))?;

    // NOTE: 既存実装は limit 未指定で LIMIT NULL (全件) になる。PoC では上限を設ける。
    // NOTE: from_station_group_id による乗り換え可否フィルタは PoC では未対応。
    let limit = request.limit.unwrap_or(50).clamp(1, 200) as usize;
    let found = index::search_by_name(&request.station_name, limit);

    let stations = found
        .into_iter()
        .map(|record| to_proto_station(record, None))
        .collect();

    grpc_web::encode_response(&MultipleStationResponse { stations })
}

/// PoC 段階では路線・列車種別は付与しない (lines / line / train_type は空)。
/// 本実装では QueryInteractor 経由で属性を付与する。
fn to_proto_station(record: &index::StationRecord, distance_km: Option<f64>) -> GrpcStation {
    GrpcStation {
        id: record.station_cd,
        group_id: record.station_g_cd,
        name: record.name.clone(),
        name_katakana: record.name_katakana.clone(),
        name_roman: record.name_roman.clone(),
        name_chinese: record.name_chinese.clone(),
        name_korean: record.name_korean.clone(),
        three_letter_code: record.three_letter_code.clone(),
        prefecture_id: record.pref_cd,
        postal_code: record.postal_code.clone(),
        address: record.address.clone(),
        latitude: record.lat,
        longitude: record.lon,
        opened_at: record.opened_at.clone(),
        closed_at: record.closed_at.clone(),
        status: record.e_status,
        distance: distance_km.map(|km| km * 1000.0),
        ..Default::default()
    }
}
