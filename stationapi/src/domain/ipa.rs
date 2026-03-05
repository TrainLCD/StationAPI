/// Convert a katakana string to its IPA transcription.
pub fn katakana_to_ipa(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut result = Vec::new();
    let mut i = 0;

    while i < len {
        // Try two-character combinations first (palatalized sounds: キョ, シャ, etc.)
        if i + 1 < len {
            if let Some(ipa) = lookup_digraph(chars[i], chars[i + 1]) {
                result.push(ipa);
                i += 2;
                continue;
            }
        }

        // Single character lookup
        if let Some(ipa) = lookup_single(chars[i]) {
            result.push(ipa);
        }
        // Skip unknown characters silently

        i += 1;
    }

    apply_phonological_rules(&result)
}

/// Look up a two-character (digraph) combination.
/// Handles palatalized sounds (拗音): キャ, シュ, チョ, etc.
fn lookup_digraph(c1: char, c2: char) -> Option<Phoneme> {
    let ipa = match (c1, c2) {
        // カ行拗音
        ('キ', 'ャ') => "kʲa",
        ('キ', 'ュ') => "kʲɯ",
        ('キ', 'ョ') => "kʲo",
        // サ行拗音 (シ is already palatal)
        ('シ', 'ャ') => "ɕa",
        ('シ', 'ュ') => "ɕɯ",
        ('シ', 'ョ') => "ɕo",
        // タ行拗音
        ('チ', 'ャ') => "t͡ɕa",
        ('チ', 'ュ') => "t͡ɕɯ",
        ('チ', 'ョ') => "t͡ɕo",
        // ナ行拗音
        ('ニ', 'ャ') => "ɲa",
        ('ニ', 'ュ') => "ɲɯ",
        ('ニ', 'ョ') => "ɲo",
        // ハ行拗音
        ('ヒ', 'ャ') => "ça",
        ('ヒ', 'ュ') => "çɯ",
        ('ヒ', 'ョ') => "ço",
        // マ行拗音
        ('ミ', 'ャ') => "mʲa",
        ('ミ', 'ュ') => "mʲɯ",
        ('ミ', 'ョ') => "mʲo",
        // ラ行拗音
        ('リ', 'ャ') => "ɾʲa",
        ('リ', 'ュ') => "ɾʲɯ",
        ('リ', 'ョ') => "ɾʲo",
        // ガ行拗音
        ('ギ', 'ャ') => "ɡʲa",
        ('ギ', 'ュ') => "ɡʲɯ",
        ('ギ', 'ョ') => "ɡʲo",
        // ザ行拗音 (ジ is voiced postalveolar affricate)
        ('ジ', 'ャ') => "dʑa",
        ('ジ', 'ュ') => "dʑɯ",
        ('ジ', 'ョ') => "ʤo",
        // バ行拗音
        ('ビ', 'ャ') => "bʲa",
        ('ビ', 'ュ') => "bʲɯ",
        ('ビ', 'ョ') => "bʲo",
        // ピ行拗音
        ('ピ', 'ャ') => "pʲa",
        ('ピ', 'ュ') => "pʲɯ",
        ('ピ', 'ョ') => "pʲo",
        _ => return None,
    };
    Some(Phoneme::Regular(ipa))
}

/// Look up a single katakana character.
fn lookup_single(c: char) -> Option<Phoneme> {
    let ipa = match c {
        // 母音
        'ア' | 'ァ' => return Some(Phoneme::Regular("a")),
        'イ' | 'ィ' => return Some(Phoneme::Regular("i")),
        'ウ' | 'ゥ' => return Some(Phoneme::Regular("ɯ")),
        'エ' | 'ェ' => return Some(Phoneme::Regular("e")),
        'オ' | 'ォ' => return Some(Phoneme::Regular("o")),
        // カ行
        'カ' => "ka",
        'キ' => "kʲi",
        'ク' => "kɯ",
        'ケ' => "ke",
        'コ' => "ko",
        // サ行
        'サ' => "sa",
        'シ' => "ɕi",
        'ス' => "sɯ",
        'セ' => "se",
        'ソ' => "so",
        // タ行
        'タ' => "ta",
        'チ' => "t͡ɕi",
        'ツ' => "t͡sɯ",
        'テ' => "te",
        'ト' => "to",
        // ナ行
        'ナ' => "na",
        'ニ' => "ɲi",
        'ヌ' => "nɯ",
        'ネ' => "ne",
        'ノ' => "no",
        // ハ行
        'ハ' => "ha",
        'ヒ' => "çi",
        'フ' => "ɸɯ",
        'ヘ' => "he",
        'ホ' => "ho",
        // マ行
        'マ' => "ma",
        'ミ' => "mi",
        'ム' => "mɯ",
        'メ' => "me",
        'モ' => "mo",
        // ヤ行
        'ヤ' | 'ャ' => "ja",
        'ユ' | 'ュ' => "jɯ",
        'ヨ' | 'ョ' => "jo",
        // ラ行
        'ラ' => "ɾa",
        'リ' => "ɾi",
        'ル' => "ɾɯ",
        'レ' => "ɾe",
        'ロ' => "ɾo",
        // ワ行
        'ワ' => "wa",
        'ヰ' => "i",
        'ヱ' => "e",
        'ヲ' => "o",
        // ガ行
        'ガ' => "ɡa",
        'ギ' => "ɡi",
        'グ' => "ɡɯ",
        'ゲ' => "ɡe",
        'ゴ' => "ɡo",
        // ザ行
        'ザ' => "za",
        'ジ' => "ʤi",
        'ズ' => "zɯ",
        'ゼ' => "ze",
        'ゾ' => "zo",
        // ダ行
        'ダ' => "da",
        'ヂ' => "dʑi",
        'ヅ' => "dzɯ",
        'デ' => "de",
        'ド' => "do",
        // バ行
        'バ' => "ba",
        'ビ' => "bi",
        'ブ' => "bɯ",
        'ベ' => "be",
        'ボ' => "bo",
        // パ行
        'パ' => "pa",
        'ピ' => "pi",
        'プ' => "pɯ",
        'ペ' => "pe",
        'ポ' => "po",
        // 特殊
        'ン' => return Some(Phoneme::MoraicNasal),
        'ッ' => return Some(Phoneme::Geminate),
        'ー' => return Some(Phoneme::LongVowel),
        _ => return None,
    };
    Some(Phoneme::Regular(ipa))
}

/// Intermediate phoneme representation before phonological rules are applied.
#[derive(Debug, Clone)]
enum Phoneme {
    Regular(&'static str),
    MoraicNasal, // ン - assimilates to following consonant
    Geminate,    // ッ - doubles following consonant
    LongVowel,   // ー - lengthens preceding vowel
}

/// Extract the leading consonant cluster from an IPA string.
/// Returns (consonant_cluster, remainder) or None if starts with a vowel.
fn split_onset(ipa: &str) -> (&str, &str) {
    // Find where the first vowel-like character starts
    let vowel_start = ipa
        .char_indices()
        .find(|(_, c)| "aiɯeouəɐ".contains(*c))
        .map(|(i, _)| i)
        .unwrap_or(ipa.len());
    ipa.split_at(vowel_start)
}

/// Get the last vowel character from an IPA string for long vowel extension.
fn last_vowel(ipa: &str) -> Option<&'static str> {
    for c in ipa.chars().rev() {
        match c {
            'a' => return Some("a"),
            'i' => return Some("i"),
            'ɯ' => return Some("ɯ"),
            'e' => return Some("e"),
            'o' => return Some("o"),
            'u' => return Some("u"),
            _ => continue,
        }
    }
    None
}

/// Classify the place of articulation of the following phoneme for ン assimilation.
fn nasal_for_following(next_ipa: &str) -> &'static str {
    // Check first meaningful character(s) of the following phoneme
    if next_ipa.starts_with('b') || next_ipa.starts_with('p') || next_ipa.starts_with('m') {
        "m" // bilabial assimilation
    } else if next_ipa.starts_with('ɲ')
        || next_ipa.starts_with("dʑ")
        || next_ipa.starts_with('ʤ')
        || next_ipa.starts_with('ɕ')
        || next_ipa.starts_with("ɡʲ")
        || next_ipa.starts_with("kʲ")
    {
        "ɲ" // palatal assimilation
    } else if next_ipa.starts_with('k') || next_ipa.starts_with('ɡ') || next_ipa.starts_with('ŋ')
    {
        "ŋ" // velar assimilation
    } else if next_ipa.starts_with('n')
        || next_ipa.starts_with('t')
        || next_ipa.starts_with('d')
        || next_ipa.starts_with('s')
        || next_ipa.starts_with('z')
        || next_ipa.starts_with('ɾ')
    {
        "n" // alveolar assimilation (includes t͡ɕ, t͡s which start with t)
    } else {
        "ɴ" // default: uvular nasal (word-final or before vowels)
    }
}

/// Apply phonological rules: ン assimilation, ッ gemination, long vowels.
fn apply_phonological_rules(phonemes: &[Phoneme]) -> String {
    let mut output = String::new();
    let len = phonemes.len();
    let mut i = 0;

    while i < len {
        match &phonemes[i] {
            Phoneme::Regular(ipa) => {
                output.push_str(ipa);
                i += 1;
            }
            Phoneme::MoraicNasal => {
                // Look ahead for assimilation
                if let Some(next_ipa) = find_next_regular(&phonemes[i + 1..]) {
                    output.push_str(nasal_for_following(next_ipa));
                } else {
                    output.push('ɴ'); // word-final
                }
                i += 1;
            }
            Phoneme::Geminate => {
                // Double the onset of the following consonant.
                // For affricates (t͡ɕ, t͡s), only the stop portion (t) is geminated.
                if let Some(next_ipa) = find_next_regular(&phonemes[i + 1..]) {
                    if next_ipa.starts_with("t͡ɕ")
                        || next_ipa.starts_with("t͡s")
                        || next_ipa.starts_with("d͡")
                    {
                        output.push('t');
                    } else {
                        let (onset, _) = split_onset(next_ipa);
                        if !onset.is_empty() {
                            output.push_str(onset);
                        }
                    }
                }
                i += 1;
            }
            Phoneme::LongVowel => {
                // Lengthen the preceding vowel
                if last_vowel(&output).is_some() {
                    // Check if already has ː
                    if !output.ends_with('ː') {
                        output.push('ː');
                    }
                } else {
                    output.push('ː');
                }
                i += 1;
            }
        }
    }

    // Apply long vowel contractions: オウ → oː pattern
    apply_vowel_length(&output)
}

/// Find the IPA string of the next Regular phoneme in the slice.
fn find_next_regular(phonemes: &[Phoneme]) -> Option<&'static str> {
    phonemes.iter().find_map(|p| match p {
        Phoneme::Regular(ipa) => Some(*ipa),
        _ => None,
    })
}

/// Apply vowel length rules for common Japanese patterns.
/// オウ → oː (after consonant+o), ョウ/ョオ patterns are handled by digraph + this.
fn apply_vowel_length(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if i + 1 < len && chars[i] == 'o' && chars[i + 1] == 'ɯ' {
            // oɯ → oː (おう/こう pattern)
            result.push('o');
            result.push('ː');
            i += 2;
            continue;
        }
        if i + 1 < len && chars[i] == 'o' && chars[i + 1] == 'o' {
            // oo → oː (おお pattern)
            result.push('o');
            result.push('ː');
            i += 2;
            continue;
        }
        result.push(chars[i]);
        i += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests based on the hardcoded IPA mappings from Cloud Functions tts.ts

    #[test]
    fn test_shibuya() {
        assert_eq!(katakana_to_ipa("シブヤ"), "ɕibɯja");
    }

    #[test]
    fn test_shinagawa() {
        assert_eq!(katakana_to_ipa("シナガワ"), "ɕinaɡawa");
    }

    #[test]
    fn test_ueno() {
        assert_eq!(katakana_to_ipa("ウエノ"), "ɯeno");
    }

    #[test]
    fn test_ikebukuro() {
        assert_eq!(katakana_to_ipa("イケブクロ"), "ikebɯkɯɾo");
    }

    #[test]
    fn test_shinjuku() {
        // ン before ジュ → ɲ, ジュ → dʑɯ
        assert_eq!(katakana_to_ipa("シンジュク"), "ɕiɲdʑɯkɯ");
    }

    #[test]
    fn test_osaka() {
        // オオ → oː
        assert_eq!(katakana_to_ipa("オオサカ"), "oːsaka");
    }

    #[test]
    fn test_kyoto() {
        // キョウ → kʲoː (via kʲo + ウ → oɯ → oː)
        assert_eq!(katakana_to_ipa("キョウト"), "kʲoːto");
    }

    #[test]
    fn test_yokohama() {
        assert_eq!(katakana_to_ipa("ヨコハマ"), "jokohama");
    }

    #[test]
    fn test_chiba() {
        assert_eq!(katakana_to_ipa("チバ"), "t͡ɕiba");
    }

    #[test]
    fn test_kawasaki() {
        assert_eq!(katakana_to_ipa("カワサキ"), "kawasakʲi");
    }

    #[test]
    fn test_tsurumi() {
        assert_eq!(katakana_to_ipa("ツルミ"), "t͡sɯɾɯmi");
    }

    #[test]
    fn test_ryogoku() {
        // リョウ → ɾʲoː (via ɾʲo + ウ → oɯ → oː)
        assert_eq!(katakana_to_ipa("リョウゴク"), "ɾʲoːɡokɯ");
    }

    #[test]
    fn test_shimbashi() {
        // ン before バ → m
        assert_eq!(katakana_to_ipa("シンバシ"), "ɕimbaɕi");
    }

    #[test]
    fn test_keisei() {
        assert_eq!(katakana_to_ipa("ケイセイ"), "keisei");
    }

    #[test]
    fn test_oshiage() {
        assert_eq!(katakana_to_ipa("オシアゲ"), "oɕiaɡe");
    }

    #[test]
    fn test_meitetsu() {
        // ツ is consistently t͡sɯ (affricate with tie bar)
        assert_eq!(katakana_to_ipa("メイテツ"), "meitet͡sɯ");
    }

    #[test]
    fn test_seibu() {
        assert_eq!(katakana_to_ipa("セイブ"), "seibɯ");
    }

    #[test]
    fn test_toride() {
        assert_eq!(katakana_to_ipa("トリデ"), "toɾide");
    }

    #[test]
    fn test_fukiage() {
        assert_eq!(katakana_to_ipa("フキアゲ"), "ɸɯkʲiaɡe");
    }

    #[test]
    fn test_fuse() {
        assert_eq!(katakana_to_ipa("フセ"), "ɸɯse");
    }

    #[test]
    fn test_inagekaigan() {
        // ン at word end → ɴ
        assert_eq!(katakana_to_ipa("イナゲカイガン"), "inaɡekaiɡaɴ");
    }

    #[test]
    fn test_inage() {
        assert_eq!(katakana_to_ipa("イナゲ"), "inaɡe");
    }

    #[test]
    fn test_kire_uriwari() {
        assert_eq!(katakana_to_ipa("キレウリワリ"), "kʲiɾeɯɾiwaɾi");
    }

    #[test]
    fn test_yao() {
        assert_eq!(katakana_to_ipa("ヤオ"), "jao");
    }

    #[test]
    fn test_mejiro() {
        assert_eq!(katakana_to_ipa("メジロ"), "meʤiɾo");
    }

    #[test]
    fn test_isesaki() {
        assert_eq!(katakana_to_ipa("イセサキ"), "isesakʲi");
    }

    #[test]
    fn test_ube() {
        assert_eq!(katakana_to_ipa("ウベ"), "ɯbe");
    }

    #[test]
    fn test_itchome() {
        // ッチョウ → tt͡ɕoː
        assert_eq!(katakana_to_ipa("イッチョウメ"), "itt͡ɕoːme");
    }

    #[test]
    fn test_sanchome() {
        assert_eq!(katakana_to_ipa("サンチョウメ"), "sant͡ɕoːme");
    }

    #[test]
    fn test_koen() {
        // コウエン: コ=ko, ウ→長音化でoː, エン=eɴ → koːeɴ
        // Note: the original hardcoded value was "koeɴ" but phonologically "koːeɴ" is correct
        assert_eq!(katakana_to_ipa("コウエン"), "koːeɴ");
    }

    #[test]
    fn test_long_vowel_mark() {
        // ー explicitly lengthens
        assert_eq!(katakana_to_ipa("ラーメン"), "ɾaːmeɴ");
    }

    #[test]
    fn test_tokyo() {
        // トウキョウ: ト=to, ウ→oː, キョ=kʲo, ウ→oː
        assert_eq!(katakana_to_ipa("トウキョウ"), "toːkʲoː");
    }

    #[test]
    fn test_nagoya() {
        assert_eq!(katakana_to_ipa("ナゴヤ"), "naɡoja");
    }

    #[test]
    fn test_sapporo() {
        // ッポ → ppo
        assert_eq!(katakana_to_ipa("サッポロ"), "sappoɾo");
    }

    #[test]
    fn test_namba() {
        // ン before バ → m
        assert_eq!(katakana_to_ipa("ナンバ"), "namba");
    }

    #[test]
    fn test_empty() {
        assert_eq!(katakana_to_ipa(""), "");
    }
}
