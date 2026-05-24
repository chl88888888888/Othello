mod game_logic;
mod response;
mod db;
mod ai;
mod network;

#[cfg(test)]
mod test;

use std::sync::Mutex;
use std::fs::File;
use game_logic::Bitboard;
use response::GameStateResponse;
use db::{Database, MoveRecord, GameSummary, GameStats, GameRecord};
use ai::{AiState, OthelloModel};
use network::OnlineState;
use tauri::Manager;
use log::{info, error, LevelFilter};
use simplelog::{WriteLogger, Config};

// ── Log initialization (overwrites old log on each startup) ──
fn init_logger(log_dir: &std::path::Path) {
    let _ = std::fs::create_dir_all(log_dir);
    let log_path = log_dir.join("othello.log");
    if let Ok(file) = File::create(&log_path) {
        let _ = WriteLogger::init(LevelFilter::Info, Config::default(), file);
    }
}

// ── Helper Functions ──────────────────────────────

/// Given the expected next player and the side that just moved,
/// returns (actual next turn, whether game is over)
fn resolve_next_turn<'a>(
    expected_next: Bitboard,
    just_moved: Bitboard,
    expected_next_name: &'a str,
    just_moved_name: &'a str,
) -> (&'a str, bool) {
    if game_logic::has_legal_move(expected_next, just_moved) {
        (expected_next_name, false)
    } else if game_logic::has_legal_move(just_moved, expected_next) {
        (just_moved_name, false) // pass back to original side
    } else {
        (just_moved_name, true) // both sides have no legal move → game over
    }
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
    let mut black_bb: Bitboard = black.parse().map_err(|e| format!("Invalid black value: {e}"))?;
    let mut white_bb: Bitboard = white.parse().map_err(|e| format!("Invalid white value: {e}"))?;

    if pos_index > 63 {
        return Err("Position index must be between 0-63".into());
    }
    let pos = game_logic::index_to_bitboard(pos_index);

    // Check if the target position is empty (prevent placing on occupied squares)
    if (pos & (black_bb | white_bb)) != 0 {
        return Err("This position is already occupied".into());
    }

    let (player, opponent): (&mut Bitboard, &mut Bitboard) = if is_black_turn {
        (&mut black_bb, &mut white_bb)
    } else {
        (&mut white_bb, &mut black_bb)
    };

    // compute_flips validates legality and returns flip results in one call
    let flips = game_logic::compute_flips(pos, *player, *opponent);
    if flips == 0 {
        return Err("This position is not a legal move".into());
    }

    game_logic::make_move_with_flips(player, opponent, pos, flips);

    let resp = build_next_state(black_bb, white_bb, is_black_turn, flips);
    info!("Move: index={pos_index}, black={}, white={}", black_bb.count_ones(), white_bb.count_ones());
    Ok(resp)
}

/// Determine whose turn is next based on the board state after a move
fn build_next_state(
    black: Bitboard,
    white: Bitboard,
    just_black: bool,
    flips: Bitboard,
) -> GameStateResponse {
    let (just_moved, next_turn) = if just_black {
        ("black", "white")
    } else {
        ("white", "black")
    };
    let (next_player, next_opponent) = if just_black {
        (white, black)
    } else {
        (black, white)
    };
    let (turn, _) = resolve_next_turn(next_player, next_opponent, next_turn, just_moved);
    GameStateResponse::build_response(black, white, turn, flips)
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
    let black_bb: Bitboard = black.parse().map_err(|e| format!("Invalid black value: {e}"))?;
    let white_bb: Bitboard = white.parse().map_err(|e| format!("Invalid white value: {e}"))?;

    let (ai_player, ai_opponent) = if is_black_turn {
        (black_bb, white_bb)
    } else {
        (white_bb, black_bb)
    };

    // AI has no legal move → pass
    if !game_logic::has_legal_move(ai_player, ai_opponent) {
        let (opp_name, ai_name) = if is_black_turn {
            ("white", "black")
        } else {
            ("black", "white")
        };
        let (turn, _) = resolve_next_turn(ai_opponent, ai_player, opp_name, ai_name);
        let mut resp = GameStateResponse::build_response(black_bb, white_bb, turn, 0);
        resp.ai_move_index = None;
        info!("AI has no legal move, passing");
        return Ok(resp);
    }

    let ai_guard = ai.lock().map_err(|e| format!("AI state lock failed: {e}"))?;
    let model = ai_guard.as_ref().ok_or("AI model not loaded")?;

    let pos_idx = model
        .best_move(black_bb, white_bb, is_black_turn)
        .ok_or("AI has no legal move")?;

    drop(ai_guard);

    let pos = game_logic::index_to_bitboard(pos_idx);
    let mut b = black_bb;
    let mut w = white_bb;

    let flips = game_logic::compute_flips(pos, ai_player, ai_opponent);

    if is_black_turn {
        game_logic::make_move_with_flips(&mut b, &mut w, pos, flips);
    } else {
        game_logic::make_move_with_flips(&mut w, &mut b, pos, flips);
    }

    let mut resp = build_next_state(b, w, is_black_turn, flips);
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
fn get_game_list(
    db: tauri::State<'_, Database>,
) -> Result<Vec<GameSummary>, String> {
    db.get_game_list()
}

#[tauri::command]
fn get_game_detail(
    db: tauri::State<'_, Database>,
    id: i64,
) -> Result<GameRecord, String> {
    db.get_game(id)
}

#[tauri::command]
fn get_stats(
    db: tauri::State<'_, Database>,
) -> Result<GameStats, String> {
    db.get_stats()
}

#[tauri::command]
fn delete_game(
    db: tauri::State<'_, Database>,
    id: i64,
) -> Result<(), String> {
    db.delete_game(id)?;
    info!("Game deleted: id={id}");
    Ok(())
}

// ── App Entry Point ──────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(None::<OthelloModel>))   // AI state
        .manage(Mutex::new(OnlineState::new()))      // online state
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
