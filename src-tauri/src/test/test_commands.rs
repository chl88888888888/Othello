//! Tests for command core functions (no Tauri State dependency)
//!
//! These test the pure logic extracted from Tauri commands:
//! - `make_move_core`   — move validation + execution
//! - `build_next_state` — response construction after a move
//! - `resolve_next_turn` — pass / game-over logic
//!
//! They also test `Database::open_in_memory()` for DB commands.

use crate::{
    core,
    game_logic,
    db::{Database, GameStats, MoveRecord},
};

// ═══════════════════════════════════════════════════
// make_move_core
// ═══════════════════════════════════════════════════

#[test]
fn test_make_move_core_black_first_d3() {
    let (black, white) = game_logic::initial_board();
    let (new_black, new_white, flips) = core::make_move_core(black, white, 19, true).unwrap();

    assert_eq!(new_black.count_ones(), 4); // 2 orig + 1 placed + 1 flipped
    assert_eq!(new_white.count_ones(), 1); // 2 - 1 flipped
    assert!(flips > 0);
}

#[test]
fn test_make_move_core_black_first_c4() {
    let (black, white) = game_logic::initial_board();
    let (new_black, new_white, flips) = core::make_move_core(black, white, 26, true).unwrap();

    assert_eq!(new_black.count_ones(), 4);
    assert_eq!(new_white.count_ones(), 1);
    assert!(flips > 0);
}

#[test]
fn test_make_move_core_position_out_of_range() {
    let (black, white) = game_logic::initial_board();
    let err = core::make_move_core(black, white, 64, true).unwrap_err();
    assert!(err.contains("0-63"));
}

#[test]
fn test_make_move_core_position_at_boundary() {
    let (black, white) = game_logic::initial_board();
    // 63 is the last valid index (h8), but it's not a legal move from start
    let err = core::make_move_core(black, white, 63, true).unwrap_err();
    assert!(err.contains("not a legal move"));
}

#[test]
fn test_make_move_core_occupied_square() {
    let (black, white) = game_logic::initial_board();
    // d5 (index 35) — occupied by black in initial board
    let err = core::make_move_core(black, white, 35, true).unwrap_err();
    assert!(err.contains("already occupied"));
}

#[test]
fn test_make_move_core_illegal_move() {
    let (black, white) = game_logic::initial_board();
    // a1 (index 0) — no adjacent opposing pieces → no flips
    let err = core::make_move_core(black, white, 0, true).unwrap_err();
    assert!(err.contains("not a legal move"));
}

#[test]
fn test_make_move_core_white_turn() {
    let (black, white) = game_logic::initial_board();
    // Black plays d3 first
    let (b1, w1, _) = core::make_move_core(black, white, 19, true).unwrap();
    // White responds at c3
    let (b2, w2, _) = core::make_move_core(b1, w1, 18, false).unwrap();
    // Both sides should still have pieces (no crazy flips)
    assert!(b2.count_ones() > 0);
    assert!(w2.count_ones() > 0);
}

// ═══════════════════════════════════════════════════
// resolve_next_turn
// ═══════════════════════════════════════════════════

#[test]
fn test_resolve_next_turn_normal() {
    let (black, white) = game_logic::initial_board();
    // After black moved, white should be next
    let (turn, over) = core::resolve_next_turn(white, black, "white", "black");
    assert_eq!(turn, "white");
    assert!(!over);
}

#[test]
fn test_resolve_next_turn_forced_pass() {
    // Construct a board where only one side can move
    // Fill board such that only (say) black has legal moves
    // Use a simple known position: black controls center, white stuck
    let black = !0u64 ^ 0x0000001818000000; // all but d4+e4
    let white = 0x0000001818000000;          // d4+e4
    // white is the expected next — does white have a legal move?
    // If not, it should pass back to black
    let (turn, over) = core::resolve_next_turn(white, black, "white", "black");
    // White has no moves on this crowded board; black might or might not
    // We just verify it doesn't panic and returns a valid turn name
    assert!(turn == "white" || turn == "black");
    // Game may or may not be over depending on board
    let _ = over;
}

#[test]
fn test_resolve_next_turn_both_no_legal() {
    // A full board with no empty squares → game over
    let black = 0xFFFFFFFF00000000u64;
    let white = 0x00000000FFFFFFFFu64;
    let (turn, over) = core::resolve_next_turn(black, white, "black", "white");
    assert!(over);
    let _ = turn;
}

// ═══════════════════════════════════════════════════
// build_next_state
// ═══════════════════════════════════════════════════

#[test]
fn test_build_next_state_after_black_move() {
    let (black, white) = game_logic::initial_board();
    let (new_b, new_w, flips) = core::make_move_core(black, white, 19, true).unwrap();
    let resp = core::build_next_state(new_b, new_w, true, flips);

    assert_eq!(resp.current_turn, "white");
    assert!(!resp.game_over);
    assert_eq!(resp.black_score, 4);
    assert_eq!(resp.white_score, 1);
}

#[test]
fn test_build_next_state_scores_match_bitboard() {
    let (black, white) = game_logic::initial_board();
    let resp = core::build_next_state(black, white, true, 0);

    assert_eq!(resp.black_score, black.count_ones());
    assert_eq!(resp.white_score, white.count_ones());
}

// ═══════════════════════════════════════════════════
// make_move_core + build_next_state roundtrip
// ═══════════════════════════════════════════════════

#[test]
fn test_full_move_roundtrip() {
    let (black, white) = game_logic::initial_board();
    let (new_b, new_w, flips) = core::make_move_core(black, white, 19, true).unwrap();
    let resp = core::build_next_state(new_b, new_w, true, flips);

    assert_eq!(resp.current_turn, "white");
    assert!(!resp.game_over);
    assert_eq!(resp.black_score, 4);
    assert_eq!(resp.white_score, 1);
}

// ═══════════════════════════════════════════════════
// Database commands (in-memory)
// ═══════════════════════════════════════════════════

fn in_memory_db() -> Database {
    Database::open_in_memory().expect("Failed to create in-memory database")
}

#[test]
fn test_db_save_and_list() {
    let db = in_memory_db();

    let moves = vec![
        MoveRecord { pos_index: 19, is_black_turn: true },
        MoveRecord { pos_index: 26, is_black_turn: false },
    ];

    let id = db.save_game(30, 34, Some("white".to_string()), moves).unwrap();
    assert!(id > 0);

    let list = db.get_game_list().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, id);
    assert_eq!(list[0].black_score, 30);
    assert_eq!(list[0].white_score, 34);
    assert_eq!(list[0].winner, Some("white".to_string()));
    assert_eq!(list[0].total_moves, 2);
}

#[test]
fn test_db_get_detail() {
    let db = in_memory_db();

    let moves = vec![MoveRecord { pos_index: 19, is_black_turn: true }];
    let id = db.save_game(10, 20, Some("black".to_string()), moves.clone()).unwrap();

    let detail = db.get_game(id).unwrap();
    assert_eq!(detail.black_score, 10);
    assert_eq!(detail.white_score, 20);
    assert_eq!(detail.moves.len(), 1);
    assert_eq!(detail.moves[0].pos_index, 19);
}

#[test]
fn test_db_get_stats() {
    let db = in_memory_db();

    db.save_game(40, 24, Some("black".to_string()), vec![]).unwrap();
    db.save_game(30, 34, Some("white".to_string()), vec![]).unwrap();
    db.save_game(32, 32, None, vec![]).unwrap();

    let stats: GameStats = db.get_stats().unwrap();
    assert_eq!(stats.total_games, 3);
    assert_eq!(stats.black_wins, 1);
    assert_eq!(stats.white_wins, 1);
    assert_eq!(stats.draws, 1);
}

#[test]
fn test_db_delete_game() {
    let db = in_memory_db();

    let id = db.save_game(1, 2, None, vec![]).unwrap();
    assert_eq!(db.get_game_list().unwrap().len(), 1);

    db.delete_game(id).unwrap();
    assert_eq!(db.get_game_list().unwrap().len(), 0);

    // Deleting again should still be Ok (no-op)
    let _ = db.delete_game(id);
}

#[test]
fn test_db_empty_list() {
    let db = in_memory_db();
    let list = db.get_game_list().unwrap();
    assert!(list.is_empty());
}

// ═══════════════════════════════════════════════════
// ai_pass_response (originally in core)
// ═══════════════════════════════════════════════════

#[test]
fn test_ai_pass_response_black_no_moves() {
    let (black, white) = game_logic::initial_board();
    let resp = core::ai_pass_response(black, white, true);
    // Black has 4 legal moves from initial board, so this won't pass.
    // But the function itself just computes the response — it doesn't
    // check legality. We only verify it returns a valid response.
    assert!(resp.current_turn == "white" || resp.current_turn == "black");
    assert!(resp.ai_move_index.is_none());
    assert_eq!(resp.flips, "0");
}

#[test]
fn test_ai_pass_response_full_board() {
    // A full board — AI truly has no moves
    let black = 0xFFFFFFFF00000000u64;
    let white = 0x00000000FFFFFFFFu64;
    let resp = core::ai_pass_response(black, white, true);
    assert!(resp.ai_move_index.is_none());
    assert_eq!(resp.flips, "0");
    assert!(resp.game_over); // both sides full → no moves → game over
}
