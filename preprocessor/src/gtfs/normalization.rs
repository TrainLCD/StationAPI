//! 配布元が修正するまで、確認済みのフィード固有の形式不備だけを補正する。
use anyhow::{ensure, Context, Result};
use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Normalization {
    #[serde(rename = "daito_2026")]
    Daito2026,
}

impl Normalization {
    pub(super) fn prepare(self, directory: &Path) -> Result<()> {
        match self {
            Self::Daito2026 => normalize_daito(directory),
        }
    }
}

fn normalize_daito(directory: &Path) -> Result<()> {
    let path = directory.join("trips.txt");
    let bytes = fs::read(&path)?;
    let text = match std::str::from_utf8(&bytes) {
        Ok(text) => std::borrow::Cow::Borrowed(text),
        Err(_) => encoding_rs::SHIFT_JIS
            .decode_without_bom_handling_and_without_replacement(&bytes)
            .context("大東市 trips.txt を Shift_JIS として変換できない")?,
    };
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(text.as_bytes());
    let headers = reader.headers()?.clone();
    ensure!(
        headers.iter().eq([
            "route_id",
            "service_id",
            "trip_id",
            "trip_headsign",
            "direction_id",
            "block_id",
            "shape_id"
        ]),
        "大東市 trips.txt のヘッダーが変更された"
    );
    let mut trips = csv::Writer::from_writer(Vec::new());
    trips.write_record(&headers)?;
    for record in reader.records() {
        let mut record = record?;
        ensure!(
            record.len() == 5 || record.len() == 7,
            "大東市 trips.txt の列数が想定外"
        );
        while record.len() < 7 {
            record.push_field("");
        }
        trips.write_record(&record)?;
    }
    let trips = trips.into_inner()?;

    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_path(directory.join("stops.txt"))?;
    let headers = reader.headers()?.clone();
    ensure!(
        headers.iter().eq([
            "stop_id",
            "stop_name",
            "platform_code",
            "stop_lat",
            "stop_lon",
            "zone_id",
            "location_type",
            "parent_station"
        ]),
        "大東市 stops.txt のヘッダーが変更された"
    );
    let mut stops = csv::Writer::from_writer(Vec::new());
    stops.write_record(&headers)?;
    for record in reader.records() {
        let record = record?;
        if record.len() == 8 {
            stops.write_record(&record)?;
        } else {
            // 原本の3行だけが小数点をカンマで記載しており、座標が4列に分裂している。
            ensure!(
                record.len() == 10
                    && matches!(record.get(0), Some("S53" | "S53_1" | "S53_2"))
                    && record.get(1) == Some("総合文化センター")
                    && record.get(3) == Some(" 34")
                    && record.get(4) == Some("704536")
                    && record.get(5) == Some("135")
                    && record.get(6) == Some("6238244"),
                "大東市 stops.txt に未確認の不正な行がある"
            );
            stops.write_record([
                &record[0],
                &record[1],
                &record[2],
                "34.704536",
                "135.6238244",
                &record[7],
                &record[8],
                &record[9],
            ])?;
        }
    }
    let stops = stops.into_inner()?;
    // 両ファイルの検証が終わるまで原本を変更しない。
    // 書き込み失敗でキャッシュを切り詰めないよう、両方を準備してから置き換える。
    // rename 間で失敗しても各ファイルは完全で、正規化は再実行できる。
    let stops_path = directory.join("stops.txt");
    let trips_tmp = path.with_extension("txt.tmp");
    let stops_tmp = stops_path.with_extension("txt.tmp");
    fs::write(&trips_tmp, trips)?;
    fs::write(&stops_tmp, stops)?;
    fs::rename(&trips_tmp, path)?;
    fs::rename(&stops_tmp, stops_path)?;
    Ok(())
}
