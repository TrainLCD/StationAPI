fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut prost_build_config = prost_build::Config::new();
    prost_build_config.protoc_arg("--experimental_allow_proto3_optional");

    tonic_build::configure()
        .build_client(false)
        .type_attribute("LineSymbol", "#[derive(fake::Dummy)]")
        .type_attribute("StationNumber", "#[derive(fake::Dummy)]")
        .type_attribute("Company", "#[derive(fake::Dummy)]")
        .type_attribute("Line", "#[derive(fake::Dummy)]")
        .type_attribute("Station", "#[derive(fake::Dummy)]")
        .compile_with_config(prost_build_config, &["proto/stationapi.proto"], &["proto"])
        .unwrap();

    Ok(())
}
