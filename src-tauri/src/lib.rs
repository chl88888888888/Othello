mod game_logic;
mod response;

use game_logic::Bitboard;
use response::GameStateResponse;

// ---------- Tauri 命令 ----------

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

    // 验证落子合法
    let legal = game_logic::compute_legal_moves(*player, *opponent);
    if (pos & legal) == 0 {
        return Err("该位置不是合法落子点".into());
    }

    // 计算本轮会翻转的棋子（在落子之前计算）
    let flips = game_logic::compute_flips(pos, *player, *opponent);

    // 执行落子
    game_logic::make_move(player, opponent, pos);

    // 确定下一手轮到谁
    let next_turn = if is_black_turn { "white" } else { "black" };
    let (next_player, next_opponent) = if is_black_turn {
        (white_bb, black_bb)
    } else {
        (black_bb, white_bb)
    };

    if game_logic::has_legal_move(next_player, next_opponent) {
        Ok(GameStateResponse::build_response(black_bb, white_bb, next_turn, flips))
    } else if game_logic::has_legal_move(next_opponent, next_player) {
        // 对方无合法落子，回合交还当前方
        let current = if is_black_turn { "black" } else { "white" };
        Ok(GameStateResponse::build_response(black_bb, white_bb, current, flips))
    } else {
        // 双方均无合法落子 → 游戏结束
        let last_turn = if is_black_turn { "black" } else { "white" };
        Ok(GameStateResponse::build_response(black_bb, white_bb, last_turn, flips))
    }
}

// ---------- 启动入口 ----------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![start_game, make_move])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
