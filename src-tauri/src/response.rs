use crate::game_logic::{self, Bitboard};
use serde::Serialize;

// ---------- Response struct (u64 passed as strings to avoid JS precision loss) ----------

#[derive(Serialize, Clone, Debug)]
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

// ── Unit Tests ───────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_logic;

    #[test]
    fn test_build_response_initial_black() {
        let (black, white) = game_logic::initial_board();
        let resp = GameStateResponse::build_response(black, white, "black", 0);

        assert_eq!(resp.current_turn, "black");
        assert!(!resp.game_over);
        assert_eq!(resp.black_score, 2);
        assert_eq!(resp.white_score, 2);
        assert!(resp.winner.is_none());
        assert_eq!(resp.flips, "0");
        assert!(resp.ai_move_index.is_none());

        // Legal moves should be non-zero (initial board has 4 moves for black)
        let legal: u64 = resp.legal_moves.parse().unwrap();
        assert!(legal > 0);
    }

    #[test]
    fn test_build_response_game_over_black_win() {
        // Fill the board: black has all but one square
        let black = !(1u64 << 0);
        let white = 1u64 << 0;
        let resp = GameStateResponse::build_response(black, white, "black", 0);

        assert!(resp.game_over);
        assert_eq!(resp.winner, Some("black".to_string()));
        assert_eq!(resp.black_score, 63);
        assert_eq!(resp.white_score, 1);
    }

    #[test]
    fn test_build_response_game_over_draw() {
        let black = 0xFFFFFFFF00000000u64; // 32 pieces each
        let white = 0x00000000FFFFFFFFu64;
        let resp = GameStateResponse::build_response(black, white, "black", 0);

        assert!(resp.game_over);
        assert_eq!(resp.winner, None);
        assert_eq!(resp.black_score, 32);
        assert_eq!(resp.white_score, 32);
    }

    #[test]
    fn test_build_response_black_and_white_serialized() {
        let (black, white) = game_logic::initial_board();
        let resp = GameStateResponse::build_response(black, white, "black", 0);

        // Verify u64 values are correctly serialized as strings
        let parsed_black: u64 = resp.black.parse().unwrap();
        let parsed_white: u64 = resp.white.parse().unwrap();
        assert_eq!(parsed_black, black);
        assert_eq!(parsed_white, white);
    }
}
