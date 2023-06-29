fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .type_attribute("LineSymbol", "#[derive(fake::Dummy)]")
        .type_attribute("StationNumber", "#[derive(fake::Dummy)]")
        .compile(&["proto/stationapi.proto"], &["proto"])
        .unwrap();

    Ok(())
}
