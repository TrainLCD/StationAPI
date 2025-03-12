use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    tonic_build::configure()
        .build_client(false)
        .build_server(true)
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
        .file_descriptor_set_path(out_dir.join("stationapi_descriptor.bin"))
        .compile_protos(&["proto/stationapi.proto"], &["proto"])?;
    Ok(())
}
