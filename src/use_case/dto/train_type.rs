use crate::{
    domain::entity::train_type::TrainType,
    pb::{TrainDirection, TrainType as GrpcTrainType},
};

impl From<TrainType> for GrpcTrainType {
    fn from(train_type: TrainType) -> Self {
        Self {
            id: train_type.id.to_owned(),
            type_id: train_type.type_cd.to_owned(),
            group_id: train_type.group_cd.to_owned(),
            name: train_type.name.to_string(),
            name_katakana: train_type.name_k.to_string(),
            name_roman: train_type.name_r.to_string(),
            name_chinese: train_type.name_zh.to_string(),
            name_korean: train_type.name_ko.to_string(),
            color: train_type.color.to_string(),
            stations: vec![],
            lines: vec![],
            another_train_types: vec![],
            direction: TrainDirection::Both.into(),
        }
    }
}
