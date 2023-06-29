fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut prost_build_config = prost_build::Config::new();
    prost_build_config.protoc_arg("--experimental_allow_proto3_optional");

    tonic_build::configure()
        .type_attribute("LineSymbol", "#[derive(fake::Dummy)]")
        .type_attribute("StationNumber", "#[derive(fake::Dummy)]")
        .type_attribute("CompanyResponse", "#[derive(fake::Dummy)]")
        .type_attribute("LineResponse", "#[derive(fake::Dummy)]")
        .type_attribute("StationResponse", "#[derive(fake::Dummy)]")
        .compile_with_config(prost_build_config, &["proto/stationapi.proto"], &["proto"])
        .unwrap();

    Ok(())
}
