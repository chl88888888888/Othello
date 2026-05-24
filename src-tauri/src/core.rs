//! Core game logic — pure functions with no Tauri dependency.
//!
//! These are the testable "brains" behind the Tauri commands.
//! All functions here take plain Rust types, never `tauri::State`.

use crate::game_logic::{self, Bitboard};
use crate::ai::OthelloModel;
use crate::response::GameStateResponse;

// ── Turn Resolution ───────────────────────────────

/// Given the expected next player and the side that just moved,
/// returns (actual next turn, whether game is over)
pub fn resolve_next_turn<'a>(
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

// ── Move Execution ────────────────────────────────

/// Core move logic — pure function, no Tauri dependency.
/// Validates and executes a move on the given board.
/// Returns (new_black, new_white, flips) on success.
pub fn make_move_core(
    black: Bitboard,
    white: Bitboard,
    pos_index: u32,
    is_black_turn: bool,
) -> Result<(Bitboard, Bitboard, Bitboard), String> {
    if pos_index > 63 {
        return Err("Position index must be between 0-63".into());
    }
    let pos = game_logic::index_to_bitboard(pos_index);

    if (pos & (black | white)) != 0 {
        return Err("This position is already occupied".into());
    }

    let mut black_bb = black;
    let mut white_bb = white;

    let (player, opponent): (&mut Bitboard, &mut Bitboard) = if is_black_turn {
        (&mut black_bb, &mut white_bb)
    } else {
        (&mut white_bb, &mut black_bb)
    };

    let flips = game_logic::compute_flips(pos, *player, *opponent);
    if flips == 0 {
        return Err("This position is not a legal move".into());
    }

    game_logic::make_move_with_flips(player, opponent, pos, flips);

    Ok((black_bb, white_bb, flips))
}

// ── AI Move ───────────────────────────────────────

/// Core AI move logic — takes a model reference instead of Tauri State.
/// Caller must ensure AI has at least one legal move before calling.
/// Returns (new_black, new_white, flips, ai_move_index).
pub fn ai_move_core(
    model: &OthelloModel,
    black: Bitboard,
    white: Bitboard,
    is_black_turn: bool,
) -> Result<(Bitboard, Bitboard, Bitboard, u32), String> {
    let pos_idx = model
        .best_move(black, white, is_black_turn)
        .ok_or("AI has no legal move")?;

    let pos = game_logic::index_to_bitboard(pos_idx);
    let mut b = black;
    let mut w = white;

    let (player, opponent): (&mut Bitboard, &mut Bitboard) = if is_black_turn {
        (&mut b, &mut w)
    } else {
        (&mut w, &mut b)
    };

    let flips = game_logic::compute_flips(pos, *player, *opponent);
    game_logic::make_move_with_flips(player, opponent, pos, flips);

    Ok((b, w, flips, pos_idx))
}

/// Build the response when AI has no legal move (pass turn).
/// Resolves whose turn it is next and returns a GameStateResponse
/// with `ai_move_index = None`.
pub fn ai_pass_response(
    black: Bitboard,
    white: Bitboard,
    is_black_turn: bool,
) -> GameStateResponse {
    let (ai_player, ai_opponent, opp_name, ai_name) = if is_black_turn {
        (black, white, "white", "black")
    } else {
        (white, black, "black", "white")
    };
    let (turn, _) = resolve_next_turn(ai_opponent, ai_player, opp_name, ai_name);
    let mut resp = GameStateResponse::build_response(black, white, turn, 0);
    resp.ai_move_index = None;
    resp
}

// ── Response Building ─────────────────────────────

/// Determine whose turn is next based on the board state after a move,
/// and build the full GameStateResponse.
pub fn build_next_state(
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
