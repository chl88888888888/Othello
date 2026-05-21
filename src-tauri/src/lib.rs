mod game_logic;
mod response;
mod db;
mod ai;
mod network;

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

// ── 日志初始化（每次启动擦除旧日志，写入单文件）──
fn init_logger(log_dir: &std::path::Path) {
    let _ = std::fs::create_dir_all(log_dir);
    let log_path = log_dir.join("othello.log");
    if let Ok(file) = File::create(&log_path) {
        let _ = WriteLogger::init(LevelFilter::Info, Config::default(), file);
    }
}

// ── 辅助函数 ──────────────────────────────────────

/// 给定预期落子方和刚下完的一方，返回（实际下一手, 游戏是否结束）
fn resolve_next_turn<'a>(
    expected_next: Bitboard,
    just_moved: Bitboard,
    expected_next_name: &'a str,
    just_moved_name: &'a str,
) -> (&'a str, bool) {
    if game_logic::has_legal_move(expected_next, just_moved) {
        (expected_next_name, false)
    } else if game_logic::has_legal_move(just_moved, expected_next) {
        (just_moved_name, false) // pass 回原方
    } else {
        (just_moved_name, true) // 双方均无合法落子 → 游戏结束
    }
}

// ── 基础命令 ──────────────────────────────────────

#[tauri::command]
fn start_game() -> GameStateResponse {
    let (black, white) = game_logic::initial_board();
    info!("新游戏开始");
    GameStateResponse::build_response(black, white, "black", 0)
}

#[tauri::command]
fn make_move(
    black: String,
    white: String,
    pos_index: u32,
    is_black_turn: bool,
) -> Result<GameStateResponse, String> {
    let mut black_bb: Bitboard = black.parse().map_err(|e| format!("无效的 black 值: {e}"))?;
    let mut white_bb: Bitboard = white.parse().map_err(|e| format!("无效的 white 值: {e}"))?;

    if pos_index > 63 {
        return Err("位置索引必须在 0-63 之间".into());
    }
    let pos = game_logic::index_to_bitboard(pos_index);

    // 检查目标位置是否为空（防止在已被占用的格子上落子）
    if (pos & (black_bb | white_bb)) != 0 {
        return Err("该位置已被占用".into());
    }

    let (player, opponent): (&mut Bitboard, &mut Bitboard) = if is_black_turn {
        (&mut black_bb, &mut white_bb)
    } else {
        (&mut white_bb, &mut black_bb)
    };

    // 一次 compute_flips 同时验证合法性和获取翻转结果
    let flips = game_logic::compute_flips(pos, *player, *opponent);
    if flips == 0 {
        return Err("该位置不是合法落子点".into());
    }

    game_logic::make_move_with_flips(player, opponent, pos, flips);

    let resp = build_next_state(black_bb, white_bb, is_black_turn, flips);
    info!("落子: index={pos_index}, 黑={}, 白={}", black_bb.count_ones(), white_bb.count_ones());
    Ok(resp)
}

/// 根据落子后的棋盘状态，确定下一手轮到谁
fn build_next_state(
    black: Bitboard,
    white: Bitboard,
    just_black: bool,
    flips: Bitboard,
) -> GameStateResponse {
    let just_moved = if just_black { "black" } else { "white" };
    let next_turn = if just_black { "white" } else { "black" };
    let (next_player, next_opponent) = if just_black {
        (white, black)
    } else {
        (black, white)
    };
    let (turn, _) = resolve_next_turn(next_player, next_opponent, next_turn, just_moved);
    GameStateResponse::build_response(black, white, turn, flips)
}

// ── AI 相关命令 ───────────────────────────────────

/// AI 落子：自动计算最佳落子并执行。若 AI 无合法落子则自动 pass
#[tauri::command]
fn ai_move(
    ai: tauri::State<'_, AiState>,
    black: String,
    white: String,
    is_black_turn: bool,
) -> Result<GameStateResponse, String> {
    let black_bb: Bitboard = black.parse().map_err(|e| format!("无效的 black 值: {e}"))?;
    let white_bb: Bitboard = white.parse().map_err(|e| format!("无效的 white 值: {e}"))?;

    let (ai_player, ai_opponent) = if is_black_turn {
        (black_bb, white_bb)
    } else {
        (white_bb, black_bb)
    };

    // AI 无合法落子 → pass
    if !game_logic::has_legal_move(ai_player, ai_opponent) {
        let (opp_name, ai_name) = if is_black_turn {
            ("white", "black")
        } else {
            ("black", "white")
        };
        let (turn, _) = resolve_next_turn(ai_opponent, ai_player, opp_name, ai_name);
        let mut resp = GameStateResponse::build_response(black_bb, white_bb, turn, 0);
        resp.ai_move_index = None;
        info!("AI 无合法落子，pass");
        return Ok(resp);
    }

    let ai_guard = ai.lock().map_err(|e| format!("AI 状态锁失败: {e}"))?;
    let model = ai_guard.as_ref().ok_or("AI 模型未加载")?;

    let pos_idx = model
        .best_move(black_bb, white_bb, is_black_turn)
        .ok_or("AI 无合法落子")?;

    drop(ai_guard);

    let pos = game_logic::index_to_bitboard(pos_idx);
    let mut b = black_bb;
    let mut w = white_bb;

    let flips = game_logic::compute_flips(pos, ai_player, ai_opponent);

    let (player, opponent): (&mut Bitboard, &mut Bitboard) = if is_black_turn {
        (&mut b, &mut w)
    } else {
        (&mut w, &mut b)
    };

    game_logic::make_move_with_flips(player, opponent, pos, flips);

    let mut resp = build_next_state(b, w, is_black_turn, flips);
    resp.ai_move_index = Some(pos_idx);
    info!("AI 落子: index={pos_idx}");
    Ok(resp)
}

// ── 数据库相关命令 ────────────────────────────────

#[tauri::command]
fn save_game(
    db: tauri::State<'_, Database>,
    black_score: u32,
    white_score: u32,
    winner: Option<String>,
    moves: Vec<MoveRecord>,
) -> Result<i64, String> {
    let id = db.save_game(black_score, white_score, winner, moves)?;
    info!("对局已保存: id={id}");
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
    info!("对局已删除: id={id}");
    Ok(())
}

// ── 启动入口 ──────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(None::<OthelloModel>))   // AI 状态
        .manage(Mutex::new(OnlineState::new()))      // 联机状态
        .setup(|app| {
            // ── 初始化日志 ──
            let log_dir = app
                .path()
                .app_data_dir()
                .expect("无法获取 app_data_dir");
            init_logger(&log_dir);

            info!("Othello启动");

            // ── 初始化数据库 ──
            let db_path = log_dir.join("othello.db");
            info!("数据库路径: {:?}", db_path);

            match Database::open(db_path) {
                Ok(database) => {
                    app.manage(database);
                    info!("数据库初始化成功");
                }
                Err(e) => {
                    error!("数据库初始化失败: {e}");
                }
            }

            // ── 初始化 AI 模型（编译期嵌入，跨平台兼容）──
            let ai_mutex = app.state::<AiState>();
            let model_bytes = include_bytes!("../resources/othello_model.safetensors").to_vec();
            info!("嵌入模型数据: {} bytes", model_bytes.len());

            match OthelloModel::load_from_bytes(model_bytes) {
                Ok(model) => {
                    let mut ai = ai_mutex.lock().unwrap();
                    *ai = Some(model);
                    info!("AI 模型加载成功");
                }
                Err(e) => {
                    error!("AI 模型加载失败: {e:?}");
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
