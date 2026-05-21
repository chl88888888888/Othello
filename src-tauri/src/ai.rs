//! AI 模块 — 加载训练好的 CNN 模型，进行局面评估与落子决策
//! 模型架构: AlphaZero 风格残差网络
//!   输入: 2×8×8 (黑白位棋盘)
//!   → Conv2d(2→64, 3×3, p1) → ReLU
//!   → ResBlock×3 (Conv2d 64→64, 3×3 + skip)
//!   → Conv2d(64→1, 1×1) → Flatten → Linear(64→1) → tanh

use candle_core::{Device, DType, Tensor};
use candle_nn::{conv2d, conv2d_no_bias, linear, Conv2d, Conv2dConfig, Linear, Module, VarBuilder};
use std::sync::Mutex;

use crate::game_logic::{self, BitIter, Bitboard};

// ── 残差块 ────────────────────────────────────────
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

// ── 完整模型 ──────────────────────────────────────
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

    /// 从字节数据直接加载模型（用于 Android 从 APK assets 加载）
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

    /// 前向传播：评估局面，返回 f32 ∈ [-1, 1]
    fn forward(&self, black: Bitboard, white: Bitboard) -> candle_core::Result<f32> {
        let x = bitboards_to_tensor(black, white, &self.device)?;
        let x = self.conv_in.forward(&x)?.relu()?;
        let x = self.res1.forward(&x)?;
        let x = self.res2.forward(&x)?;
        let x = self.res3.forward(&x)?;
        let x = self.conv_out.forward(&x)?.flatten(1, 3)?; // [1, 1, 8, 8] → [1, 64]
        let x = self.fc.forward(&x)?.tanh()?;
        // 提取标量值
        let values = x.to_vec1::<f32>()?;
        Ok(values[0])
    }

    /// 局面评估 (公开接口)
    pub fn evaluate(&self, black: Bitboard, white: Bitboard) -> f32 {
        self.forward(black, white).unwrap_or(0.0)
    }

    /// AI 最佳落子：遍历所有合法落子，选择评估分最高的走法
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

// ── 位棋盘 → Tensor 转换 ─────────────────────────
/// 将两个 u64 位棋盘转为 [1, 2, 8, 8] 张量
/// 位索引: bit 0=a1(LSB), bit 63=h8(MSB)
/// 张量布局: channel 0=黑棋, channel 1=白棋, dim 2=行(rank1→8), dim 3=列(a→h)
fn bitboards_to_tensor(black: u64, white: u64, device: &Device) -> candle_core::Result<Tensor> {
    let data: Vec<f32> = (0..64)
        .map(|i| ((black >> i) & 1) as f32)
        .chain((0..64).map(|i| ((white >> i) & 1) as f32))
        .collect();
    Tensor::from_vec(data, (1, 2, 8, 8), device)
}

// ── Tauri 托管状态 ───────────────────────────────
pub type AiState = Mutex<Option<OthelloModel>>;
