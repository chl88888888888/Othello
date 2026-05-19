import { ref, onMounted, onUnmounted, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";

// ── 类型 ──────────────────────────────────────────
export interface GameState {
  black: string;
  white: string;
  legal_moves: string;
  current_turn: string;
  game_over: boolean;
  black_score: number;
  white_score: number;
  winner: string | null;
  flips: string;
  ai_move_index: number | null;
}

interface FlipAnimation {
  progress: number;
  flipBits: bigint;
  flipFrom: "black" | "white";
  newPieceSide: "black" | "white";
  newPieceIdx: number;
  preBlack: bigint;
  preWhite: bigint;
}

// ── Canvas 常量 ───────────────────────────────────
const CELL_SIZE = 56;
const PADDING = 4;
export const BOARD_PX = CELL_SIZE * 8 + PADDING * 2;

// ── BigInt 辅助 ──────────────────────────────────
function bb(s: string): bigint {
  try {
    return BigInt(s);
  } catch {
    return 0n;
  }
}

/** 回放用的走法记录 */
export interface MoveRecord {
  pos_index: number;
  is_black_turn: boolean;
}

/** 判断 bitboard 中第 index 位（0=a1 … 63=h8）是否为 1 */
export function hasBit(bits: string, index: number): boolean {
  return (bb(bits) & (1n << BigInt(index))) !== 0n;
}

/** Canvas (row,col) → bit 索引。row=0 是棋盘顶部（rank 8），row=7 是底部（rank 1） */
export function cellBitIndex(row: number, col: number): number {
  return (7 - row) * 8 + col;
}

// ── 缓动函数 ──────────────────────────────────────
function easeInOutCubic(t: number): number {
  return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
}

function easeOutBack(t: number): number {
  const c1 = 1.70158;
  const c3 = c1 + 1;
  return 1 + c3 * Math.pow(t - 1, 3) + c1 * Math.pow(t - 1, 2);
}

function rgbScale(r: number, g: number, b: number, s: number): string {
  const rr = Math.round(r * s);
  const gg = Math.round(g * s);
  const bb = Math.round(b * s);
  return `rgb(${rr},${gg},${bb})`;
}

// ── 绘制函数 ──────────────────────────────────────
function drawPiece(
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
function drawPieceAt(
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
function drawFlippingPiece(
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
function drawNewPiecePop(
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

// ═══════════════════════════════════════════════════
//  Composable
// ═══════════════════════════════════════════════════

export function useOthello() {
  // ── 响应式状态 ──
  const black = ref("0");
  const white = ref("0");
  const legalMoves = ref("0");
  const currentTurn = ref("black");
  const gameOver = ref(false);
  const blackScore = ref(0);
  const whiteScore = ref(0);
  const winner = ref<string | null>(null);
  const errorMsg = ref("");

  // ── 动画状态 ──
  const flipAnim = ref<FlipAnimation | null>(null);
  let animFrameId: number | null = null;

  // ── 回放状态 ──
  const isReplaying = ref(false);
  const replayingGameId = ref<number | null>(null);
  let replayTimerId: ReturnType<typeof setTimeout> | null = null;

  // ── 走法记录（用于保存对局）──
  const moveHistory: MoveRecord[] = [];
  let currentGameSaved = false;

  const canvasRef = ref<HTMLCanvasElement | null>(null);

  // ── 棋盘绘制 ──
  function drawBoard() {
    const canvas = canvasRef.value;
    if (!canvas) return;

    const dpr = window.devicePixelRatio || 1;
    canvas.width = BOARD_PX * dpr;
    canvas.height = BOARD_PX * dpr;
    canvas.style.width = BOARD_PX + "px";
    canvas.style.height = BOARD_PX + "px";

    const ctx = canvas.getContext("2d")!;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

    const anim = flipAnim.value;

    const b = anim ? anim.preBlack : bb(black.value);
    const w = anim ? anim.preWhite : bb(white.value);
    const lm = anim ? 0n : bb(legalMoves.value);

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
          } else if (!gameOver.value && ((lm >> BigInt(idx)) & 1n)) {
            const alpha = currentTurn.value === "black" ? 0.3 : 0.45;
            const color = currentTurn.value === "black" ? "0,0,0" : "255,255,255";
            ctx.beginPath();
            ctx.arc(cx, cy, r * 0.28, 0, Math.PI * 2);
            ctx.fillStyle = `rgba(${color},${alpha})`;
            ctx.fill();
          }
        }
      }
    }
  }

  // ── 动画循环 ──
  function startFlipAnimation(animData: FlipAnimation, finalState: GameState) {
    const duration = 420;
    const startTime = performance.now();

    function frame(now: number) {
      const elapsed = now - startTime;
      const progress = Math.min(elapsed / duration, 1);

      animData.progress = progress;
      drawBoard();

      if (progress < 1) {
        animFrameId = requestAnimationFrame(frame);
      } else {
        flipAnim.value = null;
        animFrameId = null;
        applyState(finalState);
      }
    }

    animFrameId = requestAnimationFrame(frame);
  }

  // ── 应用状态 ──
  function applyState(s: GameState) {
    black.value = s.black;
    white.value = s.white;
    legalMoves.value = s.legal_moves;
    currentTurn.value = s.current_turn;
    gameOver.value = s.game_over;
    blackScore.value = s.black_score;
    whiteScore.value = s.white_score;
    winner.value = s.winner;
    errorMsg.value = "";
    drawBoard();
  }

  // ── Tauri 调用 ──
  async function startGame() {
    if (animFrameId !== null) {
      cancelAnimationFrame(animFrameId);
      animFrameId = null;
    }
    flipAnim.value = null;
    moveHistory.length = 0;
    currentGameSaved = false;

    try {
      const res = await invoke<GameState>("start_game");
      applyState(res);
    } catch (e) {
      errorMsg.value = `启动游戏失败: ${e}`;
    }
  }

  async function handleClick(e: MouseEvent) {
    if (gameOver.value || flipAnim.value) return;
    const canvas = canvasRef.value;
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    const scaleX = canvas.width / (rect.width * (window.devicePixelRatio || 1));
    const scaleY = canvas.height / (rect.height * (window.devicePixelRatio || 1));
    const x = (e.clientX - rect.left) * scaleX;
    const y = (e.clientY - rect.top) * scaleY;

    const col = Math.floor((x - PADDING) / CELL_SIZE);
    const row = Math.floor((y - PADDING) / CELL_SIZE);
    if (col < 0 || col > 7 || row < 0 || row > 7) return;

    const bitIndex = cellBitIndex(row, col);

    const preBlack = bb(black.value);
    const preWhite = bb(white.value);
    const flipFrom: "black" | "white" = currentTurn.value === "black" ? "white" : "black";
    const newPieceSide: "black" | "white" = currentTurn.value as "black" | "white";

    try {
      const res = await invoke<GameState>("make_move", {
        black: black.value,
        white: white.value,
        posIndex: bitIndex,
        isBlackTurn: currentTurn.value === "black",
      });

      // 记录走法
      moveHistory.push({
        pos_index: bitIndex,
        is_black_turn: currentTurn.value === "black",
      });

      const flipsBB = bb(res.flips);
      if (flipsBB === 0n) {
        applyState(res);
        return;
      }

      flipAnim.value = {
        progress: 0,
        flipBits: flipsBB,
        flipFrom,
        newPieceSide,
        newPieceIdx: bitIndex,
        preBlack,
        preWhite,
      };

      drawBoard();
      startFlipAnimation(flipAnim.value, res);
    } catch (e) {
      errorMsg.value = `落子失败: ${e}`;
    }
  }

  // ── AI 落子 ──
  const isAiThinking = ref(false);
  /** 当前人类玩家执子方 */
  const playerSide = ref<"black" | "white">("black");

  async function requestAiMove(): Promise<boolean> {
    if (gameOver.value || flipAnim.value || isAiThinking.value) return false;

    isAiThinking.value = true;
    errorMsg.value = "";

    const preBlack = bb(black.value);
    const preWhite = bb(white.value);
    const aiSide: "black" | "white" =
      currentTurn.value === "black" ? "black" : "white";
    const flipFrom: "black" | "white" = aiSide === "black" ? "white" : "black";

    try {
      const res = await invoke<GameState>("ai_move", {
        black: black.value,
        white: white.value,
        isBlackTurn: currentTurn.value === "black",
      });

      // AI 无合法落子（pass）或正常落子
      if (res.ai_move_index === null || res.ai_move_index === undefined) {
        // AI pass，直接应用状态（无翻转动画）
        applyState(res);
        isAiThinking.value = false;
        return true;
      }

      moveHistory.push({
        pos_index: res.ai_move_index,
        is_black_turn: currentTurn.value === "black",
      });

      const flipsBB = bb(res.flips);
      if (flipsBB === 0n) {
        applyState(res);
        isAiThinking.value = false;
        return true;
      }

      flipAnim.value = {
        progress: 0,
        flipBits: flipsBB,
        flipFrom,
        newPieceSide: aiSide,
        newPieceIdx: res.ai_move_index,
        preBlack,
        preWhite,
      };

      drawBoard();
      startFlipAnimation(flipAnim.value, res);
      isAiThinking.value = false;
      return true;
    } catch (e) {
      errorMsg.value = `AI 落子失败: ${e}`;
      isAiThinking.value = false;
      return false;
    }
  }

  // ── 回放功能 ──
  async function replayGame(moves: MoveRecord[], gameId: number) {
    // 停止之前的回放
    stopReplay();

    // 先重置棋盘
    await startGame();
    isReplaying.value = true;
    replayingGameId.value = gameId;

    let moveIndex = 0;

    async function playNext() {
      if (!isReplaying.value) return;
      if (moveIndex >= moves.length) {
        // 回放完毕
        stopReplay();
        return;
      }
      if (gameOver.value) {
        stopReplay();
        return;
      }

      const m = moves[moveIndex];
      moveIndex++;

      // 检查是否需要跳过（当前回合无人可走时会自动跳过，但这里直接按记录走）
      const preBlack = bb(black.value);
      const preWhite = bb(white.value);
      const flipFrom: "black" | "white" = m.is_black_turn ? "white" : "black";
      const newPieceSide: "black" | "white" = m.is_black_turn ? "black" : "white";

      try {
        const res = await invoke<GameState>("make_move", {
          black: black.value,
          white: white.value,
          posIndex: m.pos_index,
          isBlackTurn: m.is_black_turn,
        });

        const flipsBB = bb(res.flips);
        if (flipsBB === 0n) {
          applyState(res);
        } else {
          flipAnim.value = {
            progress: 0,
            flipBits: flipsBB,
            flipFrom,
            newPieceSide,
            newPieceIdx: m.pos_index,
            preBlack,
            preWhite,
          };
          drawBoard();

          // 等待动画完成
          await new Promise<void>((resolve) => {
            const duration = 420;
            const startTime = performance.now();

            function frame(now: number) {
              if (!isReplaying.value) { resolve(); return; }
              const elapsed = now - startTime;
              const progress = Math.min(elapsed / duration, 1);
              flipAnim.value!.progress = progress;
              drawBoard();

              if (progress < 1) {
                animFrameId = requestAnimationFrame(frame);
              } else {
                flipAnim.value = null;
                animFrameId = null;
                applyState(res);
                resolve();
              }
            }
            animFrameId = requestAnimationFrame(frame);
          });
        }

        // 下一手延迟
        if (isReplaying.value && !gameOver.value) {
          replayTimerId = setTimeout(() => playNext(), 600);
        }
      } catch (e) {
        errorMsg.value = `回放出错: ${e}`;
        stopReplay();
      }
    }

    // 延迟开始第一手
    replayTimerId = setTimeout(() => playNext(), 500);
  }

  function stopReplay() {
    isReplaying.value = false;
    replayingGameId.value = null;
    if (replayTimerId !== null) {
      clearTimeout(replayTimerId);
      replayTimerId = null;
    }
    if (animFrameId !== null) {
      cancelAnimationFrame(animFrameId);
      animFrameId = null;
    }
    flipAnim.value = null;
  }

  // ── 保存当前对局 ──
  async function saveCurrentGame(): Promise<boolean> {
    if (currentGameSaved) return false;
    if (moveHistory.length === 0) return false;
    if (!gameOver.value) return false;

    currentGameSaved = true;
    try {
      await invoke("save_game", {
        blackScore: blackScore.value,
        whiteScore: whiteScore.value,
        winner: winner.value,
        moves: moveHistory,
      });
      return true;
    } catch (e) {
      console.error("保存对局失败:", e);
      currentGameSaved = false;
      return false;
    }
  }

  // ── UI 辅助 ──
  function turnLabel(): string {
    return currentTurn.value === "black" ? "⚫ 黑方" : "⚪ 白方";
  }

  function winnerLabel(): string {
    if (winner.value === "black") return "🏆 黑方胜利！";
    if (winner.value === "white") return "🏆 白方胜利！";
    return "🤝 平局";
  }

  // ── 生命周期 ──
  onMounted(async () => {
    await nextTick();
    drawBoard();
    await startGame();
  });

  onUnmounted(() => {
    stopReplay();
  });

  // ── 导出 ──
  return {
    // 状态
    black,
    white,
    legalMoves,
    currentTurn,
    gameOver,
    blackScore,
    whiteScore,
    winner,
    errorMsg,
    flipAnim,
    canvasRef,
    isReplaying,
    replayingGameId,
    isAiThinking,
    playerSide,
    // 方法
    startGame,
    handleClick,
    requestAiMove,
    turnLabel,
    winnerLabel,
    replayGame,
    stopReplay,
    saveCurrentGame,
    drawBoard,
  };
}
