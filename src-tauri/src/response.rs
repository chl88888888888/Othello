use crate::game_logic::{self, Bitboard};
use serde::Serialize;

// ---------- 响应结构体（u64 以字符串形式传递，避免 JS 精度丢失） ----------

#[derive(Serialize, Clone)]
pub struct GameStateResponse {
    pub black: String,
    pub white: String,
    pub legal_moves: String,
    pub current_turn: String,
    pub game_over: bool,
    pub black_score: u32,
    pub white_score: u32,
    pub winner: Option<String>,
    /// 本次落子翻转的棋子位板（以字符串形式传递）
    pub flips: String,
    /// AI 落子位置索引（仅 ai_move 命令使用）
    pub ai_move_index: Option<u32>,
}

impl GameStateResponse {
    pub fn build_response(
        black: Bitboard,
        white: Bitboard,
        current_turn: &str,
        flips: Bitboard,
    ) -> Self {
        let (player, opponent) = if current_turn == "black" {
            (black, white)
        } else {
            (white, black)
        };

        // 计算当前方合法落子；同时判断游戏是否结束（避免重复调用 is_game_over）
        let legal = game_logic::compute_legal_moves(player, opponent);
        let game_over = if legal != 0 {
            false // 当前方有合法落子 → 游戏继续
        } else {
            // 当前方无合法落子 → 检查对方是否也无合法落子
            !game_logic::has_legal_move(opponent, player)
        };

        let (winner, over) = if game_over {
            let result = game_logic::judge_winner(black, white);
            let w = match result {
                game_logic::GameResult::BlackWin(_, _) => Some("black".to_string()),
                game_logic::GameResult::WhiteWin(_, _) => Some("white".to_string()),
                game_logic::GameResult::Draw(_) => None,
            };
            (w, true)
        } else {
            (None, false)
        };

        GameStateResponse {
            black: black.to_string(),
            white: white.to_string(),
            legal_moves: legal.to_string(),
            current_turn: current_turn.to_string(),
            game_over: over,
            black_score: black.count_ones(),
            white_score: white.count_ones(),
            winner,
            flips: flips.to_string(),
            ai_move_index: None,
        }
    }
}
