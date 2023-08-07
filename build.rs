fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut prost_build_config = prost_build::Config::new();
    prost_build_config.protoc_arg("--experimental_allow_proto3_optional");
    tonic_build::configure().compile_with_config(
        prost_build_config,
        &["proto/stationapi.proto"],
        &["proto"],
    )?;
    Ok(())
}
