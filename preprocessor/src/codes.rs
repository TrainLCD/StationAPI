//! バス由来のレコードへ割り当てる各種コードの生成。
//!
//! 実行のたびに同じ値になる必要があるため、`DefaultHasher` ではなく FNV-1a を使う。
//! 値域は鉄道側と衝突しないように前置してある。

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

/// route_id から line_cd を作る。既存の鉄道路線と衝突しない 1 億台を使う。
pub fn bus_line_cd(route_id: &str) -> i32 {
    100_000_000 + (fnv1a_hash(route_id.as_bytes()) % 10_000_000) as i32
}

/// (stop_id, route_id) から station_cd を作る。2 億台を使う。
pub fn bus_station_cd(stop_id: &str, route_id: &str) -> i32 {
    let combined = format!("{stop_id}-{route_id}");
    200_000_000 + (fnv1a_hash(combined.as_bytes()) % 100_000_000) as i32
}

/// stop_id だけから station_g_cd を作る。同じ停留所は路線をまたいで同じ値になる。
pub fn bus_station_g_cd(stop_id: &str) -> i32 {
    200_000_000 + (fnv1a_hash(stop_id.as_bytes()) % 100_000_000) as i32
}

/// (route_id, shape_id) から type_cd を作る。
pub fn bus_type_cd(route_id: &str, shape_id: &str) -> i32 {
    let combined = format!("type-{route_id}-{shape_id}");
    100_000_000 + (fnv1a_hash(combined.as_bytes()) % 100_000_000) as i32
}

/// (route_id, shape_id) から line_group_cd を作る。
pub fn bus_line_group_cd(route_id: &str, shape_id: &str) -> i32 {
    let combined = format!("lg-{route_id}-{shape_id}");
    100_000_000 + (fnv1a_hash(combined.as_bytes()) % 100_000_000) as i32
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
        let line = bus_line_cd("toei:1001");
        assert!((100_000_000..110_000_000).contains(&line));
        let station = bus_station_cd("toei:1", "toei:1001");
        assert!((200_000_000..300_000_000).contains(&station));
    }

    #[test]
    fn hiragana_to_katakana_converts_only_hiragana() {
        assert_eq!(hiragana_to_katakana("しんじゅく"), "シンジュク");
        assert_eq!(hiragana_to_katakana("東京えき"), "東京エキ");
        assert_eq!(hiragana_to_katakana("アイウエオ"), "アイウエオ");
    }
}
