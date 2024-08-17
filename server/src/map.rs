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

        // Créer les murs extérieurs
        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                if x == 0 || y == 0 || x == MAP_WIDTH - 1 || y == MAP_HEIGHT - 1 {
                    cells[y][x] = true;
                }
            }
        }

        // Générer des murs longs horizontaux et verticaux
        let num_walls = match difficulty {
            1 => 5,
            2 => 8,
            3 => 12,
            _ => 8,
        };

        for _ in 0..num_walls {
            if rng.gen_bool(0.5) {
                // Mur horizontal
                let y = rng.gen_range(2..MAP_HEIGHT-2);
                let start = rng.gen_range(2..MAP_WIDTH/3);
                let end = rng.gen_range(2*MAP_WIDTH/3..MAP_WIDTH-2);
                for x in start..end {
                    cells[y][x] = true;
                }
            } else {
                // Mur vertical
                let x = rng.gen_range(2..MAP_WIDTH-2);
                let start = rng.gen_range(2..MAP_HEIGHT/3);
                let end = rng.gen_range(2*MAP_HEIGHT/3..MAP_HEIGHT-2);
                for y in start..end {
                    cells[y][x] = true;
                }
            }
        }

        // Ajouter des ouvertures plus larges dans les murs pour créer des passages
        let num_openings = num_walls * 3;
        for _ in 0..num_openings {
            let x = rng.gen_range(2..MAP_WIDTH-2);
            let y = rng.gen_range(2..MAP_HEIGHT-2);
            
            // Créer une ouverture plus large
            for dx in -1..=1 {
                for dy in -1..=1 {
                    let nx = (x as i32 + dx) as usize;
                    let ny = (y as i32 + dy) as usize;
                    if nx > 0 && nx < MAP_WIDTH-1 && ny > 0 && ny < MAP_HEIGHT-1 {
                        cells[ny][nx] = false;
                    }
                }
            }
        }

        Map { cells }
    }

    fn is_wall(&self, x: usize, y: usize) -> bool {
        self.cells[y][x]
    }

    pub fn generate_valid_spawn_point(&self) -> (f32, f32) {
        let mut rng = rand::thread_rng();
        loop {
            let x = rng.gen_range(1..MAP_WIDTH - 1) as f32 + 0.5;
            let y = rng.gen_range(1..MAP_HEIGHT - 1) as f32 + 0.5;
            if is_valid_move(self, x, y) {
                return (x, y);
            }
        }
    }
    
}

const PLAYER_SIZE: f32 = 0.7; // Ajustez cette valeur selon la taille de votre joueur
pub fn is_valid_move(map: &Map, x: f32, y: f32) -> bool {
    let check_point = |px: f32, py: f32| -> bool {
        let cell_x = px.floor() as usize;
        let cell_y = py.floor() as usize;
        cell_x < MAP_WIDTH && cell_y < MAP_HEIGHT && !map.is_wall(cell_x, cell_y)
    };

    let half_size = PLAYER_SIZE / 2.0;

    check_point(x - half_size, y - half_size) && // Coin supérieur gauche
    check_point(x + half_size, y - half_size) && // Coin supérieur droit
    check_point(x - half_size, y + half_size) && // Coin inférieur gauche
    check_point(x + half_size, y + half_size)    // Coin inférieur droit
}