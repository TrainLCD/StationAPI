use crate::{domain::entity::train_type::TrainType, pb::TrainType as GrpcTrainType};

impl From<TrainType> for GrpcTrainType {
    fn from(train_type: TrainType) -> Self {
        let TrainType {
            id,
            station_cd: _,
            type_cd,
            line_group_cd,
            pass: _a,
            type_name,
            type_name_k,
            type_name_r,
            type_name_zh,
            type_name_ko,
            color,
            direction,
            line,
            lines,
        } = train_type;
        Self {
            id,
            type_id: type_cd,
            group_id: line_group_cd,
            name: type_name,
            name_katakana: type_name_k,
            name_roman: type_name_r,
            name_chinese: type_name_zh,
            name_korean: type_name_ko,
            color,
            line: line.map(|line| line.into()),
            lines: lines.into_iter().map(|line| line.into()).collect(),
            direction: direction as i32,
        }
    }
}
