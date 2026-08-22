//! gRPC-Web のフレーミング。
//! tonic-web のサーバー実装は wasm32 で使えないため、必要な部分だけを自前で扱う。
//!
//! フレーム形式: [flags:1][length:4 BE][payload]
//!   flags 0x00 = データ, 0x80 = トレーラー

use prost::Message;
use worker::*;

const CONTENT_TYPE: &str = "application/grpc-web+proto";

/// リクエストのデータフレームから protobuf ペイロードを取り出す。
pub fn decode_frame(body: &[u8]) -> Result<&[u8]> {
    if body.len() < 5 {
        return Err(Error::RustError("gRPC-Web frame too short".into()));
    }
    if body[0] & 0x80 != 0 {
        return Err(Error::RustError("unexpected trailer frame in request".into()));
    }
    let len = u32::from_be_bytes([body[1], body[2], body[3], body[4]]) as usize;
    body.get(5..5 + len)
        .ok_or_else(|| Error::RustError("gRPC-Web frame length mismatch".into()))
}

/// protobuf メッセージをデータフレーム + トレーラーフレームとして返す。
pub fn encode_response<M: Message>(message: &M) -> Result<Response> {
    let mut payload = Vec::with_capacity(message.encoded_len());
    message
        .encode(&mut payload)
        .map_err(|e| Error::RustError(format!("protobuf encode failed: {e}")))?;

    const TRAILER: &[u8] = b"grpc-status:0\r\n";
    let mut out = Vec::with_capacity(payload.len() + 10 + TRAILER.len());
    out.push(0x00);
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(&payload);
    out.push(0x80);
    out.extend_from_slice(&(TRAILER.len() as u32).to_be_bytes());
    out.extend_from_slice(TRAILER);

    Ok(Response::from_bytes(out)?.with_headers(response_headers()?))
}

fn response_headers() -> Result<Headers> {
    let headers = Headers::new();
    headers.set("content-type", CONTENT_TYPE)?;
    headers.set("grpc-status", "0")?;
    headers.set("access-control-allow-origin", "*")?;
    headers.set("access-control-expose-headers", "grpc-status,grpc-message")?;
    Ok(headers)
}

/// ブラウザの gRPC-Web クライアントは preflight を投げる。
pub fn preflight() -> Result<Response> {
    let headers = Headers::new();
    headers.set("access-control-allow-origin", "*")?;
    headers.set("access-control-allow-methods", "POST,OPTIONS")?;
    headers.set(
        "access-control-allow-headers",
        "content-type,x-grpc-web,x-user-agent,grpc-timeout",
    )?;
    headers.set("access-control-max-age", "86400")?;
    Ok(Response::empty()?.with_headers(headers))
}
