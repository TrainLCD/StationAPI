pub mod domain;
pub mod use_case;

// gRPC サーバー一式。wasm32 ビルドでは除外される。
#[cfg(feature = "server")]
pub mod config;
#[cfg(feature = "server")]
pub mod infrastructure;
#[cfg(feature = "server")]
pub mod presentation;

pub mod proto {
    // server 有効時は tonic のサービストレイトを含む生成コードを取り込む。
    // 無効時 (wasm) は同じ生成ファイルを prost のメッセージ型としてのみ取り込む。
    #[cfg(feature = "server")]
    tonic::include_proto!("app.trainlcd.grpc");
    #[cfg(not(feature = "server"))]
    include!(concat!(env!("OUT_DIR"), "/app.trainlcd.grpc.rs"));

    #[cfg(feature = "server")]
    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("stationapi_descriptor");
}
