pub mod domain;
pub mod infrastructure;
pub mod presentation;
pub mod use_case;

pub mod station_api {
    tonic::include_proto!("app.trainlcd.grpc");
}
