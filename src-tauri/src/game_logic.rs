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
