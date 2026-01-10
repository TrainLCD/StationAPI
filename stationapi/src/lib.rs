pub mod config;
pub mod domain;
pub mod infrastructure;
pub mod presentation;
pub mod stop_pattern;
pub mod use_case;

pub mod proto {
    tonic::include_proto!("app.trainlcd.grpc");

    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("stationapi_descriptor");
}
