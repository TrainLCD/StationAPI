fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .type_attribute("Company", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute(
            "LineSymbol",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .type_attribute(
            "TrainType",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .type_attribute(
            "StationNumber",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .type_attribute("Line", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute("Station", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute(
            "StopCondition",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile(&["proto/stationapi.proto"], &["proto"])?;
    Ok(())
}
