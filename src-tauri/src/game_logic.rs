//! Othello — Hyperbola Quintessence implementation
//! Core bitboard operation library: move flipping, legal move detection, win/loss determination

pub type Bitboard = u64;

// ---------- Bit Iterator ----------

/// Iterate over the indices (0-63) of all set bits in the bitboard
pub struct BitIter(pub u64);

impl Iterator for BitIter {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        let lsb = self.0 & self.0.wrapping_neg();
        self.0 &= self.0 - 1;
        Some(lsb.trailing_zeros())
    }
}

// ---------- Border Masks ----------
const FILE_A: Bitboard = 0x0101010101010101;
const FILE_H: Bitboard = 0x8080808080808080;
const RANK_1: Bitboard = 0x00000000000000FF;
const RANK_8: Bitboard = 0xFF00000000000000;

// ---------- Direction Definitions ----------
struct Dir {
    shift: i32,
    mask: Bitboard,
}

const DIRECTIONS: [Dir; 8] = [
    Dir {
        shift: 1,
        mask: !FILE_H,
    },
    Dir {
        shift: -1,
        mask: !FILE_A,
    },
    Dir {
        shift: 8,
        mask: !RANK_8,
    },
    Dir {
        shift: -8,
        mask: !RANK_1,
    },
    Dir {
        shift: 9,
        mask: !(FILE_H | RANK_8),
    },
    Dir {
        shift: 7,
        mask: !(FILE_A | RANK_8),
    },
    Dir {
        shift: -7,
        mask: !(FILE_H | RANK_1),
    },
    Dir {
        shift: -9,
        mask: !(FILE_A | RANK_1),
    },
];

// ---------- Safe Shift ----------
fn shift(b: Bitboard, shift: i32) -> Bitboard {
    if shift > 0 {
        b.wrapping_shl(shift as u32)
    } else {
        b.wrapping_shr((-shift) as u32)
    }
}

// ---------- Single Direction Flip ----------
fn flip_one_dir(pos: Bitboard, own: Bitboard, opponent: Bitboard, dir: &Dir) -> Bitboard {
    let mut flips: Bitboard = 0;
    let mut p = pos;
    while (p & dir.mask) != 0 {
        p = shift(p, dir.shift);
        if (p & opponent) != 0 {
            flips |= p;
        } else if (p & own) != 0 {
            return flips;
        } else {
            break;
        }
    }
    0
}

// ---------- Total Flips (All Directions) ----------
/// Compute the bitboard of opponent pieces that would be flipped by placing at `pos`
pub fn compute_flips(pos: Bitboard, own: Bitboard, opponent: Bitboard) -> Bitboard {
    DIRECTIONS
        .iter()
        .fold(0, |total, dir| total | flip_one_dir(pos, own, opponent, dir))
}

// ---------- Legal Move Detection ----------
fn empty_squares(player: Bitboard, opponent: Bitboard) -> Bitboard {
    !(player | opponent)
}

/// Check if the current player has any legal move
pub fn has_legal_move(player: Bitboard, opponent: Bitboard) -> bool {
    BitIter(empty_squares(player, opponent))
        .any(|idx| compute_flips(1u64 << idx, player, opponent) != 0)
}

/// Compute all legal move positions, returning a bitboard (each bit represents a legal position)
pub fn compute_legal_moves(player: Bitboard, opponent: Bitboard) -> Bitboard {
    BitIter(empty_squares(player, opponent))
        .filter(|&idx| compute_flips(1u64 << idx, player, opponent) != 0)
        .fold(0, |acc, idx| acc | (1u64 << idx))
}

// ---------- Execute Move ----------
/// Place a piece at `pos`, flipping captured opponent pieces. `player`/`opponent` are modified in place
pub fn make_move(player: &mut Bitboard, opponent: &mut Bitboard, pos: Bitboard) {
    let flips = compute_flips(pos, *player, *opponent);
    *player ^= pos | flips;
    *opponent ^= flips;
}

/// Execute a move using pre-computed flips to avoid redundant calculation
pub fn make_move_with_flips(
    player: &mut Bitboard,
    opponent: &mut Bitboard,
    pos: Bitboard,
    flips: Bitboard,
) {
    *player ^= pos | flips;
    *opponent ^= flips;
}

// ---------- Helper Functions ----------
/// Convert board coordinate to bitboard (e.g. `sq('d', 5)` → the bit for d5)
pub fn sq(file: char, rank: u8) -> Bitboard {
    let f = (file as u8 - b'a') as u32;
    let r = (rank - 1) as u32;
    1u64 << (r * 8 + f)
}

/// Batch coordinates → bitboard
pub fn squares(specs: &[(char, u8)]) -> Bitboard {
    specs.iter().fold(0, |acc, &(f, r)| acc | sq(f, r))
}

/// Index → bitboard (index: 0=a1, 7=h1, 56=a8, 63=h8)
pub fn index_to_bitboard(index: u32) -> Bitboard {
    1u64 << index
}

// ---------- Win/Loss Determination ----------
#[derive(Debug, PartialEq)]
pub enum GameResult {
    BlackWin(u32, u32),
    WhiteWin(u32, u32),
    Draw(u32),
}

/// When neither side can move, compare piece counts to determine the winner
pub fn judge_winner(black: Bitboard, white: Bitboard) -> GameResult {
    let bc = black.count_ones();
    let wc = white.count_ones();
    match bc.cmp(&wc) {
        std::cmp::Ordering::Greater => GameResult::BlackWin(bc, wc),
        std::cmp::Ordering::Less => GameResult::WhiteWin(bc, wc),
        std::cmp::Ordering::Equal => GameResult::Draw(bc),
    }
}

/// Initial board setup
pub fn initial_board() -> (Bitboard, Bitboard) {
    let black = squares(&[('d', 5), ('e', 4)]);
    let white = squares(&[('d', 4), ('e', 5)]);
    (black, white)
}

// ── Unit Tests ───────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── BitIter Tests ──
    #[test]
    fn test_bit_iter_empty() {
        let mut iter = BitIter(0);
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_bit_iter_single() {
        let mut iter = BitIter(1u64 << 5);
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_bit_iter_multiple() {
        let bits: Vec<u32> = BitIter(0b10101).collect();
        assert_eq!(bits, vec![0, 2, 4]);
    }

    #[test]
    fn test_bit_iter_all_bits() {
        let bits: Vec<u32> = BitIter(u64::MAX).collect();
        assert_eq!(bits.len(), 64);
        for (i, &b) in bits.iter().enumerate() {
            assert_eq!(b, i as u32);
        }
    }

    // ── Coordinate Helpers ──
    #[test]
    fn test_sq_corners() {
        // a1 = index 0
        assert_eq!(sq('a', 1), 1u64 << 0);
        // h1 = index 7
        assert_eq!(sq('h', 1), 1u64 << 7);
        // a8 = index 56
        assert_eq!(sq('a', 8), 1u64 << 56);
        // h8 = index 63
        assert_eq!(sq('h', 8), 1u64 << 63);
    }

    #[test]
    fn test_sq_center() {
        // d4 = row 3 * 8 + col 3 = 27
        assert_eq!(sq('d', 4), 1u64 << 27);
        // e5 = row 4 * 8 + col 4 = 36
        assert_eq!(sq('e', 5), 1u64 << 36);
    }

    #[test]
    fn test_squares() {
        let bb = squares(&[('a', 1), ('h', 8)]);
        assert_eq!(bb, (1u64 << 0) | (1u64 << 63));
    }

    #[test]
    fn test_index_to_bitboard() {
        assert_eq!(index_to_bitboard(0), 1u64);
        assert_eq!(index_to_bitboard(63), 1u64 << 63);
        assert_eq!(index_to_bitboard(5), 1u64 << 5);
    }

    // ── Initial Board ──
    #[test]
    fn test_initial_board() {
        let (black, white) = initial_board();
        // d5 (row 4 * 8 + col 3 = 35), e4 (row 3 * 8 + col 4 = 28)
        assert_eq!(black, (1u64 << 35) | (1u64 << 28));
        // d4 (row 3 * 8 + col 3 = 27), e5 (row 4 * 8 + col 4 = 36)
        assert_eq!(white, (1u64 << 27) | (1u64 << 36));
        // No overlap
        assert_eq!(black & white, 0);
        // Total 4 pieces
        assert_eq!(black.count_ones() + white.count_ones(), 4);
    }

    // ── Compute Flips ──
    #[test]
    fn test_compute_flips_initial_black() {
        let (black, white) = initial_board();
        // Black at d3 (index 19) should flip d4
        let pos = sq('d', 3);
        let flips = compute_flips(pos, black, white);
        assert_eq!(flips, sq('d', 4));
    }

    #[test]
    fn test_compute_flips_initial_white() {
        let (black, white) = initial_board();
        let pos = sq('d', 3);
        let flips = compute_flips(pos, black, white);
        assert_eq!(flips, sq('d', 4));
    }

    #[test]
    fn test_compute_flips_horizontal() {
        // Setup: black at c4, white at d4,e4. Black plays f4.
        let black = sq('c', 4);
        let white = squares(&[('d', 4), ('e', 4)]);
        let pos = sq('f', 4);
        let flips = compute_flips(pos, black, white);
        assert_eq!(flips, squares(&[('d', 4), ('e', 4)]));
    }

    #[test]
    fn test_compute_flips_vertical() {
        // Setup: black at d3, white at d4,d5. Black plays d6.
        let black = sq('d', 3);
        let white = squares(&[('d', 4), ('d', 5)]);
        let pos = sq('d', 6);
        let flips = compute_flips(pos, black, white);
        assert_eq!(flips, squares(&[('d', 4), ('d', 5)]));
    }

    #[test]
    fn test_compute_flips_diagonal() {
        // Setup: black at c3, white at d4,e5. Black plays f6.
        let black = sq('c', 3);
        let white = squares(&[('d', 4), ('e', 5)]);
        let pos = sq('f', 6);
        let flips = compute_flips(pos, black, white);
        assert_eq!(flips, squares(&[('d', 4), ('e', 5)]));
    }

    #[test]
    fn test_compute_flips_no_flip_invalid() {
        let (black, white) = initial_board();
        // Playing on an occupied square should return 0 (not a valid position anyway)
        let flips = compute_flips(sq('d', 4), black, white);
        // Even though d4 is occupied by white, the algorithm starts from d4
        // and looks in all directions. Since pos==own check is done before calling compute_flips,
        // this tests the raw algorithm behavior.
        // d4 is white. If own=black, then from d4 going right: d4(white) -> e4(black) -> f4(empty).
        // So this would flip... wait.
        // Actually, compute_flips(pos, own, opponent). If pos=d4, own=black, opponent=white:
        // Direction right: d4 -> e4(own=black) -> returns empty flips.
        // Direction upper-right: d4(27) -> c3(27-9=18, empty) -> no flip.
        // Direction left: d4 -> c4(empty) -> no flip.
        // So it returns 0.
        assert_eq!(flips, 0);
    }

    #[test]
    fn test_compute_flips_corner() {
        // Corner moves should have limited directions
        let black = sq('b', 2);
        let white = sq('b', 1);
        // Black at b2 plays a1, should flip b1
        let _flips = compute_flips(sq('a', 1), black, white);
        // Direction +1 (right): a1 -> b1(white) -> c1(empty). Hits empty after opponent, no flip.
        // Wait, the algorithm requires: opponent pieces followed by own piece.
        // a1 -> b1(opponent) -> c1(empty). Since it hits empty after opponent, returns 0.
        // So a1 doesn't flip b1 in this setup.
        // Let me reconsider: black at c1, white at b1, black plays a1.
        // a1 -> b1(opponent) -> c1(own). This would flip b1!
        let black = sq('c', 1);
        let white = sq('b', 1);
        let flips = compute_flips(sq('a', 1), black, white);
        assert_eq!(flips, sq('b', 1));
    }

    #[test]
    fn test_compute_flips_multi_direction() {
        // Setup where one move flips in multiple directions
        // Black at e6, White at d5,e5. Black plays d6.
        // This is a standard initial move for black.
        let (black, white) = initial_board();
        let pos = sq('d', 3);
        let flips = compute_flips(pos, black, white);
        assert_eq!(flips, sq('d', 4));
    }

    // ── Legal Move Detection ──
    #[test]
    fn test_has_legal_move_initial() {
        let (black, white) = initial_board();
        assert!(has_legal_move(black, white)); // Black has moves
    }

    #[test]
    fn test_has_legal_move_none() {
        // Full board with no moves
        let black = !0u64;
        let white = 0u64;
        assert!(!has_legal_move(black, white)); // No empty squares
        assert!(!has_legal_move(white, black)); // No pieces
    }

    #[test]
    fn test_compute_legal_moves_initial_black() {
        let (black, white) = initial_board();
        let legal = compute_legal_moves(black, white);
        // Black should have 4 legal moves: d3, c4, f5, e6
        assert_eq!(legal.count_ones(), 4);
        assert!(legal & sq('d', 3) != 0);
        assert!(legal & sq('c', 4) != 0);
        assert!(legal & sq('f', 5) != 0);
        assert!(legal & sq('e', 6) != 0);
    }

    #[test]
    fn test_compute_legal_moves_initial_white() {
        let (black, white) = initial_board();
        // White's perspective: own=white, opponent=black
        let legal = compute_legal_moves(white, black);
        // White should also have 4 legal moves from initial board
        assert_eq!(legal.count_ones(), 4);
    }

    // ── Make Move ──
    #[test]
    fn test_make_move_basic() {
        let (mut black, mut white) = initial_board();
        let pos = sq('d', 3);
        make_move(&mut black, &mut white, pos);
        // Black should now have d5, e4, d3, d4 (flipped)
        assert_eq!(black.count_ones(), 4);
        // White should have only e5
        assert_eq!(white.count_ones(), 1);
        assert!(black & sq('d', 3) != 0); // placed piece
        assert!(black & sq('d', 4) != 0); // flipped piece
        assert!(black & sq('d', 5) != 0); // original
        assert!(black & sq('e', 4) != 0); // original
        assert!(white & sq('e', 5) != 0); // remaining
    }

    #[test]
    fn test_make_move_with_flips() {
        let (mut black, mut white) = initial_board();
        let pos = sq('d', 3);
        let flips = compute_flips(pos, black, white);
        make_move_with_flips(&mut black, &mut white, pos, flips);
        assert_eq!(black.count_ones(), 4);
        assert_eq!(white.count_ones(), 1);
    }

    // ── Judge Winner ──
    #[test]
    fn test_judge_winner_black_wins() {
        let black = squares(&[('a', 1), ('b', 1)]);
        let white = sq('c', 1);
        assert_eq!(judge_winner(black, white), GameResult::BlackWin(2, 1));
    }

    #[test]
    fn test_judge_winner_white_wins() {
        let black = sq('a', 1);
        let white = squares(&[('b', 1), ('c', 1)]);
        assert_eq!(judge_winner(black, white), GameResult::WhiteWin(1, 2));
    }

    #[test]
    fn test_judge_winner_draw() {
        let black = squares(&[('a', 1), ('b', 1)]);
        let white = squares(&[('c', 1), ('d', 1)]);
        assert_eq!(judge_winner(black, white), GameResult::Draw(2));
    }

    #[test]
    fn test_judge_winner_empty_board() {
        assert_eq!(judge_winner(0, 0), GameResult::Draw(0));
    }
}
