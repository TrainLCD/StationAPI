use stationapi::infra::grpc::router::run;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    run().await
}
