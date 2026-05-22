//! AI module — Loads a trained CNN model for board evaluation and move selection
//! Model architecture: AlphaZero-style residual network
//!   Input: 2×8×8 (black/white bitboards)
//!   → Conv2d(2→64, 3×3, p1) → ReLU
//!   → ResBlock×3 (Conv2d 64→64, 3×3 + skip)
//!   → Conv2d(64→1, 1×1) → Flatten → Linear(64→1) → tanh

use candle_core::{Device, DType, Tensor};
use candle_nn::{conv2d, conv2d_no_bias, linear, Conv2d, Conv2dConfig, Linear, Module, VarBuilder};
use std::sync::Mutex;

use crate::game_logic::{self, BitIter, Bitboard};

// ── Residual Block ────────────────────────────────
struct ResBlock {
    conv1: Conv2d,
    conv2: Conv2d,
}

impl ResBlock {
    fn new(vb: VarBuilder, prefix: &str) -> candle_core::Result<Self> {
        let cfg = Conv2dConfig {
            padding: 1,
            ..Default::default()
        };
        let conv1 = conv2d(
            64,
            64,
            3,
            cfg,
            vb.clone().pp(&format!("{prefix}.conv1")),
        )?;
        let conv2 = conv2d_no_bias(
            64,
            64,
            3,
            cfg,
            vb.pp(&format!("{prefix}.conv2")),
        )?;
        Ok(Self { conv1, conv2 })
    }

    fn forward(&self, x: &Tensor) -> candle_core::Result<Tensor> {
        let y = self.conv1.forward(x)?.relu()?;
        let y = self.conv2.forward(&y)?;
        y.add(x)?.relu()
    }
}

// ── Full Model ───────────────────────────────────
pub struct OthelloModel {
    conv_in: Conv2d,
    res1: ResBlock,
    res2: ResBlock,
    res3: ResBlock,
    conv_out: Conv2d,
    fc: Linear,
    device: Device,
}

impl OthelloModel {

    /// Load model directly from byte data (for Android loading from APK assets)
    pub fn load_from_bytes(data: Vec<u8>) -> candle_core::Result<Self> {
        let device = Device::Cpu;
        let vb = VarBuilder::from_buffered_safetensors(data, DType::F32, &device)?;
        Self::build(vb, device)
    }

    fn build(vb: VarBuilder, device: Device) -> candle_core::Result<Self> {
        let cfg = Conv2dConfig {
            padding: 1,
            ..Default::default()
        };

        let conv_in = conv2d(2, 64, 3, cfg, vb.clone().pp("conv_in"))?;
        let res1 = ResBlock::new(vb.clone(), "res1")?;
        let res2 = ResBlock::new(vb.clone(), "res2")?;
        let res3 = ResBlock::new(vb.clone(), "res3")?;
        let conv_out = conv2d(
            64,
            1,
            1,
            Default::default(),
            vb.clone().pp("conv_out"),
        )?;
        let fc = linear(64, 1, vb.pp("fc"))?;

        Ok(Self {
            conv_in,
            res1,
            res2,
            res3,
            conv_out,
            fc,
            device,
        })
    }

    /// Forward pass: evaluates the board, returns f32 ∈ [-1, 1]
    fn forward(&self, black: Bitboard, white: Bitboard) -> candle_core::Result<f32> {
        let x = bitboards_to_tensor(black, white, &self.device)?;
        let x = self.conv_in.forward(&x)?.relu()?;
        let x = self.res1.forward(&x)?;
        let x = self.res2.forward(&x)?;
        let x = self.res3.forward(&x)?;
        let x = self.conv_out.forward(&x)?.flatten(1, 3)?; // [1, 1, 8, 8] → [1, 64]
        let x = self.fc.forward(&x)?.tanh()?;
        // Extract scalar value
        let values = x.to_vec1::<f32>()?;
        Ok(values[0])
    }

    /// Board evaluation (public API)
    pub fn evaluate(&self, black: Bitboard, white: Bitboard) -> f32 {
        self.forward(black, white).unwrap_or(0.0)
    }

    /// AI best move: iterates all legal moves and picks the one with highest evaluation score
    pub fn best_move(&self, black: Bitboard, white: Bitboard, is_black: bool) -> Option<u32> {
        let (player, opponent) = if is_black {
            (black, white)
        } else {
            (white, black)
        };

        let legal = game_logic::compute_legal_moves(player, opponent);
        if legal == 0 {
            return None;
        }

        let score_sign: f32 = if is_black { 1.0 } else { -1.0 };

        BitIter(legal)
            .map(|pos_idx| {
                let pos = 1u64 << pos_idx;
                let (mut sim_black, mut sim_white) = (black, white);
                if is_black {
                    game_logic::make_move(&mut sim_black, &mut sim_white, pos);
                } else {
                    game_logic::make_move(&mut sim_white, &mut sim_black, pos);
                }
                let score = self.evaluate(sim_black, sim_white) * score_sign;
                (pos_idx, score)
            })
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Less))
            .map(|(pos_idx, _)| pos_idx)
    }
}

// ── Bitboard → Tensor Conversion ──────────────────
/// Convert two u64 bitboards into a [1, 2, 8, 8] tensor
/// Bit index: bit 0=a1(LSB), bit 63=h8(MSB)
/// Tensor layout: channel 0=black, channel 1=white, dim 2=rows(rank1→8), dim 3=cols(a→h)
fn bitboards_to_tensor(black: u64, white: u64, device: &Device) -> candle_core::Result<Tensor> {
    let data: Vec<f32> = (0..64)
        .map(|i| ((black >> i) & 1) as f32)
        .chain((0..64).map(|i| ((white >> i) & 1) as f32))
        .collect();
    Tensor::from_vec(data, (1, 2, 8, 8), device)
}

// ── Tauri Managed State ───────────────────────────
pub type AiState = Mutex<Option<OthelloModel>>;

// ── Unit Tests ───────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_logic;

    #[test]
    fn test_bitboards_to_tensor_shape() {
        let device = Device::Cpu;
        let (black, white) = game_logic::initial_board();
        let tensor = bitboards_to_tensor(black, white, &device).unwrap();
        let dims = tensor.dims();
        assert_eq!(dims.len(), 4);
        assert_eq!(dims[0], 1); // batch
        assert_eq!(dims[1], 2); // channels (black + white)
        assert_eq!(dims[2], 8); // rows
        assert_eq!(dims[3], 8); // cols
    }

    #[test]
    fn test_bitboards_to_tensor_initial_board() {
        let device = Device::Cpu;
        let (black, white) = game_logic::initial_board();
        let tensor = bitboards_to_tensor(black, white, &device).unwrap();
        let flat: Vec<f32> = tensor.flatten_all().unwrap().to_vec1().unwrap();

        // Total elements: 1 * 2 * 8 * 8 = 128
        assert_eq!(flat.len(), 128);

        // Check that the black channel has pieces at d5(35) and e4(28)
        // In row-major (rank1→8, file a→h), index into channel 0:
        // d5 = row 4 * 8 + col 3 = 35
        // e4 = row 3 * 8 + col 4 = 28
        assert_eq!(flat[35], 1.0); // black piece at d5
        assert_eq!(flat[28], 1.0); // black piece at e4

        // White channel starts at offset 64
        // d4 = row 3 * 8 + col 3 = 27
        // e5 = row 4 * 8 + col 4 = 36
        assert_eq!(flat[64 + 27], 1.0); // white piece at d4
        assert_eq!(flat[64 + 36], 1.0); // white piece at e5

        // Empty squares should be 0
        assert_eq!(flat[0], 0.0);  // a1
        assert_eq!(flat[63], 0.0); // h8
    }

    #[test]
    fn test_bitboards_to_tensor_empty_board() {
        let device = Device::Cpu;
        let tensor = bitboards_to_tensor(0, 0, &device).unwrap();
        let flat: Vec<f32> = tensor.flatten_all().unwrap().to_vec1().unwrap();
        assert_eq!(flat.len(), 128);
        assert!(flat.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn test_bitboards_to_tensor_full_board() {
        let device = Device::Cpu;
        let tensor = bitboards_to_tensor(u64::MAX, 0, &device).unwrap();
        let flat: Vec<f32> = tensor.flatten_all().unwrap().to_vec1().unwrap();
        // First 64 should all be 1.0 (black), last 64 all 0.0 (white)
        assert!(flat[0..64].iter().all(|&v| v == 1.0));
        assert!(flat[64..128].iter().all(|&v| v == 0.0));
    }
}
