use crate::{
    domain::{entity::train_type::TrainType, ipa::compute_ipa_cached},
    proto::TrainType as GrpcTrainType,
    use_case::dto::tts::to_proto_tts_segments,
};

impl From<TrainType> for GrpcTrainType {
    fn from(train_type: TrainType) -> Self {
        let TrainType {
            id,
            station_cd: _,
            type_cd,
            line_group_cd,
            pass: _,
            type_name,
            type_name_k,
            type_name_r,
            type_name_zh,
            type_name_ko,
            color,
            direction,
            line,
            lines,
            kind,
        } = train_type;
        let ipa = compute_ipa_cached(&type_name_k, type_name_r.as_deref());
        let name_ipa = ipa.name_ipa;
        let name_roman_ipa = ipa.name_roman_ipa;
        let name_tts_segments = to_proto_tts_segments(ipa.tts_segments);
        Self {
            id: id.map(|id| id as u32).unwrap_or(0),
            type_id: type_cd.map(|id| id as u32).unwrap_or(0),
            group_id: line_group_cd.map(|id| id as u32).unwrap_or(0),
            name: type_name,
            name_katakana: type_name_k,
            name_roman: type_name_r,
            name_chinese: type_name_zh,
            name_korean: type_name_ko,
            color,
            line: line.map(|line| Box::new((*line).into())),
            lines: lines.into_iter().map(|line| line.into()).collect(),
            direction: direction.unwrap_or(0),
            kind: kind.unwrap_or(0),
            name_ipa,
            name_roman_ipa,
            name_tts_segments,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_train_type() -> TrainType {
        TrainType::new(
            Some(1),
            Some(1130201),
            Some(1001),
            Some(1001),
            Some(1),
            "快速".to_string(),
            "カイソク".to_string(),
            Some("Rapid".to_string()),
            Some("快速".to_string()),
            Some("쾌속".to_string()),
            "#ff6600".to_string(),
            Some(0),
            Some(1),
        )
    }

    #[test]
    fn test_train_type_sets_katakana_and_roman_ipa() {
        let grpc_train_type: GrpcTrainType = create_test_train_type().into();

        assert_eq!(grpc_train_type.name_ipa, Some("ka.isokɯ".to_string()));
        assert_eq!(grpc_train_type.name_roman_ipa, Some("ɹæpɪd".to_string()));
    }
}
