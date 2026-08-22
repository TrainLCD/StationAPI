//! バス由来のレコードへ割り当てる各種コードの生成。
//!
//! 実行のたびに同じ値になる必要があるため、`DefaultHasher` ではなく FNV-1a を使う。
//! 値域は鉄道側と衝突しないように前置してある。
//!
//! ハッシュなので別々の入力が同じ値になりうる。実データでも
//! 都営バスと西武バスの停留所で `station_cd` が 1 件ぶつかっていた。
//! 気付かずに流すと、後段で行が捨てられて停留所が欠けるか、
//! 別事業者の停留所を指す停車駅ができてしまう。
//! [`BusCodes`] は衝突したぶんを次の空き値へずらして、必ず一意にする。

use std::collections::{HashMap, HashSet};

use anyhow::{bail, Result};

/// FNV-1a。プロセスをまたいでも同じ値を返す。
fn fnv1a_hash(data: &[u8]) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

    let mut hash = FNV_OFFSET_BASIS;
    for byte in data {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// 1 つの値域を管理して、入力ごとに一意な ID を割り当てる。
///
/// 同じ入力には必ず同じ値を返す。ハッシュがぶつかったときだけ、
/// 値域内で次の空きへ 1 つずつずらす。入力を与える順序が同じなら
/// 結果も同じになる。
struct Allocator {
    kind: &'static str,
    base: i32,
    span: i32,
    by_input: HashMap<String, i32>,
    used: HashSet<i32>,
    collisions: usize,
}

impl Allocator {
    fn new(kind: &'static str, base: i32, span: i32) -> Self {
        Allocator {
            kind,
            base,
            span,
            by_input: HashMap::new(),
            used: HashSet::new(),
            collisions: 0,
        }
    }

    fn allocate(&mut self, input: &str) -> Result<i32> {
        if let Some(code) = self.by_input.get(input) {
            return Ok(*code);
        }
        let start = (fnv1a_hash(input.as_bytes()) % self.span as u64) as i32;
        for offset in 0..self.span {
            let code = self.base + (start + offset) % self.span;
            if self.used.insert(code) {
                if offset > 0 {
                    self.collisions += 1;
                }
                self.by_input.insert(input.to_string(), code);
                return Ok(code);
            }
        }
        bail!("{} の値域 ({} 件) を使い切った", self.kind, self.span)
    }

    fn get(&self, input: &str) -> Option<i32> {
        self.by_input.get(input).copied()
    }
}

/// バス用 ID の割り当て一式。
///
/// `stops_to_stations` と `trip_variations_to_types` は同じ停留所に対して
/// 同じ `station_cd` を必要とするので、必ず同じインスタンスを通す。
pub struct BusCodes {
    line: Allocator,
    station: Allocator,
    train_type: Allocator,
    line_group: Allocator,
}

impl Default for BusCodes {
    fn default() -> Self {
        BusCodes {
            // 既存の鉄道データと重ならない値域
            line: Allocator::new("line_cd", 100_000_000, 10_000_000),
            station: Allocator::new("station_cd", 200_000_000, 100_000_000),
            train_type: Allocator::new("type_cd", 100_000_000, 100_000_000),
            line_group: Allocator::new("line_group_cd", 100_000_000, 100_000_000),
        }
    }
}

impl BusCodes {
    pub fn line_cd(&mut self, route_id: &str) -> Result<i32> {
        self.line.allocate(route_id)
    }

    pub fn station_cd(&mut self, stop_id: &str, route_id: &str) -> Result<i32> {
        self.station.allocate(&format!("{stop_id}-{route_id}"))
    }

    /// 既に割り当て済みの `station_cd` だけを引く。
    ///
    /// 停車駅として書き出す行は、駅として書き出した停留所に限る。
    /// ここで新たに割り当てると、存在しない駅を指す行ができる。
    pub fn existing_station_cd(&self, stop_id: &str, route_id: &str) -> Option<i32> {
        self.station.get(&format!("{stop_id}-{route_id}"))
    }

    pub fn type_cd(&mut self, route_id: &str, shape_id: &str) -> Result<i32> {
        self.train_type
            .allocate(&format!("type-{route_id}-{shape_id}"))
    }

    pub fn line_group_cd(&mut self, route_id: &str, shape_id: &str) -> Result<i32> {
        self.line_group
            .allocate(&format!("lg-{route_id}-{shape_id}"))
    }

    /// ずらして解決した件数。0 でなければログに残す。
    pub fn collisions(&self) -> usize {
        self.line.collisions
            + self.station.collisions
            + self.train_type.collisions
            + self.line_group.collisions
    }
}

/// stop_id だけから station_g_cd を作る。同じ停留所は路線をまたいで同じ値になる。
///
/// これは駅をまとめる鍵であって一意である必要が無いため、ずらさない。
pub fn bus_station_g_cd(stop_id: &str) -> i32 {
    200_000_000 + (fnv1a_hash(stop_id.as_bytes()) % 100_000_000) as i32
}

/// route_id の接頭辞から事業者を引く。未知の接頭辞は取り込まない。
pub fn company_cd_for_gtfs_route(route_id: &str) -> Option<i32> {
    if route_id.starts_with("toei:") {
        Some(119) // 東京都交通局
    } else if route_id.starts_with("seibu:") {
        Some(253) // 西武バス
    } else if route_id.starts_with("keio:") {
        Some(254) // 京王バス
    } else if route_id.starts_with("tokyu_") {
        Some(255) // 東急バス
    } else {
        None
    }
}

/// ひらがなをカタカナへ寄せる。
pub fn hiragana_to_katakana(s: &str) -> String {
    s.chars()
        .map(|c| {
            if ('\u{3041}'..='\u{3096}').contains(&c) {
                char::from_u32(c as u32 + 0x60).unwrap_or(c)
            } else {
                c
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_stable() {
        // 値が変わると既存の station_cd / line_cd が総入れ替えになるため固定する。
        assert_eq!(fnv1a_hash(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a_hash(b"a"), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn generated_codes_are_in_range() {
        let mut codes = BusCodes::default();
        let line = codes.line_cd("toei:1001").unwrap();
        assert!((100_000_000..110_000_000).contains(&line));
        let station = codes.station_cd("toei:1", "toei:1001").unwrap();
        assert!((200_000_000..300_000_000).contains(&station));
    }

    #[test]
    fn same_input_always_gets_the_same_code() {
        let mut codes = BusCodes::default();
        let first = codes.station_cd("toei:1", "toei:1001").unwrap();
        let second = codes.station_cd("toei:1", "toei:1001").unwrap();
        assert_eq!(first, second);
        assert_eq!(codes.collisions(), 0);
    }

    #[test]
    fn colliding_inputs_get_distinct_codes() {
        // 値域を 2 まで狭めれば 3 件目で必ずぶつかる。
        let mut allocator = Allocator::new("test", 1_000, 2);
        let a = allocator.allocate("a").unwrap();
        let b = allocator.allocate("b").unwrap();
        assert_ne!(a, b);
        assert_eq!(allocator.allocate("a").unwrap(), a);
        // 空きが無くなったら黙って重複させず失敗させる
        assert!(allocator.allocate("c").is_err());
    }

    #[test]
    fn existing_station_cd_does_not_allocate() {
        let mut codes = BusCodes::default();
        assert_eq!(codes.existing_station_cd("toei:1", "toei:1001"), None);
        let code = codes.station_cd("toei:1", "toei:1001").unwrap();
        assert_eq!(codes.existing_station_cd("toei:1", "toei:1001"), Some(code));
    }

    #[test]
    fn hiragana_to_katakana_converts_only_hiragana() {
        assert_eq!(hiragana_to_katakana("しんじゅく"), "シンジュク");
        assert_eq!(hiragana_to_katakana("東京えき"), "東京エキ");
        assert_eq!(hiragana_to_katakana("アイウエオ"), "アイウエオ");
    }
}
