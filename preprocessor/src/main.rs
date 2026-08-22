//! Worker が読む `generated/*.csv` を組み立てる。
//!
//! `data/*.csv` をそのまま Worker へ渡すと本番と挙動が変わる。列車種別を持たない
//! 路線には各駅停車の系統を補う必要があり (約2,400行)、バス停・バス路線は GTFS と
//! ODPT の JSON から起こす必要があるため。
//!
//! ```text
//! data/*.csv ─┐
//! GTFS zip   ─┼─> preprocessor ─> generated/*.csv ─> worker-build ─> WASM
//! ODPT JSON  ─┘
//! ```
//!
//! 使い方:
//!
//! ```text
//! preprocessor [出力先]            # 既定は generated
//! DISABLE_BUS_FEATURE=true preprocessor   # 鉄道のみ
//! ```
//!
//! バスの一部フィードは `ODPT_ACCESS_TOKEN` を要求する。設定が無ければ
//! トークン不要なフィード (都営バス) と、7 日以内のキャッシュだけが使われる。

mod codes;
mod emit;
mod gtfs;
mod rail;
mod table;

use std::path::{Path, PathBuf};

use anyhow::Result;

/// 進捗の出力。ビルド用の CLI なので標準エラーへそのまま書く。
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => { eprintln!("[preprocessor] {}", format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => { eprintln!("[preprocessor] 警告: {}", format_args!($($arg)*)) };
}

/// バスを外すかどうか。取り込みに時間がかかるため、鉄道だけ試したいときに使う。
fn bus_feature_disabled() -> bool {
    match std::env::var("DISABLE_BUS_FEATURE") {
        Ok(value) => value.eq_ignore_ascii_case("true") || value == "1",
        Err(_) => false,
    }
}

fn main() -> Result<()> {
    let out_dir: PathBuf = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "generated".to_string())
        .into();
    let data_dir = Path::new("data");

    let mut dataset = rail::Dataset::load(data_dir)?;
    dataset.generate_virtual_local_rail_services()?;

    if bus_feature_disabled() {
        info!("DISABLE_BUS_FEATURE が立っているのでバスを取り込まない");
    } else {
        let gtfs = gtfs::load()?;
        gtfs::integrate::integrate(&mut dataset, &gtfs);
    }

    emit::write_all(&mut dataset, &out_dir)?;
    info!("完了: {}", out_dir.display());
    Ok(())
}
