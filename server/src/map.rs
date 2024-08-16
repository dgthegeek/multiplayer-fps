use rand::Rng;
use serde::{Serialize, Deserialize};

pub const MAP_WIDTH: usize = 20;
pub const MAP_HEIGHT: usize = 20;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Map {
    cells: Vec<Vec<bool>>, // true pour un mur, false pour un espace vide
}
impl Map {
    pub fn new(difficulty: u8) -> Self {
        let mut rng = rand::thread_rng();
        let mut cells = vec![vec![false; MAP_WIDTH]; MAP_HEIGHT];
        
        let (wall_chance, dead_end_chance) = match difficulty {
            1 => (0.1, 0.05),
            2 => (0.2, 0.1),
            3 => (0.3, 0.15),
            _ => (0.2, 0.1),
        };
        
        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                if x == 0 || y == 0 || x == MAP_WIDTH - 1 || y == MAP_HEIGHT - 1 {
                    cells[y][x] = true;
                } else if rng.gen::<f32>() < wall_chance {
                    cells[y][x] = true;
                } else if rng.gen::<f32>() < dead_end_chance {
                    let directions = [(0, 1), (1, 0), (0, -1), (-1, 0)];
                    for _ in 0..3 {
                        let (dx, dy) = directions[rng.gen_range(0..4)];
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx >= 0 && nx < MAP_WIDTH as i32 && ny >= 0 && ny < MAP_HEIGHT as i32 {
                            cells[ny as usize][nx as usize] = true;
                        }
                    }
                }
            }
        }
        
        Map { cells }
    }
    fn is_wall(&self, x: usize, y: usize) -> bool {
        self.cells[y][x]
    }
}

pub fn is_valid_move(map: &Map, x: f32, y: f32) -> bool {
    let cell_x = x.floor() as usize;
    let cell_y = y.floor() as usize;
    cell_x < MAP_WIDTH && cell_y < MAP_HEIGHT && !map.is_wall(cell_x, cell_y)
}