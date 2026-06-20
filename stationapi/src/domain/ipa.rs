use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

/// Cached IPA computation result for a single name.
#[derive(Clone, Debug)]
pub struct IpaResult {
    pub name_ipa: Option<String>,
    pub name_roman_ipa: Option<String>,
    pub tts_segments: Vec<TtsNameSegment>,
}

/// Cache key: (katakana reading, romanized name). These two inputs fully
/// determine the IPA output.
type IpaCacheKey = (String, Option<String>);

static STATION_IPA_CACHE: LazyLock<RwLock<HashMap<IpaCacheKey, IpaResult>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static LINE_IPA_CACHE: LazyLock<RwLock<HashMap<IpaCacheKey, IpaResult>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Compute all three IPA outputs in a single pass, eliminating the redundant
/// double-computation of `station_name_to_tts_segments`.
fn compute_ipa(name_katakana: &str, name_roman: Option<&str>) -> IpaResult {
    let name_ipa = katakana_name_to_ipa(name_katakana);
    let tts_segments = station_name_to_tts_segments(name_katakana, name_roman);
    let name_roman_ipa = non_empty_ipa(join_tts_segment_pronunciations(&tts_segments));
    IpaResult {
        name_ipa,
        name_roman_ipa,
        tts_segments,
    }
}

fn compute_line_ipa(name_katakana: &str, name_roman: Option<&str>) -> IpaResult {
    // name_ipa は日本語読み。路線名の「線」も日本語 (セン→sen) として読ませ、
    // 英語 (laɪn / meɪn laɪn など) を混入させない。英語読みは name_roman_ipa が担う。
    let name_ipa = katakana_name_to_ipa(name_katakana);
    let tts_segments = station_name_to_tts_segments(name_katakana, name_roman);
    let name_roman_ipa = station_name_to_ipa("", name_roman);
    IpaResult {
        name_ipa,
        name_roman_ipa,
        tts_segments,
    }
}

fn cached_lookup(
    cache: &LazyLock<RwLock<HashMap<IpaCacheKey, IpaResult>>>,
    key: &IpaCacheKey,
    compute: impl FnOnce() -> IpaResult,
) -> IpaResult {
    // Fast path: read lock
    if let Some(result) = cache.read().unwrap().get(key) {
        return result.clone();
    }
    // Slow path: compute and insert
    let result = compute();
    cache.write().unwrap().insert(key.clone(), result.clone());
    result
}

/// Compute IPA for station/train-type names with memoization.
pub fn compute_ipa_cached(name_katakana: &str, name_roman: Option<&str>) -> IpaResult {
    let key = (name_katakana.to_string(), name_roman.map(str::to_string));
    cached_lookup(&STATION_IPA_CACHE, &key, || {
        compute_ipa(name_katakana, name_roman)
    })
}

/// Compute IPA for line names with memoization.
pub fn compute_line_ipa_cached(name_katakana: &str, name_roman: Option<&str>) -> IpaResult {
    let key = (name_katakana.to_string(), name_roman.map(str::to_string));
    cached_lookup(&LINE_IPA_CACHE, &key, || {
        compute_line_ipa(name_katakana, name_roman)
    })
}

/// Parse a katakana string into the intermediate `Phoneme` sequence.
/// Returns `None` if the input contains characters that cannot be converted.
fn katakana_to_phonemes(input: &str) -> Option<Vec<Phoneme>> {
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

    Some(result)
}

/// Collapse separator-derived leading/trailing/consecutive whitespace into a
/// single space, matching the historical `katakana_to_ipa` normalization.
fn normalize_ipa_whitespace(ipa: &str) -> String {
    ipa.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Convert a katakana string to its IPA transcription.
/// Returns `None` if the input contains characters that cannot be converted.
pub fn katakana_to_ipa(input: &str) -> Option<String> {
    if input.is_empty() {
        return Some(String::new());
    }

    let phonemes = katakana_to_phonemes(input)?;
    let ipa = apply_phonological_rules(&phonemes);
    Some(normalize_ipa_whitespace(&ipa))
}

/// Convert a station name to IPA.
/// Prefers the official romanized/English name when present so mixed names like
/// "Kasai-Rinkai Park" use English pronunciation for translated segments.
pub fn station_name_to_ipa(name_katakana: &str, name_roman: Option<&str>) -> Option<String> {
    let segments = station_name_to_tts_segments(name_katakana, name_roman);
    non_empty_ipa(join_tts_segment_pronunciations(&segments))
}

pub fn katakana_name_to_ipa(input: &str) -> Option<String> {
    non_empty_ipa(katakana_to_ipa(input))
}

pub fn non_empty_ipa(ipa: Option<String>) -> Option<String> {
    ipa.filter(|ipa| !ipa.is_empty())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TtsAlphabetKind {
    Ipa,
    Yomigana,
    Plain,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TtsNameSegment {
    pub surface: String,
    pub fallback_text: String,
    pub pronunciation: String,
    pub alphabet: TtsAlphabetKind,
    pub lang: &'static str,
    pub separator: String,
}

pub fn station_name_to_tts_segments(
    name_katakana: &str,
    name_roman: Option<&str>,
) -> Vec<TtsNameSegment> {
    let mut segments = name_roman
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .and_then(romanized_name_to_tts_segments)
        .filter(|segments| !segments.is_empty())
        .or_else(|| katakana_name_to_tts_segments(name_katakana))
        .unwrap_or_default();

    // tts_segments と、これらを連結した name_roman_ipa は英語音声 (Azure Dragon HD 等)
    // で読み上げる「英語読み」トラック。ら行の弾き音 ɾ は英語ではフラップ /t/・/d/
    // (water /ˈwɔːɾɚ/ の t) として実現され、「ら行」ではなく「た/だ/ち」に化ける
    // (にほんおおどおり → にほんおおどおち)。そのためこのトラックに限り ら行を
    // 側面接近音 l に置換する (例: トリデ toɾide→tolide)。英単語の R は別記号 ɹ を
    // 使うので影響しない。日本語音声で読む name_ipa (katakana_to_ipa) は honest な ɾ の
    // まま残し、将来 ja-JP 音声で読む場合に備える。
    for segment in &mut segments {
        segment.pronunciation = lateralize_ra_row(&segment.pronunciation);
    }

    segments
}

/// 英語読みトラックのら行 ɾ を側面接近音 l に置換する。英単語の R (ɹ) は対象外。
/// 英語音声では ɾ が英語のフラップ /t/ として鳴り「ら行」に聞こえないため。
fn lateralize_ra_row(pronunciation: &str) -> String {
    pronunciation.replace('ɾ', "l")
}

fn join_tts_segment_pronunciations(segments: &[TtsNameSegment]) -> Option<String> {
    let mut output = String::new();

    for segment in segments {
        if segment.pronunciation.is_empty() {
            continue;
        }
        output.push_str(&segment.pronunciation);
        output.push_str(&segment.separator);
    }

    non_empty_ipa(Some(output.trim().to_string()))
}

fn katakana_name_to_tts_segments(input: &str) -> Option<Vec<TtsNameSegment>> {
    let pronunciation = katakana_name_to_ipa(input)?;
    Some(vec![TtsNameSegment {
        surface: input.to_string(),
        fallback_text: katakana_to_hiragana(input),
        pronunciation,
        alphabet: TtsAlphabetKind::Ipa,
        lang: "ja-JP",
        separator: String::new(),
    }])
}

fn should_split_camel_case_token(prev: Option<char>, current: char) -> bool {
    matches!(prev, Some(prev) if prev.is_ascii_lowercase() && current.is_ascii_uppercase())
}

fn romanized_name_to_tts_segments(input: &str) -> Option<Vec<TtsNameSegment>> {
    let mut tokens: Vec<String> = Vec::new();
    let mut token = String::new();
    let mut prev_token_char: Option<char> = None;

    for c in input.chars() {
        if is_name_token_char(c) {
            if should_split_camel_case_token(prev_token_char, c) {
                flush_name_token(&mut tokens, &mut token);
            }
            token.push(c);
            prev_token_char = Some(c);
            continue;
        }

        flush_name_token(&mut tokens, &mut token);
        prev_token_char = None;
    }

    flush_name_token(&mut tokens, &mut token);

    if tokens.is_empty() {
        return Some(vec![]);
    }

    let mut segments = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        let mut word_segments = word_to_tts_segments(token)?;
        if let Some(last) = word_segments.last_mut() {
            last.separator = if index + 1 < tokens.len() {
                " ".to_string()
            } else {
                String::new()
            };
        }
        segments.extend(word_segments);
    }

    Some(segments)
}

fn flush_name_token(tokens: &mut Vec<String>, token: &mut String) {
    if token.is_empty() {
        return;
    }

    tokens.push(token.clone());
    token.clear();
}

fn word_to_tts_segments(token: &str) -> Option<Vec<TtsNameSegment>> {
    let normalized = normalize_name_token(token);
    if normalized.is_empty() {
        return Some(vec![]);
    }

    if let Some(segments) = split_compound_token_to_tts_segments(token, &normalized) {
        return Some(segments);
    }

    if let Some(ipa) = lookup_english_word_ipa(&normalized) {
        return Some(vec![TtsNameSegment {
            surface: token.to_string(),
            fallback_text: token.to_string(),
            pronunciation: ipa.to_string(),
            alphabet: TtsAlphabetKind::Ipa,
            lang: "en-US",
            separator: String::new(),
        }]);
    }

    if normalized.chars().all(|c| c.is_ascii_digit()) {
        if let Some(ipa) = number_to_ipa(&normalized) {
            return Some(vec![TtsNameSegment {
                surface: token.to_string(),
                fallback_text: token.to_string(),
                pronunciation: ipa.to_string(),
                alphabet: TtsAlphabetKind::Ipa,
                lang: "en-US",
                separator: String::new(),
            }]);
        }

        let mut pronunciation = String::new();
        for digit in normalized.chars() {
            let ipa = number_to_ipa(&digit.to_string())?;
            pronunciation.push_str(ipa);
        }
        return Some(vec![TtsNameSegment {
            surface: token.to_string(),
            fallback_text: token.to_string(),
            pronunciation,
            alphabet: TtsAlphabetKind::Ipa,
            lang: "en-US",
            separator: String::new(),
        }]);
    }

    let katakana = romaji_to_katakana(&normalized)?;
    let pronunciation = katakana_to_ipa(&katakana)?;
    Some(vec![TtsNameSegment {
        surface: token.to_string(),
        fallback_text: katakana_to_hiragana(&katakana),
        pronunciation,
        alphabet: TtsAlphabetKind::Ipa,
        lang: "ja-JP",
        separator: String::new(),
    }])
}

fn split_compound_token_to_tts_segments(
    original: &str,
    normalized: &str,
) -> Option<Vec<TtsNameSegment>> {
    const JAPANESE_SUFFIXES: &[&str] = &["kaigan"];

    for suffix in JAPANESE_SUFFIXES {
        if normalized.len() <= suffix.len() || !normalized.ends_with(suffix) {
            continue;
        }

        let stem_char_count = normalized.chars().count() - suffix.chars().count();
        let stem_byte_offset = original
            .char_indices()
            .nth(stem_char_count)
            .map(|(index, _)| index)
            .unwrap_or(original.len());
        let stem = &original[..stem_byte_offset];
        let mut stem_segments = word_to_tts_segments(stem)?;
        let suffix_segments = word_to_tts_segments(suffix)?;
        if stem_segments.is_empty() || suffix_segments.is_empty() {
            return None;
        }
        if let Some(last) = stem_segments.last_mut() {
            last.separator = " ".to_string();
        }
        stem_segments.extend(suffix_segments);
        return Some(stem_segments);
    }

    None
}

fn katakana_to_hiragana(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            'ァ'..='ヶ' => char::from_u32(c as u32 - 0x60).unwrap_or(c),
            _ => c,
        })
        .collect()
}

fn is_name_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            '\'' | '.' | 'Ā' | 'Ī' | 'Ū' | 'Ē' | 'Ō' | 'ā' | 'ī' | 'ū' | 'ē' | 'ō'
        )
}

fn normalize_name_token(token: &str) -> String {
    token
        .trim_matches(|c: char| !is_name_token_char(c))
        .trim_end_matches('.')
        .chars()
        .flat_map(normalize_name_char)
        .collect::<String>()
        .to_lowercase()
}

fn normalize_name_char(c: char) -> Vec<char> {
    match c {
        'Ā' | 'ā' => vec!['a', 'a'],
        'Ī' | 'ī' => vec!['i', 'i'],
        'Ū' | 'ū' => vec!['u', 'u'],
        'Ē' | 'ē' => vec!['e', 'i'],
        'Ō' | 'ō' => vec!['o', 'u'],
        _ => vec![c],
    }
}

fn lookup_english_word_ipa(word: &str) -> Option<&'static str> {
    match word {
        "airport" => Some("ɛɚpɔɹt"),
        "and" => Some("ænd"),
        "art" => Some("ɑɹt"),
        "avenue" => Some("ævənuː"),
        "atomic" => Some("ətɑmɪk"),
        "beach" => Some("biːtʃ"),
        "beer" => Some("bɪɹ"),
        "big" => Some("bɪg"),
        "blue" => Some("bluː"),
        "branch" => Some("bɹæntʃ"),
        "bomb" => Some("bɑm"),
        "botanical" => Some("bətænɪkəl"),
        "building" => Some("bɪldɪŋ"),
        "business" => Some("bɪznəs"),
        "bus" => Some("bʌs"),
        "cable" => Some("keɪbəl"),
        "campus" => Some("kæmpəs"),
        "castle" => Some("kæsəl"),
        "center" | "centre" => Some("sɛntɚ"),
        "central" => Some("sɛntɹəl"),
        "city" => Some("sɪti"),
        "commuter" => Some("kəmjuːtɚ"),
        "conference" => Some("kɑnfɚəns"),
        "cruise" => Some("kɹuːz"),
        "cross" => Some("kɹɔs"),
        "district" => Some("dɪstɹɪkt"),
        "distribution" => Some("dɪstɹəbjuːʃən"),
        "direct" => Some("dɚɛkt"),
        "east" => Some("iːst"),
        "electric" => Some("ɪlɛktɹɪk"),
        "elementary" => Some("ɛləməntɛɹi"),
        "entrance" => Some("ɛntɹəns"),
        "evening" => Some("iːvnɪŋ"),
        "express" => Some("ɪkspɹɛs"),
        "family" => Some("fæməli"),
        "ferry" => Some("fɛɹi"),
        "flower" => Some("flaʊɚ"),
        "for" => Some("fɔɹ"),
        "from" => Some("fɹʌm"),
        "fruit" => Some("fɹuːt"),
        "garden" => Some("gɑɹdən"),
        "gardens" => Some("gɑɹdənz"),
        "gateway" => Some("geɪtweɪ"),
        "general" => Some("dʒɛnɚəl"),
        "golf" => Some("gɑlf"),
        "green" => Some("gɹiːn"),
        "ground" => Some("gɹaʊnd"),
        "gymnasium" => Some("dʒɪmneɪziəm"),
        "hall" => Some("hɔl"),
        "high" => Some("haɪ"),
        "hospital" => Some("hɑspɪtəl"),
        "industrial" => Some("ɪndʌstɹiəl"),
        "international" => Some("ɪntɚnæʃənəl"),
        "island" => Some("aɪlənd"),
        "isle" => Some("aɪl"),
        "japan" => Some("dʒəpæn"),
        "jr" => Some("dʒeɪ ɑɹ"),
        "junior" => Some("dʒuːnjɚ"),
        "keisei" => Some("keːseː"),
        "line" => Some("laɪn"),
        "link" => Some("lɪŋk"),
        "liner" => Some("laɪnɚ"),
        "lrt" => Some("ɛl ɑɹ tiː"),
        "limited" => Some("lɪmɪtɪd"),
        "local" => Some("loʊkəl"),
        "loop" => Some("luːp"),
        "main" => Some("meɪn"),
        "mae" => Some("mae"),
        "management" => Some("mænɪdʒmənt"),
        "marine" => Some("məɹiːn"),
        "medical" => Some("mɛdɪkəl"),
        "metro" => Some("mɛtɹoʊ"),
        "monorail" => Some("mɑnoʊɹeɪl"),
        "morning" => Some("mɔɹnɪŋ"),
        "museum" => Some("mjuːziəm"),
        "municipal" => Some("mjuːnɪsəpəl"),
        "new" => Some("nuː"),
        "north" => Some("nɔɹθ"),
        "or" => Some("ɔɹ"),
        "park" => Some("pɑɹk"),
        "peace" => Some("piːs"),
        "port" => Some("pɔɹt"),
        "pool" => Some("puːl"),
        "railway" => Some("ɹeɪlweɪ"),
        "rail" => Some("ɹeɪl"),
        "rapid" => Some("ɹæpɪd"),
        "red" => Some("ɹɛd"),
        "regional" => Some("ɹiːdʒənəl"),
        "relay" => Some("ɹiːleɪ"),
        "ropeway" => Some("ɹoʊpweɪ"),
        "route" => Some("ɹuːt"),
        "scenic" => Some("siːnɪk"),
        "saint" => Some("seɪnt"),
        "school" => Some("skuːl"),
        "science" => Some("saɪəns"),
        "section" => Some("sɛkʃən"),
        "seaside" => Some("siːsaɪd"),
        "semi" => Some("sɛmi"),
        "senior" => Some("siːnjɚ"),
        "shiyakusho" => Some("ɕijakɯɕo"),
        "sight" => Some("saɪt"),
        "site" => Some("saɪt"),
        "skiing" => Some("skiːɪŋ"),
        "skytree" => Some("skaɪtɹiː"),
        "soccer" => Some("sɑkɚ"),
        "south" => Some("saʊθ"),
        "space" => Some("speɪs"),
        "special" => Some("spɛʃəl"),
        "sports" => Some("spɔɹts"),
        "square" => Some("skwɛɚ"),
        "stadium" => Some("steɪdiəm"),
        "station" => Some("steɪʃən"),
        "streetcar" => Some("stɹiːtkɑɹ"),
        "subway" => Some("sʌbweɪ"),
        "service" => Some("sɝvɪs"),
        "shuttle" => Some("ʃʌtəl"),
        "sub" => Some("sʌb"),
        "sunrise" => Some("sʌnɹaɪz"),
        "super" => Some("suːpɚ"),
        "telecom" => Some("tɛləkɑm"),
        "teleport" => Some("tɛləpɔɹt"),
        "terminal" => Some("tɚmɪnəl"),
        "the" => Some("ðə"),
        "town" => Some("taʊn"),
        "to" => Some("tuː"),
        "trade" => Some("tɹeɪd"),
        "train" => Some("tɹeɪn"),
        "transit" => Some("tɹænsɪt"),
        "tramway" => Some("tɹæmweɪ"),
        "tram" => Some("tɹæm"),
        "transport" => Some("tɹænspɔɹt"),
        "university" => Some("juːnəvɚsəti"),
        "universal" => Some("juːnəvɚsəl"),
        "urban" => Some("ɝbən"),
        "village" => Some("vɪlɪdʒ"),
        "way" => Some("weɪ"),
        "west" => Some("wɛst"),
        "world" => Some("wɝld"),
        "yard" => Some("jɑɹd"),
        "railroad" => Some("ɹeɪlɹoʊd"),
        "access" => Some("æksɛs"),
        "excursion" => Some("ɪkskɝʒən"),
        "holiday" => Some("hɑlədeɪ"),
        "nonstop" => Some("nɑnstɑp"),
        "weekday" => Some("wiːkdeɪ"),
        "southern" => Some("sʌðɚn"),
        "sky" => Some("skaɪ"),
        "office" => Some("ɔfɪs"),
        "police" => Some("pəliːs"),
        "shrine" => Some("ʃɹaɪn"),
        "temple" => Some("tɛmpəl"),
        "prefectural" => Some("pɹifɛktʃɚəl"),
        "bridge" => Some("bɹɪdʒ"),
        "plaza" => Some("plɑːzə"),
        "canal" => Some("kənæl"),
        "hotel" => Some("hoʊtɛl"),
        "cathedral" => Some("kəθiːdɹəl"),
        "arts" => Some("ɑɹts"),
        "crafts" => Some("kɹæfts"),
        "theater" => Some("θiətɚ"),
        "abt" => Some("eɪ biː tiː"),
        "angelland" => Some("eɪndʒəllænd"),
        "arcade" => Some("ɑɹkeɪd"),
        "anoh" => Some("ano"),
        "astram" => Some("æstɹæm"),
        "balloon" => Some("bəluːn"),
        "boat" => Some("boʊt"),
        "bitchu" => Some("bit͡ɕɯ"),
        "bitchuu" => Some("bit͡ɕɯː"),
        "bosch" => Some("bɑʃ"),
        "car" => Some("kɑɹ"),
        "centerpool" => Some("sɛntɚpuːl"),
        "centralpark" => Some("sɛntɹəlpɑɹk"),
        "chinatown" => Some("tʃaɪnətaʊn"),
        "chikucenter" => Some("tʃikjuːsɛntɚ"),
        "civic" => Some("sɪvɪk"),
        "circuit" => Some("sɝkɪt"),
        "cosmosquare" => Some("kɑzmoʊskwɛɚ"),
        "dam" => Some("dæm"),
        "depot" => Some("diːpoʊ"),
        "dinostar" => Some("daɪnoʊstɑɹ"),
        "english" => Some("ɪŋglɪʃ"),
        "etchu" => Some("ett͡ɕɯ"),
        "etchuu" => Some("ett͡ɕɯː"),
        "esta" => Some("ɛstə"),
        "expo" => Some("ɛkspoʊ"),
        "galaxy" => Some("gæləksi"),
        "gorge" => Some("gɔɹdʒ"),
        "hatchobaba" => Some("hatt͡ɕoːbaba"),
        "hatchobori" => Some("hatt͡ɕoːboɾi"),
        "huis" => Some("haʊs"),
        "itchome" => Some("itt͡ɕoːme"),
        "ir" => Some("aɪ ɑɹ"),
        "j" => Some("dʒeɪ"),
        "juhatchome" => Some("dʑɯːhatt͡ɕoːme"),
        "kintestu" => Some("kintetsɯ"),
        "kutchan" => Some("kɯtt͡ɕan"),
        "linimo" => Some("linimo"),
        "minoh" => Some("minoː"),
        "newtown" => Some("njuːtaʊn"),
        "no.1" => Some("nʌmbɚ wʌn"),
        "no.6" => Some("nʌmbɚ sɪks"),
        "no.7" => Some("nʌmbɚ sɛvən"),
        "no.8" => Some("nʌmbɚ eɪt"),
        "peach" => Some("piːtʃ"),
        "retro" => Some("ɹɛtɹoʊ"),
        "rias" => Some("ɹiːəs"),
        "shim" => Some("ɕin"),
        "side" => Some("saɪd"),
        "skyliner" => Some("skaɪlaɪnɚ"),
        "skyrail" => Some("skaɪɹeɪl"),
        "sonic" => Some("sɑnɪk"),
        "saphir" => Some("sæfiɹ"),
        "spacia" => Some("speɪʃə"),
        "sta" => Some("steɪʃən"),
        "sunport" => Some("sʌnpɔɹt"),
        "th" => Some("tiː eɪtʃ"),
        "through" => Some("θɹuː"),
        "thunderbird" => Some("θʌndɚbɝd"),
        "tj" => Some("tiː dʒeɪ"),
        "wing" => Some("wɪŋ"),
        "woody" => Some("wʊdi"),
        "x" => Some("ɛks"),
        "aqua" => Some("ækwə"),
        "lavender" => Some("lævəndɚ"),
        "lilac" => Some("laɪlæk"),
        "okhotsk" => Some("oʊkhɑtsk"),
        "b" => Some("biː"),
        "crossbay" => Some("kɹɔsbeɪ"),
        "farm" => Some("fɑɹm"),
        "field" => Some("fiːld"),
        "gala" => Some("gɑːlə"),
        "girls" => Some("gɝlz"),
        "grand" => Some("gɹænd"),
        "highland" => Some("haɪlənd"),
        "hills" => Some("hɪlz"),
        "harmonyhall" => Some("hɑɹmənihɔl"),
        "harborland" => Some("hɑɹbɚlænd"),
        "heartpia" => Some("hɑɹtpiə"),
        "land" => Some("lænd"),
        "laketown" => Some("leɪktaʊn"),
        "mall" => Some("mɔl"),
        "mary's" => Some("mɛɹiz"),
        "mt" => Some("maʊnt"),
        "mt.takao" => Some("maʊnt taka.o"),
        "mt.fuji" => Some("maʊnt ɸɯdʑi"),
        "norfolk" => Some("nɔɹfoʊk"),
        "ohmi" => Some("oːmi"),
        "oarks" => Some("oʊks"),
        "paddy" => Some("pædi"),
        "pref" => Some("pɹɛf"),
        "costa" => Some("kɔstə"),
        "grandberry" => Some("gɹændbɛɹi"),
        "fujifilm" => Some("ɸɯdʑifɪɾɯm"),
        "fujitec" => Some("ɸɯdʑitek"),
        "intec" => Some("ɪntek"),
        "jatco" => Some("dʒætkoʊ"),
        "s" => Some("ɛs"),
        "t" => Some("tiː"),
        "trans" => Some("tɹæns"),
        "zoological" => Some("zuːəlɑdʒɪkəl"),
        _ => None,
    }
}

fn number_to_ipa(word: &str) -> Option<&'static str> {
    match word {
        "0" => Some("zɪɹoʊ"),
        "1" => Some("wʌn"),
        "2" => Some("tuː"),
        "3" => Some("θɹiː"),
        "4" => Some("fɔɹ"),
        "5" => Some("faɪv"),
        "6" => Some("sɪks"),
        "7" => Some("sɛvən"),
        "8" => Some("eɪt"),
        "9" => Some("naɪn"),
        _ => None,
    }
}

fn romaji_to_katakana(input: &str) -> Option<String> {
    if input.is_empty() {
        return Some(String::new());
    }

    let chars: Vec<char> = input.chars().collect();
    let mut out = String::new();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '\'' {
            i += 1;
            continue;
        }

        if i + 1 < chars.len()
            && chars[i] == chars[i + 1]
            && chars[i] != 'n'
            && is_romaji_consonant(chars[i])
        {
            out.push('ッ');
            i += 1;
            continue;
        }

        if chars[i] == 'n' || (chars[i] == 'm' && i + 1 < chars.len() && is_bilabial(chars[i + 1]))
        {
            if i + 1 == chars.len() {
                out.push('ン');
                i += 1;
                continue;
            }

            let next = chars[i + 1];
            if next == 'n' {
                out.push('ン');
                i += 1;
                continue;
            }

            if !is_romaji_vowel(next) && next != 'y' {
                out.push('ン');
                i += 1;
                continue;
            }
        }

        if let Some((kana, consumed)) = match_romaji_chunk(&chars[i..]) {
            out.push_str(kana);
            i += consumed;
            continue;
        }

        return None;
    }

    Some(out)
}

fn is_romaji_vowel(c: char) -> bool {
    matches!(c, 'a' | 'i' | 'u' | 'e' | 'o')
}

fn is_romaji_consonant(c: char) -> bool {
    c.is_ascii_alphabetic() && !is_romaji_vowel(c)
}

fn is_bilabial(c: char) -> bool {
    matches!(c, 'b' | 'p' | 'm')
}

fn match_romaji_chunk(chars: &[char]) -> Option<(&'static str, usize)> {
    const MAP: &[(&str, &str)] = &[
        ("ltsu", "ッ"),
        ("xtsu", "ッ"),
        ("kya", "キャ"),
        ("kyu", "キュ"),
        ("kyo", "キョ"),
        ("gya", "ギャ"),
        ("gyu", "ギュ"),
        ("gyo", "ギョ"),
        ("sha", "シャ"),
        ("shu", "シュ"),
        ("sho", "ショ"),
        ("sya", "シャ"),
        ("syu", "シュ"),
        ("syo", "ショ"),
        ("cha", "チャ"),
        ("chu", "チュ"),
        ("cho", "チョ"),
        ("tya", "チャ"),
        ("tyu", "チュ"),
        ("tyo", "チョ"),
        ("nya", "ニャ"),
        ("nyu", "ニュ"),
        ("nyo", "ニョ"),
        ("hya", "ヒャ"),
        ("hyu", "ヒュ"),
        ("hyo", "ヒョ"),
        ("mya", "ミャ"),
        ("myu", "ミュ"),
        ("myo", "ミョ"),
        ("rya", "リャ"),
        ("ryu", "リュ"),
        ("ryo", "リョ"),
        ("bya", "ビャ"),
        ("byu", "ビュ"),
        ("byo", "ビョ"),
        ("pya", "ピャ"),
        ("pyu", "ピュ"),
        ("pyo", "ピョ"),
        ("ja", "ジャ"),
        ("ju", "ジュ"),
        ("jo", "ジョ"),
        ("jya", "ジャ"),
        ("jyu", "ジュ"),
        ("jyo", "ジョ"),
        ("shi", "シ"),
        ("chi", "チ"),
        ("tsu", "ツ"),
        ("fu", "フ"),
        ("ji", "ジ"),
        ("ka", "カ"),
        ("ki", "キ"),
        ("ku", "ク"),
        ("ke", "ケ"),
        ("ko", "コ"),
        ("ga", "ガ"),
        ("gi", "ギ"),
        ("gu", "グ"),
        ("ge", "ゲ"),
        ("go", "ゴ"),
        ("sa", "サ"),
        ("su", "ス"),
        ("se", "セ"),
        ("so", "ソ"),
        ("za", "ザ"),
        ("zu", "ズ"),
        ("ze", "ゼ"),
        ("zo", "ゾ"),
        ("ta", "タ"),
        ("te", "テ"),
        ("to", "ト"),
        ("da", "ダ"),
        ("de", "デ"),
        ("do", "ド"),
        ("na", "ナ"),
        ("ni", "ニ"),
        ("nu", "ヌ"),
        ("ne", "ネ"),
        ("no", "ノ"),
        ("ha", "ハ"),
        ("hi", "ヒ"),
        ("he", "ヘ"),
        ("ho", "ホ"),
        ("ba", "バ"),
        ("bi", "ビ"),
        ("bu", "ブ"),
        ("be", "ベ"),
        ("bo", "ボ"),
        ("pa", "パ"),
        ("pi", "ピ"),
        ("pu", "プ"),
        ("pe", "ペ"),
        ("po", "ポ"),
        ("ma", "マ"),
        ("mi", "ミ"),
        ("mu", "ム"),
        ("me", "メ"),
        ("mo", "モ"),
        ("ya", "ヤ"),
        ("yu", "ユ"),
        ("yo", "ヨ"),
        ("ra", "ラ"),
        ("ri", "リ"),
        ("ru", "ル"),
        ("re", "レ"),
        ("ro", "ロ"),
        ("wa", "ワ"),
        ("wo", "ヲ"),
        ("va", "ヴァ"),
        ("vi", "ヴィ"),
        ("vu", "ヴ"),
        ("ve", "ヴェ"),
        ("vo", "ヴォ"),
        ("a", "ア"),
        ("i", "イ"),
        ("u", "ウ"),
        ("e", "エ"),
        ("o", "オ"),
    ];

    for (roman, kana) in MAP {
        if chars.len() < roman.len() {
            continue;
        }
        if chars.iter().take(roman.len()).copied().eq(roman.chars()) {
            return Some((*kana, roman.len()));
        }
    }

    None
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
        ('ギ', 'ャ') => "gʲa",
        ('ギ', 'ュ') => "gʲɯ",
        ('ギ', 'ョ') => "gʲo",
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
        'ガ' => "ga",
        'ギ' => "gi",
        'グ' => "gɯ",
        'ゲ' => "ge",
        'ゴ' => "go",
        // ザ行
        'ザ' => "za",
        'ジ' => "dʑi",
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
        // 区切り文字（全角・半角空白、中黒、括弧）は語の区切りとして空白に変換。
        // 「チュウオウ・ソウブセン」「トウキョウサクラトラム（トデンアラカワセン）」など、
        // これらを含む路線名で name_ipa 全体が None にならないようにする。
        '　' | ' ' | '・' | '（' | '）' | '(' | ')' => return Some(Phoneme::Regular(" ")),
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

/// Classify the place of articulation of the following phoneme for ン assimilation.
///
/// Azure ja-JP の `<phoneme alphabet="ipa">` は非ASCIIの鼻音 (口蓋垂 `ɴ` /
/// 口蓋 `ɲ` / 軟口蓋 `ŋ`) を音節化せず「ん」が脱落する (#1536)。そのため
/// 本来 `ɲ` / `ŋ` / `ɴ` に同化する位置もすべて ASCII の `n` に統一し、
/// 両唇音前の `m` だけを残す (`m` は ASCII で正しく発音される)。
fn nasal_for_following(next_ipa: &str) -> &'static str {
    if next_ipa.starts_with('b') || next_ipa.starts_with('p') || next_ipa.starts_with('m') {
        "m" // bilabial assimilation
    } else {
        "n"
    }
}

/// Determine the consonant character that a geminate (ッ) doubles in front of
/// the following phoneme. Returns `None` when nothing should be emitted
/// (no following phoneme, or the following phoneme starts with a vowel).
///
/// For affricates (t͡ɕ, t͡s), only the stop portion (t) is geminated; for the
/// voiced affricate dʑ the stop portion is d; for palatalized onsets (kʲ, gʲ,
/// etc.) only the base consonant is geminated.
fn geminate_onset_char(next_ipa: &str) -> Option<char> {
    if next_ipa.starts_with("t͡ɕ") || next_ipa.starts_with("t͡s") {
        Some('t')
    } else if next_ipa.starts_with("dʑ") {
        Some('d')
    } else {
        let (onset, _) = split_onset(next_ipa);
        if onset.is_empty() {
            None
        } else {
            strip_secondary_articulation(onset).chars().next()
        }
    }
}

/// Render the phoneme sequence into a raw IPA string, applying ン assimilation,
/// ッ gemination and long-vowel lengthening.
fn phonemes_to_raw_ipa(phonemes: &[Phoneme]) -> String {
    let mut output = String::new();
    let len = phonemes.len();
    let mut i = 0;

    while i < len {
        match &phonemes[i] {
            Phoneme::Regular(ipa) => {
                // この層は honest な IPA をそのまま出力する (ら行は弾き音 ɾ のまま)。
                // 出力は日本語音声で読む name_ipa にそのまま使う。英語音声で読む
                // name_roman_ipa / tts_segments では ɾ が英語のフラップ /t/ に化ける
                // ため、station_name_to_tts_segments の lateralize_ra_row でら行を l に
                // 置換する。区切り文字 (空白) もそのまま出力する。
                output.push_str(ipa);
            }
            Phoneme::MoraicNasal => {
                let next_regular = find_next_regular(&phonemes[i + 1..]);
                let nasal = match next_regular {
                    Some(next_ipa) => nasal_for_following(next_ipa),
                    None => "n", // word-final (Azure が ɴ を鳴らさないため n に統一, #1536)
                };
                output.push_str(nasal);
                // 母音・半母音 (ヤ行 j / ワ行 w) が続く撥音 (例: シンエゴタ,
                // シンヨコハマ) は、音節境界 `.` を挟まないと Azure が n+母音を
                // 「な行」等に融合させてしまう (しねごた)。`.` を付与して
                // 「しん.えごた」のように独立させる (#1536)。
                if next_regular.is_some_and(starts_with_vowel_or_semivowel) {
                    output.push('.');
                }
            }
            Phoneme::Geminate => {
                if let Some(next_ipa) = find_next_regular(&phonemes[i + 1..]) {
                    if let Some(c) = geminate_onset_char(next_ipa) {
                        output.push(c);
                    }
                }
            }
            Phoneme::LongVowel => {
                // Lengthen the preceding vowel unless it is already long.
                let already_long = output.ends_with('ː');
                if !already_long {
                    output.push('ː');
                }
            }
        }
        i += 1;
    }

    output
}

/// Apply phonological rules: ン assimilation, ッ gemination, long vowels.
fn apply_phonological_rules(phonemes: &[Phoneme]) -> String {
    insert_syllable_breaks(&phonemes_to_raw_ipa(phonemes))
}

/// Whether `c` is one of the IPA vowel characters used by this module
/// (`a i ɯ e o u`).
fn is_ipa_vowel(c: char) -> bool {
    "aiɯeou".contains(c)
}

/// Whether the phoneme starts with a vowel or a semivowel (ヤ行 `j` / ワ行 `w`).
/// Used to decide whether a moraic nasal needs a trailing syllable break so the
/// `n` stays a coda instead of merging into the following mora.
fn starts_with_vowel_or_semivowel(ipa: &str) -> bool {
    ipa.starts_with(is_ipa_vowel) || ipa.starts_with('j') || ipa.starts_with('w')
}

/// Whether a `prev`→`cur` vowel sequence forms a Japanese long vowel (長音) per
/// kana orthography:
///   ああ→aː  いい→iː  うう→ɯː  ええ/えい→eː  おお/おう→oː
/// `えい` (e→i) and `おう` (o→ɯ) are the orthographic long vowels written with a
/// trailing different kana, so they are included alongside the doubled vowels.
/// Other vowel sequences (ウエ, アイ, アオ, オイ …) are genuine vowel hiatus and
/// are NOT collapsed.
fn forms_long_vowel(prev: char, cur: char) -> bool {
    matches!(
        (prev, cur),
        ('a', 'a') | ('i', 'i') | ('ɯ', 'ɯ') | ('e', 'e') | ('e', 'i') | ('o', 'o') | ('o', 'ɯ')
    )
}

/// Insert IPA syllable boundary markers (`.`) between consecutive vowels.
/// This prevents Google TTS from interpreting cross-mora vowel sequences
/// (e.g. `ei` in セイ) as English diphthongs (e.g. /eɪ/ → "ai").
///
/// 連続母音が日本語の長音 (おう/おお/えい等, [`forms_long_vowel`]) を成す場合は
/// 音節境界 `.` ではなく長音記号 `ː` に置き換え、2 つ目の母音を伸ばす。Azure の
/// ja-JP `<phoneme alphabet="ipa">` では `to.ɯkʲo.ɯ` のような母音分割よりも
/// `toːkʲoː` の長音表記のほうが「とーきょー」と自然に読まれる。
fn insert_syllable_breaks(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    // 直前の母音の「基底」文字。長音化しても基底母音を保持し、続く母音と再判定する。
    let mut prev_vowel: Option<char> = None;
    // 直前の母音が (長音記号 ː で) 既に伸ばされているか。伸ばされた母音の直後に
    // さらに長音化を重ねない (二重 ː を防ぐ) ための状態。
    let mut prev_vowel_lengthened = false;

    for c in input.chars() {
        // 入力中に既に存在する長音記号 ː (カタカナ「ー」由来) は、直前母音の状態を
        // 維持したまま通す。これにより `エーアイ` → `eː.a.i` のように、長音直後に
        // 別母音が続く場合も音節境界 `.` を落とさない。
        if c == 'ː' {
            result.push(c);
            prev_vowel_lengthened = prev_vowel.is_some();
            continue;
        }

        if is_ipa_vowel(c) {
            if let Some(prev) = prev_vowel {
                if !prev_vowel_lengthened && forms_long_vowel(prev, c) {
                    // 2 つ目の母音は ː に置換して伸ばす。基底母音 (prev) は維持。
                    result.push('ː');
                    prev_vowel_lengthened = true;
                    continue;
                }
                result.push('.');
            }
            result.push(c);
            prev_vowel = Some(c);
            prev_vowel_lengthened = false;
        } else {
            result.push(c);
            prev_vowel = None;
            prev_vowel_lengthened = false;
        }
    }

    result
}

/// Find the IPA string of the next Regular phoneme in the slice.
fn find_next_regular(phonemes: &[Phoneme]) -> Option<&'static str> {
    phonemes.iter().find_map(|p| match p {
        Phoneme::Regular(ipa) => Some(*ipa),
        _ => None,
    })
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
        assert_eq!(ipa("シナガワ"), "ɕinagawa");
    }

    #[test]
    fn test_ueno() {
        assert_eq!(ipa("ウエノ"), "ɯ.eno");
    }

    #[test]
    fn test_ikebukuro() {
        assert_eq!(ipa("イケブクロ"), "ikebɯkɯɾo");
    }

    #[test]
    fn test_shinjuku() {
        // ン before ジュ → n (Azure が ɲ を鳴らさないため, #1536), ジュ → dʑɯ
        assert_eq!(ipa("シンジュク"), "ɕindʑɯkɯ");
    }

    #[test]
    fn test_seibu_shinjuku_line_nasals_are_ascii_n() {
        // 西武新宿線: 「新」(ン→ɲ) と「線」(語末ン→ɴ) の両方を n に統一し、
        // Azure で双方の「ん」が鳴るようにする (#1536)。
        assert_eq!(ipa("セイブシンジュクセン"), "seːbɯɕindʑɯkɯsen");
    }

    #[test]
    fn test_osaka() {
        // オオ → 長音 oː (Azure 向け長音統一)
        assert_eq!(ipa("オオサカ"), "oːsaka");
    }

    #[test]
    fn test_kyoto() {
        // キョウ → 長音 kʲoː (おう→oː)
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
        // ら行はそのまま ɾʲ、リョウ→長音 ɾʲoː
        assert_eq!(ipa("リョウゴク"), "ɾʲoːgokɯ");
    }

    #[test]
    fn test_shimbashi() {
        // ン before バ → m
        assert_eq!(ipa("シンバシ"), "ɕimbaɕi");
    }

    #[test]
    fn test_keisei() {
        // ケイ・セイ → 長音 keː seː (えい→eː)
        assert_eq!(ipa("ケイセイ"), "keːseː");
    }

    #[test]
    fn test_oshiage() {
        assert_eq!(ipa("オシアゲ"), "oɕi.age");
    }

    #[test]
    fn test_meitetsu() {
        // ツ is consistently t͡sɯ (affricate with tie bar)。メイ→長音 meː (えい→eː)
        assert_eq!(ipa("メイテツ"), "meːtet͡sɯ");
    }

    #[test]
    fn test_seibu() {
        // セイ → 長音 seː (えい→eː)
        assert_eq!(ipa("セイブ"), "seːbɯ");
    }

    #[test]
    fn test_toride() {
        assert_eq!(ipa("トリデ"), "toɾide");
    }

    #[test]
    fn test_fukiage() {
        assert_eq!(ipa("フキアゲ"), "ɸɯkʲi.age");
    }

    #[test]
    fn test_fuse() {
        assert_eq!(ipa("フセ"), "ɸɯse");
    }

    #[test]
    fn test_inagekaigan() {
        // ン at word end → n (Azure が ɴ を鳴らさないため, #1536)
        assert_eq!(ipa("イナゲカイガン"), "inageka.igan");
    }

    #[test]
    fn test_moraic_nasal_before_vowel_keeps_syllable_break() {
        // 母音前の撥音は n + 音節境界 `.` で独立させ、「な行」融合
        // (シンエゴタ→しねごた) や ɴ 脱落 (しえごた) を防ぐ (#1536)。
        assert_eq!(ipa("シンエゴタ"), "ɕin.egota");
        // 千円: セ-ン-エ-ン → sen.en
        assert_eq!(ipa("センエン"), "sen.en");
    }

    #[test]
    fn test_inage() {
        assert_eq!(ipa("イナゲ"), "inage");
    }

    #[test]
    fn test_kire_uriwari() {
        assert_eq!(ipa("キレウリワリ"), "kʲiɾe.ɯɾiwaɾi");
    }

    #[test]
    fn test_yao() {
        assert_eq!(ipa("ヤオ"), "ja.o");
    }

    #[test]
    fn test_mejiro() {
        assert_eq!(ipa("メジロ"), "medʑiɾo");
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
        // チョウ → 長音 t͡ɕoː (おう→oː)
        assert_eq!(ipa("イッチョウメ"), "itt͡ɕoːme");
    }

    #[test]
    fn test_sanchome() {
        // チョウ → 長音 t͡ɕoː (おう→oː)
        assert_eq!(ipa("サンチョウメ"), "sant͡ɕoːme");
    }

    #[test]
    fn test_koen() {
        // コウ → 長音 koː、エ は別母音なので音節境界を残す koː.en
        assert_eq!(ipa("コウエン"), "koː.en");
    }

    #[test]
    fn test_tokyo() {
        // トウ・キョウ → 長音 toː kʲoː (おう→oː)
        assert_eq!(ipa("トウキョウ"), "toːkʲoː");
    }

    #[test]
    fn test_nagoya() {
        assert_eq!(ipa("ナゴヤ"), "nagoja");
    }

    #[test]
    fn test_roppongi_word_initial_r_stays_flap() {
        // name_ipa 用の katakana_to_ipa は honest な ɾ を維持する (語頭も同じ)。
        // 英語読みトラックの l 置換は station_name_to_ipa 側で行う。
        assert_eq!(ipa("ロッポンギ"), "ɾoppongi");
    }

    #[test]
    fn test_word_initial_r_after_separator_stays_flap() {
        // 区切り直後 (= 後続語の語頭) のら行も katakana_to_ipa では ɾ のまま。
        assert_eq!(ipa("シン・リンカイ"), "ɕin ɾinka.i");
    }

    #[test]
    fn test_medial_r_stays_flap() {
        // 語中のら行も katakana_to_ipa では弾き音 ɾ のまま (toɾide / t͡sɯɾɯmi)。
        assert_eq!(ipa("トリデ"), "toɾide");
    }

    #[test]
    fn test_romanized_track_lateralizes_ra_row() {
        // 英語読みトラック (station_name_to_ipa = name_roman_ipa / tts_segments) は
        // 英語音声で読むため、ら行を l に置換する。語頭・語中・語末いずれも対象。
        assert_eq!(
            station_name_to_ipa("トリデ", Some("Toride")),
            Some("tolide".to_string())
        );
        // 一方 name_ipa 用の katakana_to_ipa は同じ語でも honest な ɾ を維持する。
        assert_eq!(katakana_to_ipa("トリデ"), Some("toɾide".to_string()));
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
        // ン before ヨ(j) → n + 音節境界。ɲ は Azure が鳴らさず、半母音 j の前は
        // 音節境界 `.` を挟まないと「な行」に融合するため (#1536)。
        assert_eq!(ipa("シンヨコハマ"), "ɕin.jokohama");
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
    fn test_macron_romaji_converges_with_katakana_long_vowel() {
        // マクロン付きローマ字 Ōsaka は ō→オウ と再構成されるが、長音統一により
        // オオサカ (カタカナ) と同じ oːsaka に収束する。以前は o.ɯsaka と
        // 誤った母音分割になっていた。
        assert_eq!(
            station_name_to_ipa("オオサカ", Some("Ōsaka")),
            Some("oːsaka".to_string())
        );
        // トウキョウ (おう長音) も toːkʲoː。
        assert_eq!(
            station_name_to_ipa("トウキョウ", Some("Tōkyō")),
            Some("toːkʲoː".to_string())
        );
    }

    #[test]
    fn test_station_name_ipa_uses_official_english_wording() {
        assert_eq!(
            station_name_to_ipa("カサイリンカイコウエン", Some("Kasai-Rinkai Park")),
            // 英語読みトラックなのでら行は l (Rinkai → linka.i)
            Some("kasa.i linka.i pɑɹk".to_string())
        );
    }

    #[test]
    fn test_station_name_ipa_supports_english_and_digits() {
        assert_eq!(
            station_name_to_ipa("ナリタクウコウ", Some("Narita Airport Terminal 1")),
            Some("nalita ɛɚpɔɹt tɚmɪnəl wʌn".to_string())
        );
    }

    #[test]
    fn test_station_name_ipa_supports_multi_digit_numbers() {
        assert_eq!(
            station_name_to_ipa("ハネダクウコウ", Some("Haneda Airport Terminal 10")),
            Some("haneda ɛɚpɔɹt tɚmɪnəl wʌnzɪɹoʊ".to_string())
        );
    }

    #[test]
    fn test_station_name_ipa_falls_back_to_katakana_when_roman_parse_fails() {
        assert_eq!(
            station_name_to_ipa("シブヤ", Some("???")),
            Some("ɕibɯja".to_string())
        );
    }

    #[test]
    fn test_station_name_ipa_supports_mixed_english_facility_words() {
        assert_eq!(
            station_name_to_ipa("トウキョウビッグサイト", Some("Tōkyō Big Sight")),
            Some("toːkʲoː bɪg saɪt".to_string())
        );
    }

    #[test]
    fn test_station_name_ipa_supports_common_line_words() {
        assert_eq!(
            station_name_to_ipa("ヤマノテセン", Some("Yamanote Line")),
            Some("jamanote laɪn".to_string())
        );
    }

    #[test]
    fn test_station_name_ipa_supports_bilabial_m_in_romaji() {
        assert_eq!(
            station_name_to_ipa("シンバシ", Some("Shimbashi")),
            Some("ɕimbaɕi".to_string())
        );
    }

    #[test]
    fn test_station_name_ipa_splits_compound_kaigan_suffix() {
        assert_eq!(
            station_name_to_ipa("イナゲカイガン", Some("Inagekaigan")),
            Some("inage ka.igan".to_string())
        );
    }

    #[test]
    fn test_station_name_ipa_splits_other_compound_kaigan_suffix() {
        assert_eq!(
            station_name_to_ipa("オオモリカイガン", Some("Omorikaigan")),
            Some("omoli ka.igan".to_string())
        );
    }

    #[test]
    fn test_station_name_ipa_supports_line_related_english_words() {
        assert_eq!(
            station_name_to_ipa("トウザイセン", Some("Municipal Subway Blue Line")),
            Some("mjuːnɪsəpəl sʌbweɪ bluː laɪn".to_string())
        );
    }

    #[test]
    fn test_station_name_ipa_supports_train_type_words() {
        assert_eq!(
            station_name_to_ipa("カイソク", Some("Commuter Rapid")),
            Some("kəmjuːtɚ ɹæpɪd".to_string())
        );
    }

    #[test]
    fn test_station_name_ipa_supports_spaced_romanized_names_from_csv() {
        assert_eq!(
            station_name_to_ipa("メイテツイチノミヤ", Some("Meitetsu Ichinomiya")),
            Some("meːtet͡sɯ it͡ɕinomija".to_string())
        );
    }

    #[test]
    fn test_station_name_ipa_supports_meitetsu_prefixed_station_names_from_csv() {
        let cases = [
            ("メイテツナゴヤ", "Meitetsu Nagoya", "meːtet͡sɯ nagoja"),
            (
                "メイテツイチノミヤ",
                "Meitetsu Ichinomiya",
                "meːtet͡sɯ it͡ɕinomija",
            ),
            ("メイテツギフ", "Meitetsu Gifu", "meːtet͡sɯ giɸɯ"),
        ];

        for (katakana, roman, expected) in cases {
            assert_eq!(
                station_name_to_ipa(katakana, Some(roman)),
                Some(expected.to_string()),
                "failed for {roman}"
            );
        }
    }

    #[test]
    fn test_nakaguro_treated_as_word_separator() {
        // 中黒「・」は語の区切りとして空白に変換し、全体が None にならないようにする
        assert_eq!(ipa("チュウオウ・ソウブ"), "t͡ɕɯː.oː soːbɯ");
    }

    #[test]
    fn test_fullwidth_parentheses_treated_as_word_separator() {
        // 全角括弧「（）」も空白として扱い、先頭・末尾・連続の空白は正規化される
        assert_eq!(ipa("トラム（トデン）"), "toɾamɯ toden");
    }

    #[test]
    fn test_dokkyo_daigakumae_soka_matsubara() {
        // Full-width space between words should be preserved
        assert_eq!(
            ipa("ドッキョウダイガクマエ　ソウカマツバラ"),
            "dokkʲoːda.igakɯma.e soːkamat͡sɯbaɾa"
        );
    }

    #[test]
    fn test_dokkyo_daigakumae_soka_matsubara_halfwidth() {
        // Half-width (ASCII) space between words should also be accepted
        assert_eq!(
            ipa("ドッキョウダイガクマエ ソウカマツバラ"),
            "dokkʲoːda.igakɯma.e soːkamat͡sɯbaɾa"
        );
    }

    #[test]
    fn test_long_vowel_mark_before_distinct_vowel_keeps_break() {
        // 長音「ー」由来の ː の直後に別母音が続く場合も、母音境界 `.` を落とさない。
        // エ-ー-ア-イ → eː.a.i (eːa.i のように境界が消えないこと)。
        assert_eq!(ipa("エーアイ"), "eː.a.i");
        // 長音の直後が子音の場合は余計な境界を入れない。
        assert_eq!(ipa("コーヒー"), "koːçiː");
    }
}
