use crate::{domain::entity::train_type::TrainType, station_api::TrainType as GrpcTrainType};

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
        Self {
            id,
            type_id: type_cd.unwrap_or_default(),
            group_id: line_group_cd.unwrap_or_default(),
            name: type_name.unwrap_or_default(),
            name_katakana: type_name_k.unwrap_or_default(),
            name_roman: type_name_r,
            name_chinese: type_name_zh,
            name_korean: type_name_ko,
            color: color.unwrap_or_default(),
            line: line.map(|line| Box::new((*line).into())),
            lines: lines.into_iter().map(|line| line.into()).collect(),
            direction: direction.unwrap_or_default() as i32,
            kind: kind as i32,
        }
    }
}
