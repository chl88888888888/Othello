use crate::game_logic::{self, Bitboard};
use serde::Serialize;

// ---------- Response struct (u64 passed as strings to avoid JS precision loss) ----------

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
    /// Bitboard of pieces flipped by this move (passed as string)
    pub flips: String,
    /// AI move position index (only used by ai_move command)
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

        // Compute legal moves for current side; also determine if game is over (avoid repeated is_game_over calls)
        let legal = game_logic::compute_legal_moves(player, opponent);
        let game_over = if legal != 0 {
            false // current side has legal move → game continues
        } else {
            // current side has no legal move → check if opponent also has no legal move
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
