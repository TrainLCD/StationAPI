pub mod dao;
pub mod entities;
pub mod repositories;
pub mod usecases;

pub mod service {
    tonic::include_proto!("app.trainlcd.grpc");
}
