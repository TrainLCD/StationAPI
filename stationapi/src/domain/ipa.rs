/// Katakana line-name suffixes paired with their English IPA replacements.
/// Ordered longest-first for greedy matching.
const LINE_NAME_SUFFIX_MAP: &[(&str, &str)] = &[
    ("ホンセン", " meɪn laɪn"),
    ("シセン", " laɪn"),
    ("セン", " laɪn"),
];
/// Suffixes that should NOT be replaced even though they end with セン.
const LINE_NAME_SUFFIX_EXCEPTIONS: &[&str] = &["シンカンセン"];

/// Replace a common line-name suffix (線/本線/支線) in a katakana string
/// with its English IPA equivalent (Line / Main Line).
/// 新幹線 (Shinkansen) is preserved as it is used as-is in English.
/// Returns the stem and the English IPA suffix to append.
/// If no known suffix is found, returns the full input with an empty suffix.
pub fn replace_line_name_suffix(input: &str) -> (&str, &str) {
    for exception in LINE_NAME_SUFFIX_EXCEPTIONS {
        if input.ends_with(exception) {
            return (input, "");
        }
    }
    for (suffix, replacement) in LINE_NAME_SUFFIX_MAP {
        if let Some(stem) = input.strip_suffix(suffix) {
            if !stem.is_empty() {
                return (stem, replacement);
            }
        }
    }
    (input, "")
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

/// Convert a station name to IPA.
/// Prefers the official romanized/English name when present so mixed names like
/// "Kasai-Rinkai Park" use English pronunciation for translated segments.
pub fn station_name_to_ipa(name_katakana: &str, name_roman: Option<&str>) -> Option<String> {
    name_roman
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .and_then(romanized_name_to_ipa)
        .filter(|ipa| !ipa.is_empty())
        .or_else(|| katakana_to_ipa(name_katakana))
        .filter(|ipa| !ipa.is_empty())
}

fn romanized_name_to_ipa(input: &str) -> Option<String> {
    let mut output = String::new();
    let mut token = String::new();
    let mut emitted_word = false;

    for c in input.chars() {
        if is_name_token_char(c) {
            token.push(c);
            continue;
        }

        flush_name_token(&mut output, &mut token, &mut emitted_word)?;

        if is_separator_like(c) && emitted_word && !output.ends_with(' ') {
            output.push(' ');
        }
    }

    flush_name_token(&mut output, &mut token, &mut emitted_word)?;

    Some(output.trim().to_string())
}

fn flush_name_token(
    output: &mut String,
    token: &mut String,
    emitted_word: &mut bool,
) -> Option<()> {
    if token.is_empty() {
        return Some(());
    }

    let ipa = word_to_ipa(token)?;
    if *emitted_word && !output.ends_with(' ') {
        output.push(' ');
    }
    output.push_str(&ipa);
    *emitted_word = true;
    token.clear();
    Some(())
}

fn word_to_ipa(token: &str) -> Option<String> {
    let normalized = normalize_name_token(token);
    if normalized.is_empty() {
        return Some(String::new());
    }

    if let Some(ipa) = split_compound_token_to_ipa(&normalized) {
        return Some(ipa);
    }

    if let Some(ipa) = lookup_english_word_ipa(&normalized) {
        return Some(ipa.to_string());
    }

    if normalized.chars().all(|c| c.is_ascii_digit()) {
        if let Some(ipa) = number_to_ipa(&normalized) {
            return Some(ipa.to_string());
        }

        let mut output = String::new();
        for digit in normalized.chars() {
            let ipa = number_to_ipa(&digit.to_string())?;
            output.push_str(ipa);
        }
        return Some(output);
    }

    romaji_to_katakana(&normalized).and_then(|katakana| katakana_to_ipa(&katakana))
}

fn split_compound_token_to_ipa(token: &str) -> Option<String> {
    const JAPANESE_SUFFIXES: &[&str] = &["kaigan"];

    for suffix in JAPANESE_SUFFIXES {
        if token.len() <= suffix.len() || !token.ends_with(suffix) {
            continue;
        }

        let stem = &token[..token.len() - suffix.len()];
        let stem_ipa = word_to_ipa(stem)?;
        let suffix_ipa = word_to_ipa(suffix)?;
        if stem_ipa.is_empty() || suffix_ipa.is_empty() {
            return None;
        }
        return Some(format!("{stem_ipa} {suffix_ipa}"));
    }

    None
}

fn is_name_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            '\'' | '.' | 'Ā' | 'Ī' | 'Ū' | 'Ē' | 'Ō' | 'ā' | 'ī' | 'ū' | 'ē' | 'ō'
        )
}

fn is_separator_like(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            '-' | '‐'
                | '‑'
                | '‒'
                | '–'
                | '—'
                | '―'
                | '/'
                | '･'
                | '・'
                | '·'
                | '('
                | ')'
                | '（'
                | '）'
                | ','
                | '、'
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
        "kutchan" => Some("kɯtt͡ɕaɴ"),
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
        "shim" => Some("ɕiɴ"),
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
        || next_ipa.starts_with("gʲ")
        || next_ipa.starts_with("kʲ")
        || next_ipa.starts_with('j')
        || next_ipa.starts_with('ç')
    {
        "ɲ" // palatal assimilation
    } else if next_ipa.starts_with('k') || next_ipa.starts_with('g') || next_ipa.starts_with('ŋ') {
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
                // For palatalized onsets (kʲ, gʲ, etc.), only the base consonant is geminated.
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

    insert_syllable_breaks(&output)
}

/// Insert IPA syllable boundary markers (`.`) between consecutive vowels.
/// This prevents Google TTS from interpreting cross-mora vowel sequences
/// (e.g. `ei` in セイ) as English diphthongs (e.g. /eɪ/ → "ai").
fn insert_syllable_breaks(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut prev_is_vowel = false;

    for c in input.chars() {
        let is_vowel = "aiɯeou".contains(c);
        if is_vowel && prev_is_vowel {
            result.push('.');
        }
        result.push(c);
        prev_is_vowel = is_vowel;
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
        // ン before ジュ → ɲ, ジュ → dʑɯ
        assert_eq!(ipa("シンジュク"), "ɕiɲdʑɯkɯ");
    }

    #[test]
    fn test_osaka() {
        assert_eq!(ipa("オオサカ"), "o.osaka");
    }

    #[test]
    fn test_kyoto() {
        assert_eq!(ipa("キョウト"), "kʲo.ɯto");
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
        assert_eq!(ipa("リョウゴク"), "ɾʲo.ɯgokɯ");
    }

    #[test]
    fn test_shimbashi() {
        // ン before バ → m
        assert_eq!(ipa("シンバシ"), "ɕimbaɕi");
    }

    #[test]
    fn test_keisei() {
        assert_eq!(ipa("ケイセイ"), "ke.ise.i");
    }

    #[test]
    fn test_oshiage() {
        assert_eq!(ipa("オシアゲ"), "oɕi.age");
    }

    #[test]
    fn test_meitetsu() {
        // ツ is consistently t͡sɯ (affricate with tie bar)
        assert_eq!(ipa("メイテツ"), "me.itet͡sɯ");
    }

    #[test]
    fn test_seibu() {
        assert_eq!(ipa("セイブ"), "se.ibɯ");
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
        // ン at word end → ɴ
        assert_eq!(ipa("イナゲカイガン"), "inageka.igaɴ");
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
        assert_eq!(ipa("イッチョウメ"), "itt͡ɕo.ɯme");
    }

    #[test]
    fn test_sanchome() {
        assert_eq!(ipa("サンチョウメ"), "sant͡ɕo.ɯme");
    }

    #[test]
    fn test_koen() {
        assert_eq!(ipa("コウエン"), "ko.ɯ.eɴ");
    }

    #[test]
    fn test_tokyo() {
        assert_eq!(ipa("トウキョウ"), "to.ɯkʲo.ɯ");
    }

    #[test]
    fn test_nagoya() {
        assert_eq!(ipa("ナゴヤ"), "nagoja");
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
    fn test_empty() {
        assert_eq!(katakana_to_ipa(""), Some(String::new()));
    }

    #[test]
    fn test_unknown_characters_returns_none() {
        assert_eq!(katakana_to_ipa("ABC"), None);
        assert_eq!(katakana_to_ipa("シブヤX"), None);
    }

    #[test]
    fn test_station_name_ipa_uses_official_english_wording() {
        assert_eq!(
            station_name_to_ipa("カサイリンカイコウエン", Some("Kasai-Rinkai Park")),
            Some("kasa.i ɾiŋka.i pɑɹk".to_string())
        );
    }

    #[test]
    fn test_station_name_ipa_supports_english_and_digits() {
        assert_eq!(
            station_name_to_ipa("ナリタクウコウ", Some("Narita Airport Terminal 1")),
            Some("naɾita ɛɚpɔɹt tɚmɪnəl wʌn".to_string())
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
            Some("to.ɯkʲo.ɯ bɪg saɪt".to_string())
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
            Some("inage ka.igaɴ".to_string())
        );
    }

    #[test]
    fn test_station_name_ipa_splits_other_compound_kaigan_suffix() {
        assert_eq!(
            station_name_to_ipa("オオモリカイガン", Some("Omorikaigan")),
            Some("omoɾi ka.igaɴ".to_string())
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
    fn test_dokkyo_daigakumae_soka_matsubara() {
        // Full-width space between words should be preserved
        assert_eq!(
            ipa("ドッキョウダイガクマエ　ソウカマツバラ"),
            "dokkʲo.ɯda.igakɯma.e so.ɯkamat͡sɯbaɾa"
        );
    }

    #[test]
    fn test_dokkyo_daigakumae_soka_matsubara_halfwidth() {
        // Half-width (ASCII) space between words should also be accepted
        assert_eq!(
            ipa("ドッキョウダイガクマエ ソウカマツバラ"),
            "dokkʲo.ɯda.igakɯma.e so.ɯkamat͡sɯbaɾa"
        );
    }

    // ============================================
    // replace_line_name_suffix tests
    // ============================================

    #[test]
    fn test_replace_sen() {
        assert_eq!(
            replace_line_name_suffix("セイブイケブクロセン"),
            ("セイブイケブクロ", " laɪn")
        );
    }

    #[test]
    fn test_replace_honsen() {
        assert_eq!(
            replace_line_name_suffix("トウカイドウホンセン"),
            ("トウカイドウ", " meɪn laɪn")
        );
    }

    #[test]
    fn test_replace_shinkansen_preserved() {
        // 新幹線(Shinkansen)は英語でもそのまま使われるので除去しない
        assert_eq!(
            replace_line_name_suffix("トウホクシンカンセン"),
            ("トウホクシンカンセン", "")
        );
    }

    #[test]
    fn test_replace_shisen() {
        assert_eq!(
            replace_line_name_suffix("ナガノハラクサツグチシセン"),
            ("ナガノハラクサツグチ", " laɪn")
        );
    }

    #[test]
    fn test_replace_no_suffix() {
        // ライン等セン以外の末尾はそのまま返す
        assert_eq!(
            replace_line_name_suffix("ショウナンシンジュクライン"),
            ("ショウナンシンジュクライン", "")
        );
    }

    #[test]
    fn test_replace_bare_sen_returns_unchanged() {
        // "セン" だけの場合、stemが空になるので除去しない
        assert_eq!(replace_line_name_suffix("セン"), ("セン", ""));
    }
}
