//! station_station_types.csv を固定長バイナリへ事前変換する。
//!
//! この CSV は 41,250 行あり、isolate 起動時の CSV パースがコールドスタートの
//! 大半を占める。全列が整数なので 1 行 = i32 x 4 の固定長にしておけば、
//! ランタイムではスライスを読むだけで済む。

use std::{env, fs, path::PathBuf};

/// CSV に NULL 表現が無いため、欠損値を i32::MIN で表す
const NULL_I32: i32 = i32::MIN;

fn main() {
    let csv_path = "../data/5!station_station_types.csv";
    println!("cargo:rerun-if-changed={csv_path}");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(csv_path)
        .expect("station_station_types.csv を開けない");

    let headers = reader.headers().expect("ヘッダを読めない").clone();
    let col = |name: &str| headers.iter().position(|h| h.trim() == name);
    let (Some(i_station), Some(i_type)) = (col("station_cd"), col("type_cd")) else {
        panic!("station_cd / type_cd 列が見つからない");
    };
    let i_group = col("line_group_cd");
    let i_pass = col("pass");

    let num = |r: &csv::StringRecord, i: Option<usize>| -> i32 {
        i.and_then(|i| r.get(i))
            .and_then(|v| v.trim().parse::<i32>().ok())
            .unwrap_or(NULL_I32)
    };

    let mut out: Vec<u8> = Vec::with_capacity(45_000 * 16);
    for record in reader.records().flatten() {
        let station_cd = num(&record, Some(i_station));
        let type_cd = num(&record, Some(i_type));
        // ランタイム側の CSV パースと同じく、必須列が壊れている行は落とす
        if station_cd == NULL_I32 || type_cd == NULL_I32 {
            continue;
        }
        for value in [
            station_cd,
            type_cd,
            num(&record, i_group),
            num(&record, i_pass),
        ] {
            out.extend_from_slice(&value.to_le_bytes());
        }
    }

    fs::write(out_dir.join("sst.bin"), &out).expect("sst.bin を書けない");
    println!("cargo:warning=sst.bin: {} 行", out.len() / 16);
}
