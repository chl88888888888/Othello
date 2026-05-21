import type { FlipAnimation } from "./types";
import { CELL_SIZE, PADDING, BOARD_PX } from "./constants";
import { cellBitIndex, easeInOutCubic, easeOutBack, rgbScale } from "./helpers";

// ── 基础棋子绘制 ───────────────────────────────────

export function drawPiece(
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  r: number,
  side: "black" | "white",
) {
  ctx.save();
  ctx.shadowColor = "rgba(0,0,0,0.35)";
  ctx.shadowBlur = 4;
  ctx.shadowOffsetX = 1;
  ctx.shadowOffsetY = 2;

  const grad = ctx.createRadialGradient(
    cx - r * 0.3,
    cy - r * 0.3,
    r * 0.05,
    cx,
    cy,
    r,
  );

  if (side === "black") {
    grad.addColorStop(0, "#4a4a4a");
    grad.addColorStop(0.7, "#1a1a1a");
    grad.addColorStop(1, "#050505");
  } else {
    grad.addColorStop(0, "#ffffff");
    grad.addColorStop(0.6, "#e8e8e8");
    grad.addColorStop(1, "#b0b0b0");
  }

  ctx.beginPath();
  ctx.arc(cx, cy, r, 0, Math.PI * 2);
  ctx.fillStyle = grad;
  ctx.fill();
  ctx.restore();
}

/** 在原点 (0,0) 画棋子（配合 translate 使用） */
export function drawPieceAt(
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  r: number,
  side: "black" | "white",
  brightness: number = 1,
) {
  ctx.save();
  ctx.shadowColor = "rgba(0,0,0,0.35)";
  ctx.shadowBlur = 4;
  ctx.shadowOffsetX = 1;
  ctx.shadowOffsetY = 2;

  const grad = ctx.createRadialGradient(
    cx - r * 0.3,
    cy - r * 0.3,
    r * 0.05,
    cx,
    cy,
    r,
  );

  if (side === "black") {
    grad.addColorStop(0, rgbScale(0x4a, 0x4a, 0x4a, brightness));
    grad.addColorStop(0.7, rgbScale(0x1a, 0x1a, 0x1a, brightness));
    grad.addColorStop(1, rgbScale(0x05, 0x05, 0x05, brightness));
  } else {
    grad.addColorStop(0, rgbScale(0xff, 0xff, 0xff, brightness));
    grad.addColorStop(0.6, rgbScale(0xe8, 0xe8, 0xe8, brightness));
    grad.addColorStop(1, rgbScale(0xb0, 0xb0, 0xb0, brightness));
  }

  ctx.beginPath();
  ctx.arc(cx, cy, r, 0, Math.PI * 2);
  ctx.fillStyle = grad;
  ctx.fill();
  ctx.restore();
}

/** 绘制翻转中的棋子：水平缩放模拟 3D 翻转 */
export function drawFlippingPiece(
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  r: number,
  flipFromBlack: boolean,
  progress: number,
) {
  const t = easeInOutCubic(progress);

  let scaleX: number;
  let side: "black" | "white";

  if (t < 0.5) {
    scaleX = 1 - 2 * t;
    side = flipFromBlack ? "black" : "white";
  } else {
    scaleX = 2 * t - 1;
    side = flipFromBlack ? "white" : "black";
  }

  ctx.save();
  ctx.translate(cx, cy);
  ctx.scale(scaleX, 1);

  const brightness = Math.abs(scaleX);
  drawPieceAt(ctx, 0, 0, r, side, brightness);
  ctx.restore();
}

/** 新落子弹入动画 */
export function drawNewPiecePop(
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  r: number,
  side: "black" | "white",
  progress: number,
) {
  const t = easeOutBack(progress);
  const scale = 0.3 + 0.7 * t;

  ctx.save();
  ctx.translate(cx, cy);
  ctx.scale(scale, scale);
  drawPieceAt(ctx, 0, 0, r, side);
  ctx.restore();
}

// ── 棋盘绘制参数 ───────────────────────────────────

export interface DrawBoardOptions {
  blackBits: bigint;
  whiteBits: bigint;
  legalMoveBits: bigint;
  currentTurn: "black" | "white";
  gameOver: boolean;
  anim: FlipAnimation | null;
}

/** 绘制整个棋盘（纯函数，不依赖任何外部状态） */
export function drawBoard(
  canvas: HTMLCanvasElement,
  options: DrawBoardOptions,
) {
  const { blackBits: b, whiteBits: w, legalMoveBits: lm, currentTurn, gameOver, anim } = options;

  const dpr = window.devicePixelRatio || 1;

  // 自适应屏幕宽度：棋盘最大 456px，但不超过视口宽度减去内边距
  const maxDisplayWidth = Math.min(window.innerWidth - 32, BOARD_PX);
  const displaySize = Math.floor(maxDisplayWidth);

  // Canvas 内部渲染保持全分辨率
  canvas.width = Math.floor(BOARD_PX * dpr);
  canvas.height = Math.floor(BOARD_PX * dpr);
  // CSS 显示尺寸自适应
  canvas.style.width = displaySize + "px";
  canvas.style.height = displaySize + "px";

  const ctx = canvas.getContext("2d")!;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

  // 棋盘底色
  ctx.fillStyle = "#1a6b3c";
  ctx.fillRect(0, 0, BOARD_PX, BOARD_PX);

  // 棋盘木纹背景
  ctx.fillStyle = "#1e7e46";
  ctx.fillRect(PADDING, PADDING, CELL_SIZE * 8, CELL_SIZE * 8);

  // 网格
  ctx.strokeStyle = "#0d3d1f";
  ctx.lineWidth = 1;
  for (let i = 0; i <= 8; i++) {
    const pos = PADDING + i * CELL_SIZE;
    ctx.beginPath();
    ctx.moveTo(pos, PADDING);
    ctx.lineTo(pos, PADDING + CELL_SIZE * 8);
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(PADDING, pos);
    ctx.lineTo(PADDING + CELL_SIZE * 8, pos);
    ctx.stroke();
  }

  // 星位标记
  const dots = [
    [2, 2],
    [2, 6],
    [6, 2],
    [6, 6],
  ];
  ctx.fillStyle = "#0d3d1f";
  for (const [dc, dr] of dots) {
    const dx = PADDING + dc * CELL_SIZE;
    const dy = PADDING + dr * CELL_SIZE;
    ctx.beginPath();
    ctx.arc(dx, dy, 3, 0, Math.PI * 2);
    ctx.fill();
  }

  // 逐格绘制棋子 / 提示
  for (let row = 0; row < 8; row++) {
    for (let col = 0; col < 8; col++) {
      const idx = cellBitIndex(row, col);
      const cx = PADDING + col * CELL_SIZE + CELL_SIZE / 2;
      const cy = PADDING + row * CELL_SIZE + CELL_SIZE / 2;
      const r = CELL_SIZE * 0.4;

      const isOldBlack = (b >> BigInt(idx)) & 1n;
      const isOldWhite = (w >> BigInt(idx)) & 1n;

      if (anim) {
        const isFlipping = (anim.flipBits >> BigInt(idx)) & 1n;
        const isNewPiece = idx === anim.newPieceIdx;
        const progress = anim.progress;
        const flipFromBlack = anim.flipFrom === "black";

        if (isFlipping) {
          drawFlippingPiece(ctx, cx, cy, r, flipFromBlack, progress);
        } else if (isNewPiece) {
          drawNewPiecePop(ctx, cx, cy, r, anim.newPieceSide, progress);
        } else if (isOldBlack) {
          drawPiece(ctx, cx, cy, r, "black");
        } else if (isOldWhite) {
          drawPiece(ctx, cx, cy, r, "white");
        }
      } else {
        if (isOldBlack) {
          drawPiece(ctx, cx, cy, r, "black");
        } else if (isOldWhite) {
          drawPiece(ctx, cx, cy, r, "white");
        } else if (!gameOver && ((lm >> BigInt(idx)) & 1n)) {
          const alpha = currentTurn === "black" ? 0.3 : 0.45;
          const color = currentTurn === "black" ? "0,0,0" : "255,255,255";
          ctx.beginPath();
          ctx.arc(cx, cy, r * 0.28, 0, Math.PI * 2);
          ctx.fillStyle = `rgba(${color},${alpha})`;
          ctx.fill();
        }
      }
    }
  }
}


