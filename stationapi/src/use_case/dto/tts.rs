use crate::{
    domain::ipa::{TtsAlphabetKind, TtsNameSegment},
    proto::{TtsAlphabet, TtsSegment},
};

pub fn to_proto_tts_segments(segments: Vec<TtsNameSegment>) -> Vec<TtsSegment> {
    segments
        .into_iter()
        .map(|segment| TtsSegment {
            surface: segment.surface,
            fallback_text: segment.fallback_text,
            pronunciation: segment.pronunciation,
            alphabet: match segment.alphabet {
                TtsAlphabetKind::Ipa => TtsAlphabet::Ipa as i32,
                TtsAlphabetKind::Yomigana => TtsAlphabet::Yomigana as i32,
                TtsAlphabetKind::Plain => TtsAlphabet::Plain as i32,
            },
            lang: segment.lang.to_string(),
            separator: segment.separator,
        })
        .collect()
}
