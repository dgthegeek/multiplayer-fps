# Game Server

This server manages a real-time multiplayer game where players can move through a maze and battle each other.

## Features

- Simultaneous handling of multiple clients
- Procedural map generation with different difficulty levels
- Shooting-based combat system
- Score management and winner determination
- Network communication via UDP

## Prerequisites

- Rust (2021 edition or higher)
- Cargo (Rust package manager)

## Installation

1. Clone this repository:

```
git clone https://learn.zone01dakar.sn/git/dgaye/multiplayer-fps.git
cd multiplayer-fps
```

## Usage

1. Launch the server:

```
cargo run --bin maze_wars_server
```

2. Choose the difficulty level when prompted:
- 1: Easy
- 2: Medium
- 3: Hard

3. The server will listen on the address `0.0.0.0:34254` by default.

## Project Structure

- `main.rs`: Server entry point
- `game_state.rs`: Game state management
- `map.rs`: Map generation and management
- `player.rs`: Player definition and logic
- `messages.rs`: Client/server message definitions
- `network.rs`: Network communication management
- `handlers.rs`: Message processing and game logic

## Communication Protocol

The server uses JSON messages to communicate with clients. The main types of messages are:

### Client Messages

- `Join`: New player connection
- `Move`: Player movement
- `Shoot`: Player shooting

### Server Messages

- `Welcome`: Welcoming a new player with game information
- `GameState`: Game state update
- `PlayerShot`: Successful shot notification
- `PlayerDied`: Player death notification
- `GameOver`: End of game with scores

## Customization

- Modify constants in `map.rs` to change the map size
- Adjust `PLAYER_SPEED` and `SHOOT_RANGE` in `player.rs` to modify game dynamics
- Change the game duration by modifying `game_duration` in `GameState::new()`