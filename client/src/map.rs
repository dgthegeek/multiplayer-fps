use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Map {
    pub cells: Vec<Vec<bool>>,
}
