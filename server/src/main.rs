use tokio::net::UdpSocket;
use serde::{Serialize, Deserialize};
use std::time::Instant;
use std::{collections::HashMap, time::Duration};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use rand::Rng;
use std::io::{self, Write};

const MAP_WIDTH: usize = 20;
const MAP_HEIGHT: usize = 20;
const PLAYER_SPEED: f32 = 0.1;
const SHOOT_RANGE: f32 = 10.0;

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Map {
    cells: Vec<Vec<bool>>, // true pour un mur, false pour un espace vide
}

impl Map {
    fn new(difficulty: u8) -> Self {
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

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Player {
    name: String,
    position: (f32, f32),
    is_alive: bool,
    points: u32,
}

#[derive(Serialize, Deserialize, Debug)]
enum ClientMessage {
    Join { name: String },
    Move { direction: (f32, f32) },
    Shoot { direction: (f32, f32) },
}

#[derive(Serialize, Deserialize, Debug)]
enum ServerMessage {
    Welcome { map: Map, player_id: String, difficulty: u8 },
    GameState { players: HashMap<String, (f32, f32, bool)> },
    PlayerShot { shooter: String, target: String },
    PlayerDied { player: String },
    GameOver { winner: String, scores: Vec<(String, u32)> },
}

struct GameState {
    players: HashMap<SocketAddr, Player>,
    map: Map,
    difficulty: u8,
    game_start_time: Instant,
    game_duration: Duration,
}

impl GameState {
    fn new(difficulty: u8) -> Self {
        Self {
            players: HashMap::new(),
            map: Map::new(difficulty),
            difficulty,
            game_start_time: Instant::now(),
            game_duration: Duration::from_secs(60), // 5 minutes
        }
    }

    fn is_game_over(&self) -> bool {
        self.game_start_time.elapsed() >= self.game_duration
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Choose difficulty level (1: Easy, 2: Medium, 3: Hard):");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let difficulty: u8 = input.trim().parse().unwrap_or(2);

    let socket = UdpSocket::bind("0.0.0.0:34254").await?;
    let socket = Arc::new(socket);
    let game_state = Arc::new(Mutex::new(GameState::new(difficulty)));

    let game_state_clone = Arc::clone(&game_state);
    let socket_clone = Arc::clone(&socket);
    tokio::spawn(async move {
        if let Err(e) = check_game_over(game_state_clone, socket_clone).await {
            eprintln!("Error in game over check: {}", e);
        }
    });

    println!("Server listening on {}", socket.local_addr()?);

    loop {
        let mut buf = vec![0u8; 4096]; 
        let (len, addr) = socket.recv_from(&mut buf).await?;
        let message: ClientMessage = serde_json::from_slice(&buf[..len])?;
        handle_message(message, addr, Arc::clone(&game_state), Arc::clone(&socket)).await?;
    }
}

fn is_valid_move(map: &Map, x: f32, y: f32) -> bool {
    let cell_x = x.floor() as usize;
    let cell_y = y.floor() as usize;
    cell_x < MAP_WIDTH && cell_y < MAP_HEIGHT && !map.is_wall(cell_x, cell_y)
}

async fn handle_message(
    message: ClientMessage,
    addr: SocketAddr,
    game_state: Arc<Mutex<GameState>>,
    socket: Arc<UdpSocket>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut state = game_state.lock().await;
    match message {
        ClientMessage::Join { name } => {
            println!("Player connected: {} (IP: {})", name, addr);
            let player = Player {
                name: name.clone(),
                position: (1.0, 1.0),
                is_alive: true,
                points: 0
            };
            state.players.insert(addr, player);
            let welcome_message = ServerMessage::Welcome {
                map: state.map.clone(),
                player_id: name,
                difficulty: state.difficulty,
            };
            let serialized = serde_json::to_string(&welcome_message)?;
            socket.send_to(serialized.as_bytes(), addr).await?;
            println!("Sent Welcome message to new player");
            broadcast_game_state(&state, &socket).await?;
        }
        ClientMessage::Move { direction } => {
            let new_position = {
                if let Some(player) = state.players.get(&addr) {
                    let new_x = player.position.0 + direction.0 * PLAYER_SPEED;
                    let new_y = player.position.1 + direction.1 * PLAYER_SPEED;
                    if is_valid_move(&state.map, new_x, new_y) {
                        Some((new_x, new_y))
                    } else {
                        None
                    }
                } else {
                    None
                }
            };
        
            if let Some(new_pos) = new_position {
                if let Some(player) = state.players.get_mut(&addr) {
                    player.position = new_pos;
                }
            }
        }
        ClientMessage::Shoot { direction } => {
            let shooter_name = state.players.get(&addr).map(|p| p.name.clone());
            if let Some(shooter_name) = shooter_name {
                println!("Player {} is shooting!", shooter_name);
                
                let start_pos = state.players.get(&addr).unwrap().position;
                let end_pos = (
                    start_pos.0 + direction.0 * SHOOT_RANGE,
                    start_pos.1 + direction.1 * SHOOT_RANGE
                );
                
                let mut hit_player = None;
                let mut closest_distance = f32::MAX;
                
                for (player_addr, player) in state.players.iter_mut() {
                    if player_addr != &addr && player.is_alive {
                        let player_pos = player.position;
                        
                        // Calculer la distance du joueur à la ligne de tir
                        let a = end_pos.1 - start_pos.1;
                        let b = start_pos.0 - end_pos.0;
                        let c = end_pos.0 * start_pos.1 - start_pos.0 * end_pos.1;
                        
                        let distance = (a * player_pos.0 + b * player_pos.1 + c).abs() / (a * a + b * b).sqrt();
                        
                        // Vérifier si le joueur est dans la portée et la direction du tir
                        let dot_product = (player_pos.0 - start_pos.0) * direction.0 + (player_pos.1 - start_pos.1) * direction.1;
                        
                        if distance < 0.5 && dot_product > 0.0 && dot_product < SHOOT_RANGE {
                            let player_distance = ((player_pos.0 - start_pos.0).powi(2) + (player_pos.1 - start_pos.1).powi(2)).sqrt();
                            if player_distance < closest_distance {
                                closest_distance = player_distance;
                                hit_player = Some((player_addr.clone(), player.name.clone()));
                            }
                        }
                    }
                }
                
                if let Some((hit_addr, hit_name)) = hit_player {
                    if let Some(player) = state.players.get_mut(&hit_addr) {
                        player.is_alive = false;
                    }
                    if let Some(shooter) = state.players.get_mut(&addr) {
                        shooter.points += 10;
                    }
                    
                    let shot_message = ServerMessage::PlayerShot { 
                        shooter: shooter_name.clone(),
                        target: hit_name.clone(),
                    };
                    let serialized = serde_json::to_string(&shot_message)?;
                    socket.send_to(serialized.as_bytes(), &hit_addr).await?;
                    
                    let death_message = ServerMessage::PlayerDied { 
                        player: hit_name.clone()
                    };
                    let serialized = serde_json::to_string(&death_message)?;
                    for addr in state.players.keys() {
                        socket.send_to(serialized.as_bytes(), addr).await?;
                    }
                    
                    println!("Player {} was shot and killed by {}!", hit_name, shooter_name);
                }
            }
        }
    }
    broadcast_game_state(&state, &socket).await?;
    Ok(())
}

async fn broadcast_game_state(
    state: &GameState,
    socket: &Arc<UdpSocket>,
) -> Result<(), Box<dyn std::error::Error>> {
    let players_state: HashMap<String, (f32, f32, bool)> = state.players
        .iter()
        .map(|(_, player)| (player.name.clone(), (player.position.0, player.position.1, player.is_alive)))
        .collect();
    let game_state_message = ServerMessage::GameState { players: players_state.clone() };
    let serialized = serde_json::to_string(&game_state_message)?;
    
    println!("Broadcasting GameState:");
    for (name, (x, y, is_alive)) in &players_state {
        println!("  Player: {}, Position: ({}, {}), Alive: {}", name, x, y, is_alive);
    }
    
    for addr in state.players.keys() {
        socket.send_to(serialized.as_bytes(), addr).await?;
    }
    Ok(())
}

async fn check_game_over(game_state: Arc<Mutex<GameState>>, socket: Arc<UdpSocket>) -> Result<(), Box<dyn std::error::Error>> {
    let mut interval = tokio::time::interval(Duration::from_secs(1));

    loop {
        interval.tick().await;
        let mut state = game_state.lock().await;

        if state.is_game_over() {
            let winner = state.players.values()
                .max_by_key(|p| p.points)
                .cloned();

            if let Some(winner) = winner {
                let game_over_message = ServerMessage::GameOver {
                    winner: winner.name,
                    scores: state.players.values().map(|p| (p.name.clone(), p.points)).collect(),
                };

                let serialized = serde_json::to_string(&game_over_message)?;
                for addr in state.players.keys() {
                    socket.send_to(serialized.as_bytes(), addr).await?;
                }

                // Réinitialiser le jeu
                *state = GameState::new(state.difficulty);
            }
        }
    }
}