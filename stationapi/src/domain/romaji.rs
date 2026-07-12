//! Katakana → Hepburn romaji conversion for bus-stop names that ship without an
//! official English reading.
//!
//! Keio Bus (GTFS) and Tokyu Bus (ODPT JSON) provide a kana reading for every
//! stop but no English (`en`) translation, so their `stop_name_r` would otherwise
//! stay empty. Because `gtfs_stops.stop_name_r` is the single upstream source that
//! fans out into the `stations` projection, name search, and the romanized bus
//! route/headsign names, filling it here supplements every English-facing surface
//! at once.
//!
//! The curated rail dataset (`data/3!stations.csv`) uses modified Hepburn with
//! macrons for long vowels (Tōkyō, Kyōto, Shin-Ōsaka), so this converter mirrors
//! that style:
//! - long o (オウ / オオ) → `ō`, palatalised long u (シュウ / チュウ) → `ū`
//! - the chōonpu `ー` lengthens the preceding vowel
//! - `ン` is always `n`, sokuon `ッ` geminates the next consonant (`ッチ` → `tch`)
//!
//! Cross-morpheme vowel runs that the hand-curated rail data keeps literal
//! (e.g. Yasuushi = ヤスウシ, Masuura = マスウラ) cannot be told apart from true long
//! vowels using the reading alone, so a plain `ウ` only lengthens after an o-sound
//! or a palatalised u-sound and otherwise stays literal — matching the rail style
//! for the cases it can, and erring toward the literal reading for the rest.
//!
//! Anything the tables cannot map (kanji, Latin letters, digits) makes the whole
//! conversion return `None`, so callers fall back to leaving the name empty rather
//! than emitting a partial, misleading transcription.

/// One romanized mora, tracking enough state to apply long-vowel rules.
struct Syllable {
    text: String,
    /// Plain base vowel (`a`/`i`/`u`/`e`/`o`) of the mora, or `None` for `ン`.
    /// Kept stable even after the text is lengthened to a macron.
    base: Option<char>,
    /// Whether this mora came from a palatalised (小ャ/ュ/ョ) digraph, which is
    /// what distinguishes a true long `ū` (シュウ) from a literal run (スウ).
    is_yoon: bool,
}

/// Convert a katakana (or hiragana) reading to lowercase modified-Hepburn romaji.
///
/// Returns `None` when the input is empty or contains a character outside the
/// kana tables, so the caller can decline to supplement rather than emit a
/// partial transcription.
pub fn katakana_to_romaji(input: &str) -> Option<String> {
    if input.trim().is_empty() {
        return None;
    }

    let chars: Vec<char> = normalize_kana(input).chars().collect();
    let len = chars.len();
    let mut syllables: Vec<Syllable> = Vec::new();
    let mut pending_sokuon = false;
    let mut i = 0;

    while i < len {
        let c = chars[i];

        // Word separators inside a compound reading collapse to a single space.
        if is_separator(c) {
            pending_sokuon = false;
            push_separator(&mut syllables);
            i += 1;
            continue;
        }

        // Chōonpu: lengthen the previous vowel to a macron.
        if c == 'ー' {
            lengthen_last(&mut syllables)?;
            i += 1;
            continue;
        }

        // Sokuon: geminate the following consonant.
        if c == 'ッ' {
            pending_sokuon = true;
            i += 1;
            continue;
        }

        // A bare ウ / オ can act as a long-vowel marker instead of its own mora.
        if !pending_sokuon && (c == 'ウ' || c == 'オ') {
            if let Some(last) = syllables.last() {
                let lengthen = match c {
                    // オ only lengthens a preceding o-sound (オオ / コオ → ō),
                    // leaving cross-morpheme オ literal (シズオカ → shizuoka).
                    'オ' => last.base == Some('o'),
                    // ウ lengthens a preceding o-sound (トウ → tō) or a
                    // palatalised u-sound (シュウ → shū); a plain u stays literal
                    // (スウ → suu).
                    _ => last.base == Some('o') || (last.is_yoon && last.base == Some('u')),
                };
                if lengthen {
                    lengthen_last(&mut syllables)?;
                    i += 1;
                    continue;
                }
            }
            // Otherwise fall through and romanize ウ / オ as a normal mora (this
            // also lets ウ still open a ウィ/ウェ/ウォ digraph below).
        }

        // Two-character digraphs (palatalised キャ etc. and loanword ファ etc.).
        if i + 1 < len {
            if let Some(romaji) = lookup_digraph(c, chars[i + 1]) {
                let is_yoon = matches!(chars[i + 1], 'ャ' | 'ュ' | 'ョ');
                push_syllable(&mut syllables, romaji, is_yoon, &mut pending_sokuon);
                i += 2;
                continue;
            }
        }

        // Single kana.
        if let Some(romaji) = lookup_single(c) {
            push_syllable(&mut syllables, romaji, false, &mut pending_sokuon);
            i += 1;
            continue;
        }

        // Unknown character: refuse to emit a partial transcription.
        return None;
    }

    let result: String = syllables.iter().map(|s| s.text.as_str()).collect();
    let result = result.trim().to_string();
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Convert a kana reading into a Title-Cased display name suitable for the
/// romanized `stop_name_r` / `station_name_r` fields.
pub fn romaji_display_name(input: &str) -> Option<String> {
    katakana_to_romaji(input).map(|romaji| title_case(&romaji))
}

/// Strip macron accents from a romanized name, mapping each long vowel back to
/// its plain ASCII letter (ō → o, ū → u, ā → a, ī → i, ē → e).
///
/// The dataset stores two romanized forms per station: `station_name_r` keeps
/// the macrons (Tōkyō) while `station_name_rn` is the plain-ASCII spelling
/// (Tokyo). This produces the latter from the former.
pub fn strip_macrons(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            'ā' => 'a',
            'ī' => 'i',
            'ū' => 'u',
            'ē' => 'e',
            'ō' => 'o',
            'Ā' => 'A',
            'Ī' => 'I',
            'Ū' => 'U',
            'Ē' => 'E',
            'Ō' => 'O',
            other => other,
        })
        .collect()
}

/// Capitalize the first letter of every whitespace/separator-delimited word.
fn title_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    let mut capitalize_next = true;
    for c in s.chars() {
        if c.is_alphabetic() {
            if capitalize_next {
                out.extend(c.to_uppercase());
                capitalize_next = false;
            } else {
                out.push(c);
            }
        } else {
            out.push(c);
            capitalize_next = true;
        }
    }
    out
}

/// Push a romanized mora, applying a pending sokuon gemination if present.
fn push_syllable(
    syllables: &mut Vec<Syllable>,
    romaji: &str,
    is_yoon: bool,
    pending_sokuon: &mut bool,
) {
    let mut text = romaji.to_string();
    if *pending_sokuon {
        *pending_sokuon = false;
        if let Some(first) = text.chars().next() {
            if is_romaji_vowel(first) {
                // Gemination before a vowel is meaningless; drop the sokuon.
            } else if text.starts_with("ch") {
                // Hepburn spells っち as "tchi", not "cchi".
                text = format!("t{text}");
            } else {
                text = format!("{first}{text}");
            }
        }
    }
    let base = base_vowel(romaji);
    syllables.push(Syllable {
        text,
        base,
        is_yoon,
    });
}

/// Emit a single space separator, coalescing consecutive separators.
fn push_separator(syllables: &mut Vec<Syllable>) {
    if syllables.last().map(|s| s.text.as_str()) == Some(" ") {
        return;
    }
    syllables.push(Syllable {
        text: " ".to_string(),
        base: None,
        is_yoon: false,
    });
}

/// Replace the final vowel of the last mora with its macron form.
/// Returns `None` when there is nothing to lengthen or the mora ends in a
/// consonant (e.g. a stray `ー` after `ン`), failing the whole conversion.
fn lengthen_last(syllables: &mut [Syllable]) -> Option<()> {
    let last = syllables.last_mut()?;
    let mut chars: Vec<char> = last.text.chars().collect();
    let final_c = *chars.last()?;
    let macron = match final_c {
        'a' => 'ā',
        'i' => 'ī',
        'u' => 'ū',
        'e' => 'ē',
        'o' => 'ō',
        // Already long: an extra ー/ウ is redundant, keep as-is.
        'ā' | 'ī' | 'ū' | 'ē' | 'ō' => return Some(()),
        _ => return None,
    };
    chars.pop();
    chars.push(macron);
    last.text = chars.into_iter().collect();
    Some(())
}

/// Base vowel of a romaji mora (its last plain vowel), or `None` for `ン`.
fn base_vowel(romaji: &str) -> Option<char> {
    romaji.chars().rev().find(|c| is_romaji_vowel(*c))
}

fn is_romaji_vowel(c: char) -> bool {
    matches!(c, 'a' | 'i' | 'u' | 'e' | 'o')
}

fn is_separator(c: char) -> bool {
    matches!(c, '\u{30FB}' | ' ' | '\u{3000}' | '/' | '\u{FF0F}')
}

/// Normalize hiragana to katakana so both reading styles convert identically.
fn normalize_kana(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if ('\u{3041}'..='\u{3096}').contains(&c) {
                char::from_u32(c as u32 + 0x60).unwrap_or(c)
            } else {
                c
            }
        })
        .collect()
}

/// Two-character katakana combinations (palatalised sounds and loanword kana).
fn lookup_digraph(c1: char, c2: char) -> Option<&'static str> {
    let romaji = match (c1, c2) {
        // Palatalised (拗音)
        ('キ', 'ャ') => "kya",
        ('キ', 'ュ') => "kyu",
        ('キ', 'ョ') => "kyo",
        ('シ', 'ャ') => "sha",
        ('シ', 'ュ') => "shu",
        ('シ', 'ョ') => "sho",
        ('チ', 'ャ') => "cha",
        ('チ', 'ュ') => "chu",
        ('チ', 'ョ') => "cho",
        ('ニ', 'ャ') => "nya",
        ('ニ', 'ュ') => "nyu",
        ('ニ', 'ョ') => "nyo",
        ('ヒ', 'ャ') => "hya",
        ('ヒ', 'ュ') => "hyu",
        ('ヒ', 'ョ') => "hyo",
        ('ミ', 'ャ') => "mya",
        ('ミ', 'ュ') => "myu",
        ('ミ', 'ョ') => "myo",
        ('リ', 'ャ') => "rya",
        ('リ', 'ュ') => "ryu",
        ('リ', 'ョ') => "ryo",
        ('ギ', 'ャ') => "gya",
        ('ギ', 'ュ') => "gyu",
        ('ギ', 'ョ') => "gyo",
        ('ジ', 'ャ') | ('ヂ', 'ャ') => "ja",
        ('ジ', 'ュ') | ('ヂ', 'ュ') => "ju",
        ('ジ', 'ョ') | ('ヂ', 'ョ') => "jo",
        ('ビ', 'ャ') => "bya",
        ('ビ', 'ュ') => "byu",
        ('ビ', 'ョ') => "byo",
        ('ピ', 'ャ') => "pya",
        ('ピ', 'ュ') => "pyu",
        ('ピ', 'ョ') => "pyo",
        ('フ', 'ャ') => "fya",
        ('フ', 'ュ') => "fyu",
        ('フ', 'ョ') => "fyo",
        // Loanword combinations (外来音)
        ('シ', 'ェ') => "she",
        ('ジ', 'ェ') => "je",
        ('チ', 'ェ') => "che",
        ('テ', 'ィ') => "ti",
        ('テ', 'ュ') => "tyu",
        ('デ', 'ィ') => "di",
        ('デ', 'ュ') => "dyu",
        ('ト', 'ゥ') => "tu",
        ('ド', 'ゥ') => "du",
        ('フ', 'ァ') => "fa",
        ('フ', 'ィ') => "fi",
        ('フ', 'ェ') => "fe",
        ('フ', 'ォ') => "fo",
        ('ウ', 'ィ') => "wi",
        ('ウ', 'ェ') => "we",
        ('ウ', 'ォ') => "wo",
        ('ヴ', 'ァ') => "va",
        ('ヴ', 'ィ') => "vi",
        ('ヴ', 'ェ') => "ve",
        ('ヴ', 'ォ') => "vo",
        ('ヴ', 'ュ') => "vyu",
        ('ツ', 'ァ') => "tsa",
        ('ツ', 'ィ') => "tsi",
        ('ツ', 'ェ') => "tse",
        ('ツ', 'ォ') => "tso",
        ('イ', 'ェ') => "ye",
        ('ク', 'ァ') => "kwa",
        ('グ', 'ァ') => "gwa",
        _ => return None,
    };
    Some(romaji)
}

/// Single katakana → romaji.
fn lookup_single(c: char) -> Option<&'static str> {
    let romaji = match c {
        'ア' => "a",
        'イ' => "i",
        'ウ' => "u",
        'エ' => "e",
        'オ' => "o",
        'カ' => "ka",
        'キ' => "ki",
        'ク' => "ku",
        'ケ' => "ke",
        'コ' => "ko",
        'サ' => "sa",
        'シ' => "shi",
        'ス' => "su",
        'セ' => "se",
        'ソ' => "so",
        'タ' => "ta",
        'チ' => "chi",
        'ツ' => "tsu",
        'テ' => "te",
        'ト' => "to",
        'ナ' => "na",
        'ニ' => "ni",
        'ヌ' => "nu",
        'ネ' => "ne",
        'ノ' => "no",
        'ハ' => "ha",
        'ヒ' => "hi",
        'フ' => "fu",
        'ヘ' => "he",
        'ホ' => "ho",
        'マ' => "ma",
        'ミ' => "mi",
        'ム' => "mu",
        'メ' => "me",
        'モ' => "mo",
        'ヤ' => "ya",
        'ユ' => "yu",
        'ヨ' => "yo",
        'ラ' => "ra",
        'リ' => "ri",
        'ル' => "ru",
        'レ' => "re",
        'ロ' => "ro",
        'ワ' => "wa",
        'ヰ' => "wi",
        'ヱ' => "we",
        'ヲ' => "o",
        'ン' => "n",
        'ガ' => "ga",
        'ギ' => "gi",
        'グ' => "gu",
        'ゲ' => "ge",
        'ゴ' => "go",
        'ザ' => "za",
        'ジ' => "ji",
        'ズ' => "zu",
        'ゼ' => "ze",
        'ゾ' => "zo",
        'ダ' => "da",
        'ヂ' => "ji",
        'ヅ' => "zu",
        'デ' => "de",
        'ド' => "do",
        'バ' => "ba",
        'ビ' => "bi",
        'ブ' => "bu",
        'ベ' => "be",
        'ボ' => "bo",
        'パ' => "pa",
        'ピ' => "pi",
        'プ' => "pu",
        'ペ' => "pe",
        'ポ' => "po",
        'ヴ' => "vu",
        // Small kana appearing on their own (rare) fall back to their full sound.
        'ァ' => "a",
        'ィ' => "i",
        'ゥ' => "u",
        'ェ' => "e",
        'ォ' => "o",
        'ャ' => "ya",
        'ュ' => "yu",
        'ョ' => "yo",
        'ヮ' => "wa",
        _ => return None,
    };
    Some(romaji)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn romaji(input: &str) -> String {
        katakana_to_romaji(input).expect("expected convertible katakana")
    }

    #[test]
    fn basic_gojuon() {
        assert_eq!(romaji("シブヤ"), "shibuya");
        assert_eq!(romaji("シナガワ"), "shinagawa");
        assert_eq!(romaji("ナゴヤ"), "nagoya");
        assert_eq!(romaji("オダワラ"), "odawara");
    }

    #[test]
    fn long_o_uses_macron_like_rail_data() {
        assert_eq!(romaji("トウキョウ"), "tōkyō");
        assert_eq!(romaji("キョウト"), "kyōto");
        assert_eq!(romaji("シンオオサカ"), "shinōsaka");
        assert_eq!(romaji("コウベ"), "kōbe");
    }

    #[test]
    fn palatalised_long_u_uses_macron() {
        assert_eq!(romaji("カゴシマチュウオウ"), "kagoshimachūō");
        assert_eq!(romaji("マシュウ"), "mashū");
        assert_eq!(romaji("ジュウモンジ"), "jūmonji");
    }

    #[test]
    fn cross_morpheme_u_run_stays_literal() {
        // ヤスウシ (Yasuushi) / マスウラ (Masuura): the rail data keeps these literal.
        assert_eq!(romaji("ヤスウシ"), "yasuushi");
        assert_eq!(romaji("マスウラ"), "masuura");
    }

    #[test]
    fn cross_morpheme_o_stays_literal() {
        // シズオカ (Shizuoka): オ after a u-sound is a separate mora, not a long vowel.
        assert_eq!(romaji("シズオカ"), "shizuoka");
    }

    #[test]
    fn ei_is_not_lengthened() {
        assert_eq!(romaji("タイセイ"), "taisei");
        assert_eq!(romaji("ホウエイ"), "hōei");
        assert_eq!(romaji("シンメイ"), "shinmei");
    }

    #[test]
    fn sokuon_geminates_next_consonant() {
        assert_eq!(romaji("ロッポンギ"), "roppongi");
        assert_eq!(romaji("ハッサム"), "hassamu");
        // っち → tchi
        assert_eq!(romaji("マッチャ"), "matcha");
    }

    #[test]
    fn chouonpu_lengthens_preceding_vowel() {
        assert_eq!(romaji("ビール"), "bīru");
        assert_eq!(romaji("パーク"), "pāku");
        assert_eq!(romaji("スーパー"), "sūpā");
    }

    #[test]
    fn n_is_always_n() {
        assert_eq!(romaji("シンバシ"), "shinbashi");
        assert_eq!(romaji("ホンマチ"), "honmachi");
    }

    #[test]
    fn palatalised_and_loanword_digraphs() {
        assert_eq!(romaji("トウキョウジョシダイ"), "tōkyōjoshidai");
        assert_eq!(romaji("フェリー"), "ferī");
        assert_eq!(romaji("ジェイアール"), "jeiāru");
    }

    #[test]
    fn hiragana_is_accepted() {
        assert_eq!(romaji("しぶや"), "shibuya");
        assert_eq!(romaji("とうきょう"), "tōkyō");
    }

    #[test]
    fn separators_become_spaces() {
        assert_eq!(romaji("シブヤ・エキ"), "shibuya eki");
    }

    #[test]
    fn unconvertible_input_returns_none() {
        assert_eq!(katakana_to_romaji("渋谷"), None);
        assert_eq!(katakana_to_romaji("Shibuya"), None);
        assert_eq!(katakana_to_romaji("シブヤ2"), None);
        assert_eq!(katakana_to_romaji(""), None);
        assert_eq!(katakana_to_romaji("   "), None);
    }

    #[test]
    fn strip_macrons_produces_plain_ascii_form() {
        // Matches the rail dataset's _r → _rn convention.
        assert_eq!(strip_macrons("Tōkyō"), "Tokyo");
        assert_eq!(strip_macrons("Kyōto"), "Kyoto");
        assert_eq!(strip_macrons("Shin-Ōsaka"), "Shin-Osaka");
        assert_eq!(strip_macrons("Mikawa-Anjō"), "Mikawa-Anjo");
        // Already plain names are unchanged.
        assert_eq!(strip_macrons("Shinagawa"), "Shinagawa");
        // Derived bus romaji round-trips through both forms.
        let r = romaji_display_name("トウキョウ").unwrap();
        assert_eq!(r, "Tōkyō");
        assert_eq!(strip_macrons(&r), "Tokyo");
    }

    #[test]
    fn display_name_is_title_cased() {
        assert_eq!(romaji_display_name("シブヤ").as_deref(), Some("Shibuya"));
        assert_eq!(romaji_display_name("トウキョウ").as_deref(), Some("Tōkyō"));
        assert_eq!(
            romaji_display_name("シブヤエキマエ").as_deref(),
            Some("Shibuyaekimae")
        );
        assert_eq!(
            romaji_display_name("シブヤ・エキマエ").as_deref(),
            Some("Shibuya Ekimae")
        );
        assert_eq!(romaji_display_name("渋谷"), None);
    }
}
