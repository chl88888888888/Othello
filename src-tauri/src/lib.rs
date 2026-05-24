mod ai;
mod core;
mod db;
mod game_logic;
mod network;
mod response;

#[cfg(test)]
mod test;

use ai::{AiState, OthelloModel};
use db::{Database, GameRecord, GameStats, GameSummary, MoveRecord};
use game_logic::Bitboard;
use log::{error, info, LevelFilter};
use network::OnlineState;
use response::GameStateResponse;
use simplelog::{Config, WriteLogger};
use std::fs::File;
use std::sync::Mutex;
use tauri::Manager;

// ── Log initialization (overwrites old log on each startup) ──
fn init_logger(log_dir: &std::path::Path) {
    let _ = std::fs::create_dir_all(log_dir);
    let log_path = log_dir.join("othello.log");
    if let Ok(file) = File::create(&log_path) {
        let _ = WriteLogger::init(LevelFilter::Info, Config::default(), file);
    }
}

// ── Helpers ─────────────────────────────────────

fn parse_bitboards(black: &str, white: &str) -> Result<(Bitboard, Bitboard), String> {
    let b: Bitboard = black
        .parse()
        .map_err(|e| format!("Invalid black value: {e}"))?;
    let w: Bitboard = white
        .parse()
        .map_err(|e| format!("Invalid white value: {e}"))?;
    Ok((b, w))
}

// ── Basic Commands ────────────────────────────────

#[tauri::command]
fn start_game() -> GameStateResponse {
    let (black, white) = game_logic::initial_board();
    info!("New game started");
    GameStateResponse::build_response(black, white, "black", 0)
}

#[tauri::command]
fn make_move(
    black: String,
    white: String,
    pos_index: u32,
    is_black_turn: bool,
) -> Result<GameStateResponse, String> {
    let (black_bb, white_bb) = parse_bitboards(&black, &white)?;

    let (new_black, new_white, flips) =
        core::make_move_core(black_bb, white_bb, pos_index, is_black_turn)?;

    let resp = core::build_next_state(new_black, new_white, is_black_turn, flips);
    info!(
        "Move: index={pos_index}, black={}, white={}",
        new_black.count_ones(),
        new_white.count_ones()
    );
    Ok(resp)
}

// ── AI Related Commands ────────────────────────────

/// AI move: auto-computes the best move and executes it. Auto-passes if AI has no legal move.
#[tauri::command]
fn ai_move(
    ai: tauri::State<'_, AiState>,
    black: String,
    white: String,
    is_black_turn: bool,
) -> Result<GameStateResponse, String> {
    let (black_bb, white_bb) = parse_bitboards(&black, &white)?;

    let (player, opponent) = if is_black_turn {
        (black_bb, white_bb)
    } else {
        (white_bb, black_bb)
    };

    if !game_logic::has_legal_move(player, opponent) {
        info!("AI has no legal move, passing");
        return Ok(core::ai_pass_response(black_bb, white_bb, is_black_turn));
    }

    let ai_guard = ai
        .lock()
        .map_err(|e| format!("AI state lock failed: {e}"))?;
    let model = ai_guard.as_ref().ok_or("AI model not loaded")?;
    let (new_black, new_white, flips, pos_idx) =
        core::ai_move_core(model, black_bb, white_bb, is_black_turn)?;
    drop(ai_guard);

    let mut resp = core::build_next_state(new_black, new_white, is_black_turn, flips);
    resp.ai_move_index = Some(pos_idx);
    info!("AI move: index={pos_idx}");
    Ok(resp)
}

// ── Database Related Commands ─────────────────────

#[tauri::command]
fn save_game(
    db: tauri::State<'_, Database>,
    black_score: u32,
    white_score: u32,
    winner: Option<String>,
    moves: Vec<MoveRecord>,
) -> Result<i64, String> {
    let id = db.save_game(black_score, white_score, winner, moves)?;
    info!("Game saved: id={id}");
    Ok(id)
}

#[tauri::command]
fn get_game_list(db: tauri::State<'_, Database>) -> Result<Vec<GameSummary>, String> {
    db.get_game_list()
}

#[tauri::command]
fn get_game_detail(db: tauri::State<'_, Database>, id: i64) -> Result<GameRecord, String> {
    db.get_game(id)
}

#[tauri::command]
fn get_stats(db: tauri::State<'_, Database>) -> Result<GameStats, String> {
    db.get_stats()
}

#[tauri::command]
fn delete_game(db: tauri::State<'_, Database>, id: i64) -> Result<(), String> {
    db.delete_game(id)?;
    info!("Game deleted: id={id}");
    Ok(())
}

// ── App Entry Point ──────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(None::<OthelloModel>)) // AI state
        .manage(Mutex::new(OnlineState::new())) // online state
        .setup(|app| {
            // ── Initialize Logger ──
            let log_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app_data_dir");
            init_logger(&log_dir);

            info!("Othello started");

            // ── Initialize Database ──
            let db_path = log_dir.join("othello.db");
            info!("Database path: {:?}", db_path);

            match Database::open(db_path) {
                Ok(database) => {
                    app.manage(database);
                    info!("Database initialized successfully");
                }
                Err(e) => {
                    error!("Database initialization failed: {e}");
                }
            }

            // ── Initialize AI model (embedded at compile time, cross-platform) ──
            let ai_mutex = app.state::<AiState>();
            let model_bytes = include_bytes!("../resources/othello_model.safetensors").to_vec();
            info!("Embedded model data: {} bytes", model_bytes.len());

            match OthelloModel::load_from_bytes(model_bytes) {
                Ok(model) => {
                    let mut ai = ai_mutex.lock().unwrap();
                    *ai = Some(model);
                    info!("AI model loaded successfully");
                }
                Err(e) => {
                    error!("AI model load failed: {e:?}");
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_game,
            make_move,
            ai_move,
            save_game,
            get_game_list,
            get_game_detail,
            get_stats,
            delete_game,
            network::connect_server,
            network::disconnect_server,
            network::find_match,
            network::online_send_move,
            network::online_give_up,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
