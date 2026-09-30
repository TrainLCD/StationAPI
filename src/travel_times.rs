//! 実際の所要時間 (`travel_times/cases.csv`) に対する `trainRoute` の `Estimated`
//! (MobileApp の GPX の生成が使う推定) の回帰の見張り。`estimateArrivalTimes` と
//! `connectedRoutes` は元の較正のままなので、ここでは測らない。
//!
//! 基準ごとに推定の所要時間を出し、実際の典型的な所要時間 (平日日中の中央値) から
//! のずれを求める。記録した推定 (`travel_times/baseline.csv`) より悪くなった基準が
//! あるか、平均が悪くなったら失敗にする。速度の較正や一般則を 1 つの路線に合わせて
//! 変えたときに、ほかの路線がどれだけ崩れたかをここで見る。
//!
//! 実際の所要時間の範囲 (最小〜最大) からの外れも表に出すが、判定には使わない。
//! 待ち合わせなどで範囲に外れ値の列車が入ると幅が広がり、範囲内なら誤差 0 と
//! 数える物差しでは、典型的な値から大きく離れても見逃すため。
//!
//! 記録は、本番と同じ生成データ (`make data` で作る `generated/`) で出した推定で、
//! 比べるのも生成データのときだけにする。到着時間推定は、生成データにしか無い
//! 線路の長さや種別グループを使うので、`data/*.csv` だけでは本番の推定を再現
//! できない。CI では `build_worker.yml` が `generated/` を作ってからこれを走らせる。
//! `data/*.csv` で動くとき (`ci.yml` のテストなど) は、表を出すだけにする。
//!
//! 推定を意図して変えたときは、`make data` のあとに次で記録を更新する。
//! `TRAVEL_TIMES_UPDATE_BASELINE=1 cargo test -p stationapi-worker travel_times`

use std::collections::HashMap;

use stationapi::domain::repository::station_repository::StationRepository;
use stationapi::model::{RouteLegRequest, TrainRouteModel};
use stationapi::use_case::traits::query::QueryUseCase;

use crate::repository::MemStationRepository;

const CASES: &str = include_str!("../travel_times/cases.csv");
const BASELINE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/travel_times/baseline.csv");
/// 1 基準あたり、典型的な値からのずれがこれだけ増えたら失敗にする (割合)
const CASE_TOLERANCE: f64 = 0.01;
/// 平均の比較の許容幅 (割合)。記録は推定の分を小数 4 桁に丸めて書くので、その丸めで
/// 平均がわずかに動くぶんを吸収する
const MEAN_TOLERANCE: f64 = 1e-4;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Measure {
    /// 出発駅の発車から到着駅の到着まで
    Arrival,
    /// 出発駅の発車から到着駅の発車まで (到着時刻を載せない時刻表向け)
    Departure,
}

#[derive(Debug)]
struct Case {
    label: String,
    line_group_id: u32,
    from_station_id: u32,
    to_station_id: u32,
    /// 推定する区間の終わり。`Departure` では到着駅の先まで推定しないと
    /// 到着駅が終点になり、停車時間が付かない
    slice_end_station_id: u32,
    measure: Measure,
    real_min: f64,
    real_max: f64,
    /// 典型的な所要時間 (平日日中の中央値。分からないときは範囲の中央)
    real_typical: f64,
}

impl Case {
    /// 典型的な所要時間からのずれ (絶対値の割合)。判定に使う指標
    fn typical_error(&self, estimated: f64) -> f64 {
        (estimated - self.real_typical).abs() / self.real_typical
    }

    /// 実際の範囲からの外れ。範囲内なら 0、外れたら近い端に対する割合
    fn range_error(&self, estimated: f64) -> f64 {
        if estimated < self.real_min {
            (self.real_min - estimated) / self.real_min
        } else if estimated > self.real_max {
            (estimated - self.real_max) / self.real_max
        } else {
            0.0
        }
    }
}

fn parse_cases() -> Vec<Case> {
    let mut lines = CASES.lines();
    let header: Vec<&str> = lines.next().expect("見出しの行が無い").split(',').collect();
    let col = |name: &str| {
        header
            .iter()
            .position(|h| *h == name)
            .unwrap_or_else(|| panic!("列 {name} が無い"))
    };
    let (label, group, from, to, end, measure, min, max, typical) = (
        col("label"),
        col("line_group_id"),
        col("from_station_id"),
        col("to_station_id"),
        col("slice_end_station_id"),
        col("measure"),
        col("real_min_minutes"),
        col("real_max_minutes"),
        col("real_typical_minutes"),
    );
    lines
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let f: Vec<&str> = line.split(',').collect();
            assert_eq!(f.len(), header.len(), "列の数が見出しと違う: {line}");
            let num = |i: usize| -> u32 {
                f[i].parse()
                    .unwrap_or_else(|_| panic!("数値ではない: {line}"))
            };
            let minutes = |i: usize| -> f64 {
                f[i].parse()
                    .unwrap_or_else(|_| panic!("数値ではない: {line}"))
            };
            let measure = match f[measure] {
                "arrival" => Measure::Arrival,
                "departure" => Measure::Departure,
                other => panic!("measure は arrival / departure のどちらか: {other}"),
            };
            let to_station_id = num(to);
            Case {
                label: f[label].to_string(),
                line_group_id: num(group),
                from_station_id: num(from),
                to_station_id,
                slice_end_station_id: if f[end].is_empty() {
                    to_station_id
                } else {
                    num(end)
                },
                measure,
                real_min: minutes(min),
                real_max: minutes(max),
                real_typical: minutes(typical),
            }
        })
        .collect()
}

/// repository の実装は await しない (索引を引くだけ) ので、1 回 poll すれば終わる
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    match std::pin::pin!(future).poll(&mut context) {
        std::task::Poll::Ready(value) => value,
        std::task::Poll::Pending => panic!("repository futures complete without waiting"),
    }
}

/// 推定の所要時間 (分)。種別グループがこのデータに無ければ `None`
fn estimate(case: &Case) -> Option<f64> {
    let group = block_on(MemStationRepository.get_by_line_group_id(case.line_group_id)).ok()?;
    if group.is_empty() {
        return None;
    }
    let legs = [RouteLegRequest {
        line_group_id: case.line_group_id,
        from_station_id: case.from_station_id,
        to_station_id: case.slice_end_station_id,
    }];
    // 見張るのは trainRoute の Estimated (GPX の生成が使う推定)。estimateArrivalTimes と
    // connectedRoutes は元の較正のままで、推定の規則や較正を変えても動かない
    let segments =
        block_on(crate::interactor().get_connected_train_route(&legs, TrainRouteModel::Estimated))
            .unwrap_or_else(|e| panic!("{}: 推定できない: {e}", case.label));
    let segment = segments
        .iter()
        .skip(1)
        .find(|segment| {
            segment.station.as_ref().map(|station| station.id) == Some(case.to_station_id)
        })
        .unwrap_or_else(|| panic!("{}: 推定の駅列に到着駅が無い", case.label));
    match case.measure {
        Measure::Arrival => segment.arrival_cumulative_minutes,
        Measure::Departure => segment.departure_cumulative_minutes,
    }
}

fn read_baseline() -> HashMap<String, f64> {
    let text = std::fs::read_to_string(BASELINE_PATH).unwrap_or_default();
    text.lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let (label, minutes) = line.rsplit_once(',').expect("label,estimated_minutes の形");
            (
                label.to_string(),
                minutes.parse().expect("推定の分が数値ではない"),
            )
        })
        .collect()
}

#[test]
fn cases_are_well_formed() {
    let cases = parse_cases();
    assert!(!cases.is_empty());
    let mut labels = std::collections::HashSet::new();
    for case in &cases {
        assert!(labels.insert(&case.label), "label が重複: {}", case.label);
        assert!(
            case.real_min > 0.0
                && case.real_min <= case.real_typical
                && case.real_typical <= case.real_max,
            "{}",
            case.label
        );
        if case.measure == Measure::Departure {
            assert_ne!(
                case.slice_end_station_id, case.to_station_id,
                "{}: departure は到着駅の先まで推定する (slice_end_station_id)",
                case.label
            );
        }
    }
}

#[test]
fn estimates_do_not_drift_away_from_real_travel_times() {
    let cases = parse_cases();
    let estimated: Vec<(&Case, Option<f64>)> = cases.iter().map(|c| (c, estimate(c))).collect();

    let mut report = vec![String::from(
        "\n基準 | 実際 (典型) | 推定 | 典型からのずれ | 範囲からの外れ",
    )];
    for (case, est) in &estimated {
        let real = format!(
            "{}〜{}分 ({}分)",
            case.real_min, case.real_max, case.real_typical
        );
        report.push(match est {
            Some(est) => format!(
                "{} | {real} | {est:.1}分 | {:.1}% | {:.1}%",
                case.label,
                case.typical_error(*est) * 100.0,
                case.range_error(*est) * 100.0
            ),
            None => format!(
                "{} | {real} | (このデータに種別グループが無い) | - | -",
                case.label
            ),
        });
    }
    println!("{}", report.join("\n"));

    // 記録は本番と同じ生成データで作る。data/*.csv では線路の長さや生成された
    // 種別グループが無く、推定が本番と違うので、記録とは比べない
    let embedded_generated = env!("STATIONAPI_EMBEDDED_DATA") == "generated";

    if std::env::var("TRAVEL_TIMES_UPDATE_BASELINE").as_deref() == Ok("1") {
        assert!(
            embedded_generated,
            "記録は生成データで作る。make data で generated/ を作ってから更新する"
        );
        let mut out = String::from("label,estimated_minutes\n");
        for (case, est) in &estimated {
            if let Some(est) = est {
                out.push_str(&format!("{},{est:.4}\n", case.label));
            }
        }
        std::fs::write(BASELINE_PATH, out).expect("記録を書き込めない");
        return;
    }

    if !embedded_generated {
        println!(
            "data/*.csv で動いているので記録とは比べない。\
             make data で generated/ を作ると記録と比べる"
        );
        return;
    }

    let baseline = read_baseline();
    let (mut now_sum, mut base_sum, mut n) = (0.0, 0.0, 0);
    let mut worse = Vec::new();
    for (case, est) in &estimated {
        // 推定できない基準は記録にも入らないので飛ばしてよい。記録済みの基準を
        // 推定できなくなったのは、データの変更で種別グループが消えたなどの異常
        // なので、黙って比較から外さずに止める
        let Some(est) = est else {
            assert!(
                !baseline.contains_key(&case.label),
                "{}: 記録済みの基準を推定できない (種別グループがデータから消えた可能性)",
                case.label
            );
            continue;
        };
        let base = baseline.get(&case.label).unwrap_or_else(|| {
            panic!(
                "{}: 記録が無い。TRAVEL_TIMES_UPDATE_BASELINE=1 で記録を更新する",
                case.label
            )
        });
        let (now_err, base_err) = (case.typical_error(*est), case.typical_error(*base));
        if now_err > base_err + CASE_TOLERANCE {
            worse.push(format!(
                "{}: 典型的な所要時間からのずれが {:.1}% → {:.1}% (推定 {base:.1}分 → {est:.1}分)",
                case.label,
                base_err * 100.0,
                now_err * 100.0
            ));
        }
        now_sum += now_err;
        base_sum += base_err;
        n += 1;
    }
    assert!(n > 0, "このデータで推定できる基準が 1 つも無い");
    assert!(
        worse.is_empty(),
        "実際の所要時間から離れた基準がある:\n{}",
        worse.join("\n")
    );
    let (now_mean, base_mean) = (now_sum / n as f64, base_sum / n as f64);
    assert!(
        now_mean <= base_mean + MEAN_TOLERANCE,
        "典型的な所要時間からのずれの平均が {:.2}% → {:.2}% に悪化した",
        base_mean * 100.0,
        now_mean * 100.0
    );
}
