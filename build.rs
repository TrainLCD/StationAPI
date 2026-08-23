//! station_station_types.csv を固定長バイナリへ事前変換する。
//!
//! この CSV は 41,250 行あり、isolate 起動時の CSV パースがコールドスタートの
//! 大半を占める。全列が整数なので 1 行 = i32 x 4 の固定長にしておけば、
//! ランタイムではスライスを読むだけで済む。

use std::{env, fs, path::Path, path::PathBuf};

/// CSV に NULL 表現が無いため、欠損値を i32::MIN で表す
const NULL_I32: i32 = i32::MIN;

/// Workers が読む CSV を OUT_DIR へ集める。
///
/// generated/ があればそれを使う。無ければ data/*.csv にフォールバックする。
/// generated は preprocessor が組み立てたもので、data/*.csv には無い
/// 各駅停車の生成系統や、GTFS 由来のバス停・バス路線を含む。
fn stage_csv(out_dir: &Path, name: &str, fallback: &str) -> bool {
    let generated = Path::new("generated").join(name);
    println!("cargo:rerun-if-changed=generated/{name}");
    println!("cargo:rerun-if-changed={fallback}");

    let (src, from_generated) = if generated.is_file() {
        (generated, true)
    } else {
        (PathBuf::from(fallback), false)
    };
    fs::copy(&src, out_dir.join(name))
        .unwrap_or_else(|e| panic!("{} を配置できない: {e}", src.display()));
    from_generated
}

fn main() {
    // 本番と同じデータを使うには preprocessor の出力が要る。preprocessor は
    // 列車種別を持たない路線へ各駅停車の系統を補う (約2,400行)。この行は
    // data/*.csv に存在しないため、CSV を直接読むと 2,268 駅の
    // has_train_types が false になり本番と挙動がずれる。
    //
    // CI では `cargo run --profile tool -p stationapi-preprocessor` で generated/ を
    // 作り、それをここで優先して読む。
    let generated = Path::new("generated/station_station_types.csv");
    let fallback = Path::new("data/5!station_station_types.csv");
    let (csv_path, has_generated) = if generated.is_file() {
        (generated, true)
    } else {
        println!(
            "cargo:warning=generated/station_station_types.csv が無いため \
             data/5!station_station_types.csv を使用します。生成される各駅停車の系統が \
             含まれないため、本番と挙動が異なります"
        );
        (fallback, false)
    };
    let csv_path = csv_path.to_string_lossy().to_string();
    println!("cargo:rerun-if-changed={csv_path}");
    println!("cargo:rerun-if-changed=generated/station_station_types.csv");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    let staged = [
        stage_csv(&out_dir, "stations.csv", "data/3!stations.csv"),
        stage_csv(&out_dir, "lines.csv", "data/2!lines.csv"),
        stage_csv(&out_dir, "companies.csv", "data/1!companies.csv"),
        stage_csv(&out_dir, "types.csv", "data/4!types.csv"),
        stage_csv(&out_dir, "aliases.csv", "data/6!aliases.csv"),
        stage_csv(&out_dir, "line_aliases.csv", "data/7!line_aliases.csv"),
        // station_station_types は下の sst 変換でも参照するが、
        // 混在判定に含めるためここでも存在を見る
        Path::new("generated/station_station_types.csv").is_file(),
    ];
    // 混在を許すと、例えば generated の station_station_types だけが入り
    // stations が data/*.csv のままになる。生成された系統の station_cd が
    // 駅索引に存在せず、列車種別が欠落した Worker が出来上がってしまう。
    // cargo:warning は CI で見落とされるため、混在はビルドを止める。
    let generated_count = staged.iter().filter(|v| **v).count();
    if generated_count != 0 && generated_count != staged.len() {
        panic!(
            "generated が {} / {} ファイルしかありません。\n\
             生成物と data/*.csv が混ざるとデータが不整合になります。\n\
             `cargo run --profile tool -p stationapi-preprocessor` で全テーブルを書き出すか、\n\
             generated を削除して data/*.csv だけを使ってください。",
            generated_count,
            staged.len()
        );
    }
    if generated_count == 0 {
        println!(
            "cargo:warning=generated が無いため data/*.csv を使用します。\
             生成される各駅停車の系統や GTFS 由来のバスデータが含まれないため、\
             本番と挙動が異なります"
        );
    }
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(&csv_path)
        .expect("station_station_types.csv を開けない");

    let headers = reader.headers().expect("ヘッダを読めない").clone();
    let col = |name: &str| headers.iter().position(|h| h.trim() == name);
    let (Some(i_station), Some(i_type)) = (col("station_cd"), col("type_cd")) else {
        panic!("station_cd / type_cd 列が見つからない");
    };
    let i_group = col("line_group_cd");
    let i_pass = col("pass");
    // 生成物は SERIAL 採番後の実 id を持つ。CSV は "DEFAULT" なので行順で採番する。
    let i_id = if has_generated { col("id") } else { None };

    let num = |r: &csv::StringRecord, i: Option<usize>| -> i32 {
        i.and_then(|i| r.get(i))
            .and_then(|v| v.trim().parse::<i32>().ok())
            .unwrap_or(NULL_I32)
    };

    let mut out: Vec<u8> = Vec::with_capacity(45_000 * 16);
    let mut expected_id = 0i32;
    for record in reader.records().flatten() {
        let station_cd = num(&record, Some(i_station));
        let type_cd = num(&record, Some(i_type));
        // ランタイム側の CSV パースと同じく、必須列が壊れている行は落とす
        if station_cd == NULL_I32 || type_cd == NULL_I32 {
            continue;
        }
        // ランタイムは行順で id を振り直すため、生成物の id が連番でなければ
        // 停車順序がずれる。ずれていたらビルドを止める。
        expected_id += 1;
        if let Some(i) = i_id {
            let actual = num(&record, Some(i));
            assert_eq!(
                actual, expected_id,
                "station_station_types.id が連番ではない (期待 {expected_id}, 実際 {actual})"
            );
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
