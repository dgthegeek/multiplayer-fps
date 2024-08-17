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

        // Générer uniquement des murs longs horizontaux et verticaux
        let num_walls = match difficulty {
            1 => 7,
            2 => 10,
            3 => 15,
            _ => 10,
        };

        for _ in 0..num_walls {
            if rng.gen_bool(0.5) {
                // Mur horizontal long
                let y = rng.gen_range(3..MAP_HEIGHT-3);
                for x in 1..MAP_WIDTH-1 {
                    cells[y][x] = true;
                }
            } else {
                // Mur vertical long
                let x = rng.gen_range(3..MAP_WIDTH-3);
                for y in 1..MAP_HEIGHT-1 {
                    cells[y][x] = true;
                }
            }
        }

        // Ajouter des brèches larges dans les murs
        let num_breaches = num_walls * 2;  // Augmenté pour plus de passages
        for _ in 0..num_breaches {
            let x = rng.gen_range(3..MAP_WIDTH-3);
            let y = rng.gen_range(3..MAP_HEIGHT-3);
            
            // Créer une brèche large
            for dx in -3..=3 {  // Augmenté à 7x7 pour des brèches plus larges
                for dy in -3..=3 {
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

const PLAYER_SIZE: f32 = 0.5; // Ajustez cette valeur selon la taille de votre joueur
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