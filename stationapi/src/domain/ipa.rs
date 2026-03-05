/// Common katakana suffixes for line names, ordered longest-first for greedy matching.
const LINE_NAME_SUFFIXES: &[&str] = &["ホンセン", "シセン", "セン"];
/// Suffixes that should NOT be stripped even though they end with セン.
const LINE_NAME_SUFFIX_EXCEPTIONS: &[&str] = &["シンカンセン"];

/// Strip a common line-name suffix (線/本線/支線/新幹線) from a katakana string.
/// Returns the stem (without the suffix). If no known suffix is found, returns the input unchanged.
pub fn strip_line_name_suffix(input: &str) -> &str {
    for exception in LINE_NAME_SUFFIX_EXCEPTIONS {
        if input.ends_with(exception) {
            return input;
        }
    }
    for suffix in LINE_NAME_SUFFIXES {
        if let Some(stem) = input.strip_suffix(suffix) {
            if !stem.is_empty() {
                return stem;
            }
        }
    }
    input
}

/// Convert a katakana string to its IPA transcription.
/// Returns `None` if the input contains characters that cannot be converted.
pub fn katakana_to_ipa(input: &str) -> Option<String> {
    if input.is_empty() {
        return Some(String::new());
    }

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

        // Single character lookup — return None on unknown characters
        result.push(lookup_single(chars[i])?);

        i += 1;
    }

    Some(apply_phonological_rules(&result))
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
        ('ジ', 'ョ') => "dʑo",
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
        // 空白（全角・半角）はそのまま透過
        '　' | ' ' => return Some(Phoneme::Regular(" ")),
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
/// Returns (onset, remainder). If the string starts with a vowel, onset is "".
fn split_onset(ipa: &str) -> (&str, &str) {
    // Find where the first vowel-like character starts
    let vowel_start = ipa
        .char_indices()
        .find(|(_, c)| "aiɯeouəɐ".contains(*c))
        .map(|(i, _)| i)
        .unwrap_or(ipa.len());
    ipa.split_at(vowel_start)
}

/// Strip secondary articulation markers (e.g., palatalization ʲ) from an onset,
/// returning only the base consonant(s).
fn strip_secondary_articulation(onset: &str) -> String {
    onset.replace('ʲ', "")
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
        || next_ipa.starts_with('j')
        || next_ipa.starts_with('ç')
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
                // For palatalized onsets (kʲ, ɡʲ, etc.), only the base consonant is geminated.
                if let Some(next_ipa) = find_next_regular(&phonemes[i + 1..]) {
                    if next_ipa.starts_with("t͡ɕ") || next_ipa.starts_with("t͡s") {
                        output.push('t');
                    } else if next_ipa.starts_with("dʑ") || next_ipa.starts_with("ʤ") {
                        output.push('d');
                    } else {
                        let (onset, _) = split_onset(next_ipa);
                        if !onset.is_empty() {
                            let base = strip_secondary_articulation(onset);
                            if let Some(c) = base.chars().next() {
                                output.push(c);
                            }
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

    /// Helper: unwrap the Option for concise test assertions.
    fn ipa(input: &str) -> String {
        katakana_to_ipa(input).expect("expected valid katakana input")
    }

    // Tests based on the hardcoded IPA mappings from Cloud Functions tts.ts

    #[test]
    fn test_shibuya() {
        assert_eq!(ipa("シブヤ"), "ɕibɯja");
    }

    #[test]
    fn test_shinagawa() {
        assert_eq!(ipa("シナガワ"), "ɕinaɡawa");
    }

    #[test]
    fn test_ueno() {
        assert_eq!(ipa("ウエノ"), "ɯeno");
    }

    #[test]
    fn test_ikebukuro() {
        assert_eq!(ipa("イケブクロ"), "ikebɯkɯɾo");
    }

    #[test]
    fn test_shinjuku() {
        // ン before ジュ → ɲ, ジュ → dʑɯ
        assert_eq!(ipa("シンジュク"), "ɕiɲdʑɯkɯ");
    }

    #[test]
    fn test_osaka() {
        // オオ → oː
        assert_eq!(ipa("オオサカ"), "oːsaka");
    }

    #[test]
    fn test_kyoto() {
        // キョウ → kʲoː (via kʲo + ウ → oɯ → oː)
        assert_eq!(ipa("キョウト"), "kʲoːto");
    }

    #[test]
    fn test_yokohama() {
        assert_eq!(ipa("ヨコハマ"), "jokohama");
    }

    #[test]
    fn test_chiba() {
        assert_eq!(ipa("チバ"), "t͡ɕiba");
    }

    #[test]
    fn test_kawasaki() {
        assert_eq!(ipa("カワサキ"), "kawasakʲi");
    }

    #[test]
    fn test_tsurumi() {
        assert_eq!(ipa("ツルミ"), "t͡sɯɾɯmi");
    }

    #[test]
    fn test_ryogoku() {
        // リョウ → ɾʲoː (via ɾʲo + ウ → oɯ → oː)
        assert_eq!(ipa("リョウゴク"), "ɾʲoːɡokɯ");
    }

    #[test]
    fn test_shimbashi() {
        // ン before バ → m
        assert_eq!(ipa("シンバシ"), "ɕimbaɕi");
    }

    #[test]
    fn test_keisei() {
        assert_eq!(ipa("ケイセイ"), "keisei");
    }

    #[test]
    fn test_oshiage() {
        assert_eq!(ipa("オシアゲ"), "oɕiaɡe");
    }

    #[test]
    fn test_meitetsu() {
        // ツ is consistently t͡sɯ (affricate with tie bar)
        assert_eq!(ipa("メイテツ"), "meitet͡sɯ");
    }

    #[test]
    fn test_seibu() {
        assert_eq!(ipa("セイブ"), "seibɯ");
    }

    #[test]
    fn test_toride() {
        assert_eq!(ipa("トリデ"), "toɾide");
    }

    #[test]
    fn test_fukiage() {
        assert_eq!(ipa("フキアゲ"), "ɸɯkʲiaɡe");
    }

    #[test]
    fn test_fuse() {
        assert_eq!(ipa("フセ"), "ɸɯse");
    }

    #[test]
    fn test_inagekaigan() {
        // ン at word end → ɴ
        assert_eq!(ipa("イナゲカイガン"), "inaɡekaiɡaɴ");
    }

    #[test]
    fn test_inage() {
        assert_eq!(ipa("イナゲ"), "inaɡe");
    }

    #[test]
    fn test_kire_uriwari() {
        assert_eq!(ipa("キレウリワリ"), "kʲiɾeɯɾiwaɾi");
    }

    #[test]
    fn test_yao() {
        assert_eq!(ipa("ヤオ"), "jao");
    }

    #[test]
    fn test_mejiro() {
        assert_eq!(ipa("メジロ"), "meʤiɾo");
    }

    #[test]
    fn test_isesaki() {
        assert_eq!(ipa("イセサキ"), "isesakʲi");
    }

    #[test]
    fn test_ube() {
        assert_eq!(ipa("ウベ"), "ɯbe");
    }

    #[test]
    fn test_itchome() {
        // ッチョウ → tt͡ɕoː
        assert_eq!(ipa("イッチョウメ"), "itt͡ɕoːme");
    }

    #[test]
    fn test_sanchome() {
        assert_eq!(ipa("サンチョウメ"), "sant͡ɕoːme");
    }

    #[test]
    fn test_koen() {
        // コウエン: コ=ko, ウ→長音化でoː, エン=eɴ → koːeɴ
        // Note: the original hardcoded value was "koeɴ" but phonologically "koːeɴ" is correct
        assert_eq!(ipa("コウエン"), "koːeɴ");
    }

    #[test]
    fn test_long_vowel_mark() {
        // ー explicitly lengthens
        assert_eq!(ipa("ラーメン"), "ɾaːmeɴ");
    }

    #[test]
    fn test_tokyo() {
        // トウキョウ: ト=to, ウ→oː, キョ=kʲo, ウ→oː
        assert_eq!(ipa("トウキョウ"), "toːkʲoː");
    }

    #[test]
    fn test_nagoya() {
        assert_eq!(ipa("ナゴヤ"), "naɡoja");
    }

    #[test]
    fn test_sapporo() {
        // ッポ → ppo
        assert_eq!(ipa("サッポロ"), "sappoɾo");
    }

    #[test]
    fn test_namba() {
        // ン before バ → m
        assert_eq!(ipa("ナンバ"), "namba");
    }

    #[test]
    fn test_shin_yokohama() {
        // ン before ヨ(j) → ɲ (palatal assimilation)
        assert_eq!(ipa("シンヨコハマ"), "ɕiɲjokohama");
    }

    #[test]
    fn test_geminate_ji() {
        // ッジ → dʤi (voiced affricate gemination emits 'd')
        assert_eq!(ipa("カッジ"), "kadʤi");
    }

    #[test]
    fn test_geminate_ju() {
        // ッジュ → ddʑɯ (voiced affricate gemination with digraph)
        assert_eq!(ipa("カッジュ"), "kaddʑɯ");
    }

    #[test]
    fn test_empty() {
        assert_eq!(katakana_to_ipa(""), Some(String::new()));
    }

    #[test]
    fn test_unknown_characters_returns_none() {
        assert_eq!(katakana_to_ipa("ABC"), None);
        assert_eq!(katakana_to_ipa("シブヤX"), None);
    }

    #[test]
    fn test_geminate_palatalized() {
        // ッキョ → kkʲo (only the base consonant 'k' is geminated, not 'kʲ')
        assert_eq!(ipa("ニッキョウ"), "ɲikkʲoː");
    }

    #[test]
    fn test_dokkyo_daigakumae_soka_matsubara() {
        // Full-width space between words should be preserved
        assert_eq!(
            ipa("ドッキョウダイガクマエ　ソウカマツバラ"),
            "dokkʲoːdaiɡakɯmae soːkamat͡sɯbaɾa"
        );
    }

    #[test]
    fn test_dokkyo_daigakumae_soka_matsubara_halfwidth() {
        // Half-width (ASCII) space between words should also be accepted
        assert_eq!(
            ipa("ドッキョウダイガクマエ ソウカマツバラ"),
            "dokkʲoːdaiɡakɯmae soːkamat͡sɯbaɾa"
        );
    }

    // ============================================
    // strip_line_name_suffix tests
    // ============================================

    #[test]
    fn test_strip_sen() {
        assert_eq!(strip_line_name_suffix("セイブイケブクロセン"), "セイブイケブクロ");
    }

    #[test]
    fn test_strip_honsen() {
        assert_eq!(strip_line_name_suffix("トウカイドウホンセン"), "トウカイドウ");
    }

    #[test]
    fn test_strip_shinkansen_preserved() {
        // 新幹線(Shinkansen)は英語でもそのまま使われるので除去しない
        assert_eq!(strip_line_name_suffix("トウホクシンカンセン"), "トウホクシンカンセン");
    }

    #[test]
    fn test_strip_shisen() {
        assert_eq!(strip_line_name_suffix("ナガノハラクサツグチシセン"), "ナガノハラクサツグチ");
    }

    #[test]
    fn test_strip_no_suffix() {
        // ライン等セン以外の末尾はそのまま返す
        assert_eq!(strip_line_name_suffix("ショウナンシンジュクライン"), "ショウナンシンジュクライン");
    }

    #[test]
    fn test_strip_bare_sen_returns_unchanged() {
        // "セン" だけの場合、stemが空になるので除去しない
        assert_eq!(strip_line_name_suffix("セン"), "セン");
    }
}
