use serde::{Deserialize, Serialize};

use super::{line::Line, train_type::TrainType};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Route {
    pub train_type: Option<TrainType>,
    pub lines: Vec<Line>,
}

impl Route {
    pub fn new(train_type: Option<TrainType>, lines: Vec<Line>) -> Self {
        Self { train_type, lines }
    }
}
