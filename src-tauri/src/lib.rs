mod game_logic;
mod response;
mod db;
mod ai;
mod network;

use std::sync::Mutex;
use game_logic::Bitboard;
use response::GameStateResponse;
use db::{Database, MoveRecord, GameSummary, GameStats, GameRecord};
use ai::{AiState, OthelloModel};
use network::OnlineState;
use tauri::Manager;

#[tauri::command]
fn start_game() -> GameStateResponse {
    let (black, white) = game_logic::initial_board();
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

    let (player, opponent): (&mut Bitboard, &mut Bitboard) = if is_black_turn {
        (&mut black_bb, &mut white_bb)
    } else {
        (&mut white_bb, &mut black_bb)
    };

    // 计算翻转并同时验证合法性（一次 compute_flips 同时验证和获取结果）
    let flips = game_logic::compute_flips(pos, *player, *opponent);
    if flips == 0 {
        return Err("该位置不是合法落子点".into());
    }

    // 执行落子（使用预计算的 flips，不再重复计算）
    game_logic::make_move_with_flips(player, opponent, pos, flips);

    Ok(build_next_state(black_bb, white_bb, is_black_turn, flips))
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

    if game_logic::has_legal_move(next_player, next_opponent) {
        GameStateResponse::build_response(black, white, next_turn, flips)
    } else if game_logic::has_legal_move(next_opponent, next_player) {
        // 对方无合法落子，回合交还当前方
        GameStateResponse::build_response(black, white, just_moved, flips)
    } else {
        // 双方均无合法落子 → 游戏结束
        GameStateResponse::build_response(black, white, just_moved, flips)
    }
}

// ---------- AI 相关命令 ----------

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
        let opponent_turn = if is_black_turn { "white" } else { "black" };
        let (opp_player, opp_opponent) = if is_black_turn {
            (white_bb, black_bb)
        } else {
            (black_bb, white_bb)
        };

        let turn = if game_logic::has_legal_move(opp_player, opp_opponent) {
            opponent_turn
        } else {
            // 双方都无合法落子
            if is_black_turn { "black" } else { "white" }
        };

        let mut resp = GameStateResponse::build_response(black_bb, white_bb, turn, 0);
        resp.ai_move_index = None;
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

    // 计算翻转（一次计算，复用）
    let flips = game_logic::compute_flips(pos, ai_player, ai_opponent);

    let (player, opponent): (&mut Bitboard, &mut Bitboard) = if is_black_turn {
        (&mut b, &mut w)
    } else {
        (&mut w, &mut b)
    };

    game_logic::make_move_with_flips(player, opponent, pos, flips);

    let mut resp = build_next_state(b, w, is_black_turn, flips);
    resp.ai_move_index = Some(pos_idx);
    Ok(resp)
}

// ---------- 数据库相关命令 ----------

/// 保存一局已结束的对局
#[tauri::command]
fn save_game(
    db: tauri::State<'_, Database>,
    black_score: u32,
    white_score: u32,
    winner: Option<String>,
    moves: Vec<MoveRecord>,
) -> Result<i64, String> {
    db.save_game(black_score, white_score, winner, moves)
}

/// 获取所有对局摘要列表
#[tauri::command]
fn get_game_list(
    db: tauri::State<'_, Database>,
) -> Result<Vec<GameSummary>, String> {
    db.get_game_list()
}

/// 获取单局完整记录（用于回放）
#[tauri::command]
fn get_game_detail(
    db: tauri::State<'_, Database>,
    id: i64,
) -> Result<GameRecord, String> {
    db.get_game(id)
}

/// 获取总胜负统计
#[tauri::command]
fn get_stats(
    db: tauri::State<'_, Database>,
) -> Result<GameStats, String> {
    db.get_stats()
}

/// 删除指定对局
#[tauri::command]
fn delete_game(
    db: tauri::State<'_, Database>,
    id: i64,
) -> Result<(), String> {
    db.delete_game(id)
}

// ---------- 启动入口 ----------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(None::<OthelloModel>)) // AI 状态
        .manage(Mutex::new(OnlineState::new())) // 联机状态
        .setup(|app| {
            // ── 初始化数据库 ──
            // Android: 使用 Tauri 的 app_data_dir，可写入
            let db_dir = app
                .path()
                .app_data_dir()
                .expect("无法获取 app_data_dir");

            // 确保目录存在
            if let Err(e) = std::fs::create_dir_all(&db_dir) {
                eprintln!("[DB] 无法创建数据目录 {:?}: {e}", db_dir);
            }

            let db_path = db_dir.join("othello.db");
            println!("[DB] 数据库路径: {:?}", db_path);

            match Database::open(db_path) {
                Ok(database) => {
                    app.manage(database);
                    println!("[DB] 数据库初始化成功");
                }
                Err(e) => {
                    // 不要 panic，只打印错误，app 仍然可以运行（只是无法保存记录）
                    eprintln!("[DB] 数据库初始化失败: {e}");
                }
            }

            // ── 初始化 AI 模型（编译期嵌入，确保跨平台兼容）──
            let ai_mutex = app.state::<AiState>();

            // 使用 include_bytes! 在编译时嵌入模型，避免 Android 运行时资产解析问题
            let model_bytes = include_bytes!("../resources/othello_model.safetensors").to_vec();
            println!("[AI] 嵌入模型数据: {} bytes", model_bytes.len());
            match OthelloModel::load_from_bytes(model_bytes) {
                Ok(model) => {
                    let mut ai = ai_mutex.lock().unwrap();
                    *ai = Some(model);
                    println!("[AI] 模型加载成功（编译期嵌入）");
                }
                Err(e) => {
                    eprintln!("[AI] 模型加载失败: {e:?}");
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
