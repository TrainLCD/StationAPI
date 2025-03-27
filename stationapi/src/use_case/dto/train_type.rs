use crate::{domain::entity::train_type::TrainType, proto::TrainType as GrpcTrainType};

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
            direction: direction.map(|direction| direction as i32).unwrap_or(0),
            kind: kind.map(|kind| kind as i32).unwrap_or(0),
        }
    }
}
