//! 黑白棋 — Hyperbola Quintessence 实现
//! 核心 bitboard 操作库：落子翻转、合法落子检测、胜负判定

pub type Bitboard = u64;

// ---------- 边界掩码 ----------
const FILE_A: Bitboard = 0x0101010101010101;
const FILE_H: Bitboard = 0x8080808080808080;
const RANK_1: Bitboard = 0x00000000000000FF;
const RANK_8: Bitboard = 0xFF00000000000000;

// ---------- 方向定义 ----------
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

// ---------- 安全位移 ----------
fn shift(b: Bitboard, shift: i32) -> Bitboard {
    if shift > 0 {
        b.wrapping_shl(shift as u32)
    } else {
        b.wrapping_shr((-shift) as u32)
    }
}

// ---------- 单方向翻转 ----------
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

// ---------- 全方向总翻转 ----------
/// 计算在 `pos` 落子后，会翻转的对方棋子 bitboard
pub fn compute_flips(pos: Bitboard, own: Bitboard, opponent: Bitboard) -> Bitboard {
    let mut total = 0;
    DIRECTIONS
        .iter()
        .for_each(|dir| total |= flip_one_dir(pos, own, opponent, dir));
    total
}

// ---------- 合法落子点检测 ----------
fn empty_squares(player: Bitboard, opponent: Bitboard) -> Bitboard {
    !(player | opponent)
}

/// 判断当前玩家是否有合法落子
pub fn has_legal_move(player: Bitboard, opponent: Bitboard) -> bool {
    let mut empty = empty_squares(player, opponent);
    while empty != 0 {
        let pos = empty & empty.wrapping_neg();
        if compute_flips(pos, player, opponent) != 0 {
            return true;
        }
        empty &= empty.wrapping_sub(1);
    }
    false
}

/// 计算所有合法落子位置，返回 bitboard（每位代表一个可落子位置）
pub fn compute_legal_moves(player: Bitboard, opponent: Bitboard) -> Bitboard {
    let mut empty = empty_squares(player, opponent);
    let mut legal = 0u64;
    while empty != 0 {
        let pos = empty & empty.wrapping_neg();
        if compute_flips(pos, player, opponent) != 0 {
            legal |= pos;
        }
        empty &= empty.wrapping_sub(1);
    }
    legal
}

// ---------- 执行落子 ----------
/// 在 `pos` 落子，同时翻转被夹对方棋子。`player`/`opponent` 会被原地修改
pub fn make_move(player: &mut Bitboard, opponent: &mut Bitboard, pos: Bitboard) {
    let flips = compute_flips(pos, *player, *opponent);
    *player ^= pos | flips;
    *opponent ^= flips;
}

/// 使用预计算的 flips 执行落子，避免重复计算
pub fn make_move_with_flips(
    player: &mut Bitboard,
    opponent: &mut Bitboard,
    pos: Bitboard,
    flips: Bitboard,
) {
    *player ^= pos | flips;
    *opponent ^= flips;
}

// ---------- 辅助函数 ----------
/// 将棋盘坐标转为 bitboard（例如 `sq('d', 5)` → d5 对应的位）
pub fn sq(file: char, rank: u8) -> Bitboard {
    let f = (file as u8 - b'a') as u32;
    let r = (rank - 1) as u32;
    1u64 << (r * 8 + f)
}

/// 批量坐标 → bitboard
pub fn squares(specs: &[(char, u8)]) -> Bitboard {
    specs.iter().fold(0, |acc, &(f, r)| acc | sq(f, r))
}

/// 索引 → bitboard（index: 0=a1, 7=h1, 56=a8, 63=h8）
pub fn index_to_bitboard(index: u32) -> Bitboard {
    1u64 << index
}

// ---------- 胜负判断 ----------
#[derive(Debug, PartialEq)]
pub enum GameResult {
    BlackWin(u32, u32),
    WhiteWin(u32, u32),
    Draw(u32),
}

/// 双方都无法落子时，比较棋子个数判定胜负
pub fn judge_winner(black: Bitboard, white: Bitboard) -> GameResult {
    let bc = black.count_ones();
    let wc = white.count_ones();
    match bc.cmp(&wc) {
        std::cmp::Ordering::Greater => GameResult::BlackWin(bc, wc),
        std::cmp::Ordering::Less => GameResult::WhiteWin(bc, wc),
        std::cmp::Ordering::Equal => GameResult::Draw(bc),
    }
}

/// 初始棋盘
pub fn initial_board() -> (Bitboard, Bitboard) {
    let black = squares(&[('d', 5), ('e', 4)]);
    let white = squares(&[('d', 4), ('e', 5)]);
    (black, white)
}
