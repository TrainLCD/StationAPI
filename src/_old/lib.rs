pub mod application;
pub mod domain;
pub mod infra;

pub mod pb {
    tonic::include_proto!("app.trainlcd.grpc");
}
