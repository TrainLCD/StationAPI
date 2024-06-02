use serde::{Deserialize, Serialize};

use super::{station::Station, train_type::TrainType};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Route {
    pub train_type: Option<TrainType>,
    pub stops: Vec<Station>,
}

impl Route {
    pub fn new(train_type: Option<TrainType>, stops: Vec<Station>) -> Self {
        Self { train_type, stops }
    }
}
