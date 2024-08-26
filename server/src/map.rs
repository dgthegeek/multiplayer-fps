use rand::Rng;
use serde::{Serialize, Deserialize};

pub const MAP_WIDTH: usize = 25;
pub const MAP_HEIGHT: usize = 25;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Map {
    cells: Vec<Vec<bool>>, // true pour un mur, false pour un espace vide
    internal_wall_count: usize,
    map_width: usize,
    map_height: usize, 
}
impl Map {
    pub fn new(difficulty: u8) -> Self {
        let mut rng = rand::thread_rng();
        let mut cells = vec![vec![false; MAP_WIDTH]; MAP_HEIGHT];

        // Créer les murs extérieurs (clôtures)
        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                if x == 0 || y == 0 || x == MAP_WIDTH - 1 || y == MAP_HEIGHT - 1 {
                    cells[y][x] = true;
                }
            }
        }

        // Générer des murs intérieurs
        let num_walls = match difficulty {
            1 => 3,
            2 => 5,
            3 => 7,
            _ => 5,
        };   

        let min_wall_length = 3;
        let max_wall_length = 6;
        let wall_margin = 3;
        let wall_spacing = 8;

        for _ in 0..num_walls {
            let mut attempts = 0;
            'placement: while attempts < 100 {
                attempts += 1;
                let is_horizontal = rng.gen_bool(0.5);
                let length = rng.gen_range(min_wall_length..=max_wall_length);

                if is_horizontal {
                    let y = rng.gen_range(wall_margin..MAP_HEIGHT - wall_margin);
                    let start_x = rng.gen_range(wall_margin..MAP_WIDTH - wall_margin - length);
                    
                    // Vérifier si l'emplacement est libre
                    if !is_area_clear(&cells, start_x, y, length, true, wall_spacing) {
                        continue 'placement;
                    }

                    // Placer le mur
                    for x in start_x..start_x+length {
                        cells[y][x] = true;
                    }
                } else {
                    let x = rng.gen_range(wall_margin..MAP_WIDTH - wall_margin);
                    let start_y = rng.gen_range(wall_margin..MAP_HEIGHT - wall_margin - length);
                    
                    // Vérifier si l'emplacement est libre
                    if !is_area_clear(&cells, x, start_y, length, false, wall_spacing) {
                        continue 'placement;
                    }

                    // Placer le mur
                    for y in start_y..start_y+length {
                        cells[y][x] = true;
                    }
                }
                break; // Mur placé avec succès
            }
        }
        Map { cells, internal_wall_count: num_walls, map_width: MAP_WIDTH, map_height: MAP_HEIGHT }
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

fn is_area_clear(cells: &Vec<Vec<bool>>, start_x: usize, start_y: usize, length: usize, is_horizontal: bool, spacing: usize) -> bool {
    let (width, height) = (cells[0].len(), cells.len());
    let (start_check_x, end_check_x, start_check_y, end_check_y) = if is_horizontal {
        (start_x.saturating_sub(spacing), (start_x + length + spacing).min(width),
         start_y.saturating_sub(spacing), (start_y + spacing + 1).min(height))
    } else {
        (start_x.saturating_sub(spacing), (start_x + spacing + 1).min(width),
         start_y.saturating_sub(spacing), (start_y + length + spacing).min(height))
    };

    for y in start_check_y..end_check_y {
        for x in start_check_x..end_check_x {
            if cells[y][x] {
                return false;
            }
        }
    }
    true
}