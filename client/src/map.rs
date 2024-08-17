use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Map {
    pub cells: Vec<Vec<bool>>,
}

impl Map {
    pub fn is_wall(&self, x: usize, y: usize) -> bool {
        self.cells[y][x]
    }
}

pub fn is_valid_move(map: &Map, x: f32, y: f32) -> bool {
    let cell_x = x.floor() as usize;
    let cell_y = y.floor() as usize;
    cell_x < map.cells[0].len() && cell_y < map.cells.len() && !map.is_wall(cell_x, cell_y)
}