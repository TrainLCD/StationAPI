//! StationAPI を Cloudflare Workers 上で動かす。
//!
//! かつてはオンプレのサーバーが gRPC-Web を返し、BFF が GraphQL へ変換していた。
//! こちらは GraphQL をそのまま返すので BFF を挟まない。
//!
//! 公開スキーマは schema/public.graphql が正で、CI が SDL を
//! 突き合わせる。エンドポイントはクライアント互換のためサブドメイン直下
//! (`/`) でクエリを受ける。`/__schema` などの内部向けは `__` を前置して区別する。
//!
//! データベースは持たず、ビルド時に埋め込んだ CSV をインメモリ索引にする。
//! UseCase 層は共有のまま、repository トレイトの実装だけを差し替えている。

mod graphql;
mod index;
mod repository;

use async_graphql::http::GraphiQLSource;
use async_graphql::Request as GqlRequest;
use worker::*;

use stationapi::use_case::interactor::query::QueryInteractor;

use repository::{
    MemCompanyRepository, MemLineRepository, MemStationRepository, MemTrainTypeRepository,
};

pub type Interactor = QueryInteractor<
    MemStationRepository,
    MemLineRepository,
    MemTrainTypeRepository,
    MemCompanyRepository,
>;

/// repository は状態を持たない (索引は OnceLock 側にある) ので毎回生成して問題ない。
fn interactor() -> Interactor {
    QueryInteractor {
        station_repository: MemStationRepository,
        line_repository: MemLineRepository,
        train_type_repository: MemTrainTypeRepository,
        company_repository: MemCompanyRepository,
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
        return preflight();
    }
    // 疎通確認用。データに一切触らないので、/__health との差を見れば
    // データ初期化のコストを外から切り分けられる。
    if method == Method::Get && path == "/__ping" {
        return Response::ok("pong");
    }
    if method == Method::Get && path == "/__health" {
        return Response::ok(format!(
            "stations={} lines={} companies={}",
            index::stations().len(),
            index::lines().len(),
            index::companies().len()
        ));
    }
    // スキーマを配る。CI はこれと schema/public.graphql を突き合わせる。
    if method == Method::Get && path == "/__schema" {
        let schema = graphql::build_schema(interactor());
        return with_cors(Response::ok(schema.sdl())?);
    }
    // クライアント互換のため、サブドメイン直下で GraphQL を受ける
    if method == Method::Get && path == "/" {
        return Response::from_html(GraphiQLSource::build().endpoint("/").finish());
    }
    if method == Method::Post && path == "/" {
        return handle_graphql(req).await;
    }

    Response::error("Not Found", 404)
}

async fn handle_graphql(mut req: Request) -> Result<Response> {
    let body = req.text().await?;
    let request: GqlRequest = serde_json::from_str(&body)
        .map_err(|e| Error::RustError(format!("GraphQL リクエストを解釈できません: {e}")))?;

    let schema = graphql::build_schema(interactor());
    let response = schema.execute(request).await;

    let payload = serde_json::to_string(&response)
        .map_err(|e| Error::RustError(format!("GraphQL レスポンスを作れません: {e}")))?;

    let headers = Headers::new();
    headers.set("content-type", "application/json")?;
    headers.set("access-control-allow-origin", "*")?;
    Ok(Response::ok(payload)?.with_headers(headers))
}

fn preflight() -> Result<Response> {
    let headers = Headers::new();
    headers.set("access-control-allow-origin", "*")?;
    headers.set("access-control-allow-methods", "POST,GET,OPTIONS")?;
    headers.set("access-control-allow-headers", "content-type")?;
    headers.set("access-control-max-age", "86400")?;
    Ok(Response::empty()?.with_headers(headers))
}

fn with_cors(response: Response) -> Result<Response> {
    let headers = Headers::new();
    headers.set("access-control-allow-origin", "*")?;
    headers.set("content-type", "text/plain; charset=utf-8")?;
    Ok(response.with_headers(headers))
}
