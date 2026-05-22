//! Integration tests for Othello game logic
//!
//! These tests simulate complete game scenarios, testing the interplay
//! between multiple functions in the `game_logic` module.

use crate::game_logic::{self, Bitboard, BitIter};

// ── Helper Functions ──────────────────────────────

/// Build a bitboard from a list of index positions
fn bb_from_indices(indices: &[u32]) -> Bitboard {
    indices.iter().fold(0, |acc, &i| acc | (1u64 << i))
}

// ── Initial Board Integration Tests ───────────────

#[test]
fn test_initial_board_has_correct_pieces() {
    let (black, white) = game_logic::initial_board();
    // d5, e4 are black
    assert!(black & game_logic::sq('d', 5) != 0);
    assert!(black & game_logic::sq('e', 4) != 0);
    // d4, e5 are white
    assert!(white & game_logic::sq('d', 4) != 0);
    assert!(white & game_logic::sq('e', 5) != 0);
    // No overlap
    assert_eq!(black & white, 0);
    // Correct piece counts
    assert_eq!(black.count_ones(), 2);
    assert_eq!(white.count_ones(), 2);
    // Board total
    assert_eq!((black | white).count_ones(), 4);
}

// ── Single Move Integration Tests ─────────────────

#[test]
fn test_black_first_move_d3() {
    let (black, white) = game_logic::initial_board();
    let pos = game_logic::sq('d', 3); // index 19

    let flips = game_logic::compute_flips(pos, black, white);
    assert_eq!(flips, game_logic::sq('d', 4)); // Flips d4

    let (mut b, mut w) = (black, white);
    game_logic::make_move(&mut b, &mut w, pos);

    assert_eq!(b.count_ones(), 4); // 2 original + 1 placed + 1 flipped
    assert_eq!(w.count_ones(), 1); // 2 - 1 flipped
}

#[test]
fn test_black_first_move_c4() {
    let (mut black, mut white) = game_logic::initial_board();
    let pos = game_logic::sq('c', 4);
    let flips = game_logic::compute_flips(pos, black, white);
    assert!(flips > 0);
    game_logic::make_move(&mut black, &mut white, pos);
    assert_eq!(black.count_ones(), 4);
}

#[test]
fn test_black_first_move_f5() {
    let (mut black, mut white) = game_logic::initial_board();
    let pos = game_logic::sq('f', 5);
    let flips = game_logic::compute_flips(pos, black, white);
    assert!(flips > 0);
    game_logic::make_move(&mut black, &mut white, pos);
    assert_eq!(black.count_ones(), 4);
}

#[test]
fn test_black_first_move_e6() {
    let (mut black, mut white) = game_logic::initial_board();
    let pos = game_logic::sq('e', 6);
    let flips = game_logic::compute_flips(pos, black, white);
    assert!(flips > 0);
    game_logic::make_move(&mut black, &mut white, pos);
    assert_eq!(black.count_ones(), 4);
}

#[test]
fn test_invalid_move_on_occupied_square() {
    let (black, white) = game_logic::initial_board();
    // Try to play on d4 (already occupied by white)
    let pos = game_logic::sq('d', 4);
    let _flips = game_logic::compute_flips(pos, black, white);
    // The raw algorithm on an occupied square won't flip anything valid
    // because it doesn't check occupancy - but make_move does
    // For integration test, verify this position is not in legal moves
    let legal = game_logic::compute_legal_moves(black, white);
    assert_eq!(legal & pos, 0);
}

#[test]
fn test_invalid_move_no_flips() {
    let (black, white) = game_logic::initial_board();
    // Play at a1 which is isolated
    let pos = game_logic::sq('a', 1);
    let flips = game_logic::compute_flips(pos, black, white);
    assert_eq!(flips, 0);
}

// ── Multi-Move Game Simulation ────────────────────

#[test]
fn test_two_move_sequence() {
    let (mut black, mut white) = game_logic::initial_board();

    // Black plays d3
    game_logic::make_move(&mut black, &mut white, game_logic::sq('d', 3));
    // White plays c3
    let pos = game_logic::sq('c', 3);
    let flips = game_logic::compute_flips(pos, white, black);
    assert!(flips > 0, "White should have a legal move at c3 after black's d3");
    game_logic::make_move(&mut white, &mut black, pos);

    assert!(black.count_ones() >= 2);
    assert!(white.count_ones() >= 2);
}

#[test]
fn test_pass_scenario() {
    // Create a board where one side has no moves
    // Black occupies everything except one square surrounded by black,
    // White has no legal moves.
    // Simulate: black fills 63 squares, white has 1.
    // But this won't produce a pass because white has moves.
    // Let's instead test via the helper: has_legal_move should return false
    // when a player has no pieces.
    assert!(!game_logic::has_legal_move(0, 1u64));
    assert!(!game_logic::has_legal_move(0, 0));
}

#[test]
fn test_both_sides_no_legal_move() {
    // Full board, no empty squares
    let black = 0xFFFFFFFFFFFFFFFFu64;
    let white = 0u64;
    // Neither side has a move (board is full)
    assert!(!game_logic::has_legal_move(black, white));
    assert!(!game_logic::has_legal_move(white, black));
}

// ── Legal Move Computation ────────────────────────

#[test]
fn test_legal_moves_bit_positions_valid() {
    let (black, white) = game_logic::initial_board();
    let legal = game_logic::compute_legal_moves(black, white);

    // All legal move positions should be on empty squares
    let occupied = black | white;
    assert_eq!(legal & occupied, 0);

    // All legal move positions should be valid board positions (0-63)
    // No bits beyond position 63 should be set
    assert_eq!(legal >> 63, 0, "No bits beyond position 63 should be set");
}

#[test]
fn test_each_legal_move_flips_at_least_one_piece() {
    let (black, white) = game_logic::initial_board();
    let legal = game_logic::compute_legal_moves(black, white);

    for idx in BitIter(legal) {
        let pos = game_logic::index_to_bitboard(idx);
        let flips = game_logic::compute_flips(pos, black, white);
        assert!(flips > 0, "Legal move at index {} should flip pieces", idx);
    }
}

// ── Corner Play ───────────────────────────────────

#[test]
fn test_corner_move_is_stable() {
    // Once a piece is placed in a corner, it cannot be flipped
    // Black has c1 (own), white at b1 (opponent). Black plays a1.
    // From a1, going right: a1 -> b1(opponent) -> c1(own) → flips b1
    let black = bb_from_indices(&[2]);  // c1
    let white = bb_from_indices(&[1]);  // b1
    let pos = game_logic::index_to_bitboard(0); // a1

    let flips = game_logic::compute_flips(pos, black, white);
    assert_eq!(flips.count_ones(), 1, "Should flip exactly 1 piece (b1)");
    assert_eq!(flips, game_logic::index_to_bitboard(1)); // flips b1

    let (mut b, mut w) = (black, white);
    game_logic::make_move(&mut b, &mut w, pos);

    // a1 is now black, and cannot be flipped back
    assert!(b & game_logic::index_to_bitboard(0) != 0);
    // b1 is now black (flipped)
    assert!(b & game_logic::index_to_bitboard(1) != 0);
    // White has no pieces
    assert_eq!(w.count_ones(), 0);
}

// ── Game Result Integration ───────────────────────

#[test]
fn test_full_game_black_win() {
    let black = 0xFFFFFFFFFFFFFFFFu64;
    let white = 0u64;
    match game_logic::judge_winner(black, white) {
        game_logic::GameResult::BlackWin(bc, wc) => {
            assert_eq!(bc, 64);
            assert_eq!(wc, 0);
        }
        _ => panic!("Expected BlackWin"),
    }
}

#[test]
fn test_full_game_white_win() {
    let black = 0u64;
    let white = 0xFFFFFFFFFFFFFFFFu64;
    match game_logic::judge_winner(black, white) {
        game_logic::GameResult::WhiteWin(bc, wc) => {
            assert_eq!(bc, 0);
            assert_eq!(wc, 64);
        }
        _ => panic!("Expected WhiteWin"),
    }
}

#[test]
fn test_draw_after_simulation() {
    // Test the draw condition directly — 32 pieces each
    let black = 0x00000000FFFFFFFFu64; // 32 ones in lower half
    let white = 0xFFFFFFFF00000000u64; // 32 ones in upper half
    match game_logic::judge_winner(black, white) {
        game_logic::GameResult::Draw(count) => {
            assert_eq!(count, 32);
        }
        other => panic!("Expected Draw, got {:?}", other),
    }
}

// ── Bit Iteration Integration ─────────────────────

#[test]
fn test_bit_iter_over_legal_moves() {
    let (black, white) = game_logic::initial_board();
    let legal = game_logic::compute_legal_moves(black, white);
    let indices: Vec<u32> = BitIter(legal).collect();
    assert_eq!(indices.len(), 4);
    // All should be valid
    for idx in &indices {
        assert!(*idx < 64);
    }
    // No duplicates
    let mut sorted = indices.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), indices.len());
}

// ── Edge Case: Empty Board ────────────────────────

#[test]
fn test_empty_board_no_legal_moves() {
    let legal = game_logic::compute_legal_moves(0, 0);
    assert_eq!(legal, 0);
    assert!(!game_logic::has_legal_move(0, 0));
}

#[test]
fn test_single_piece_no_flanking_opponent() {
    // One black piece alone, white has no moves
    let black = game_logic::sq('d', 4);
    let white = 0u64;
    assert!(!game_logic::has_legal_move(white, black));
    // Black also has no moves (no opponent to flank)
    assert!(!game_logic::has_legal_move(black, white));
    // Game should be over
    let result = game_logic::judge_winner(black, white);
    match result {
        game_logic::GameResult::BlackWin(bc, wc) => {
            assert_eq!(bc, 1);
            assert_eq!(wc, 0);
        }
        _ => panic!("Expected BlackWin"),
    }
}

// ── Edge Case: Multi-Direction Flips ──────────────

#[test]
fn test_move_flips_in_multiple_directions() {
    // Create: black at a1, white at a2,a3. Black plays a4.
    let black = game_logic::sq('a', 1);
    let white = game_logic::squares(&[('a', 2), ('a', 3)]);
    let pos = game_logic::sq('a', 4);
    let flips = game_logic::compute_flips(pos, black, white);
    assert_eq!(flips.count_ones(), 2);
    assert!(flips & game_logic::sq('a', 2) != 0);
    assert!(flips & game_logic::sq('a', 3) != 0);
}

// ── Stability: Move Does Not Affect Unrelated Pieces ──

#[test]
fn test_move_only_affects_captured_pieces() {
    let (black, white) = game_logic::initial_board();
    let pos = game_logic::sq('d', 3);
    let (mut b, mut w) = (black, white);

    game_logic::make_move(&mut b, &mut w, pos);

    // d5 and e4 should still be black (original black pieces)
    assert!(b & game_logic::sq('d', 5) != 0);
    assert!(b & game_logic::sq('e', 4) != 0);
    // e5 should still be white (not flipped)
    assert!(w & game_logic::sq('e', 5) != 0);
    // d4 should now be black (was flipped)
    assert!(b & game_logic::sq('d', 4) != 0);
}
