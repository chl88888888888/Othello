import { ref, onMounted, onUnmounted, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";

// ── Sub-modules ──────────────────────────────────
import type { GameState, FlipAnimation, MoveRecord } from "./othello/types";
import { bb, canvasToBitIndex } from "./othello/helpers";
import { drawBoard as renderBoard } from "./othello/renderer";
import type { DrawBoardOptions } from "./othello/renderer";
import { runFlipAnimation, runReplayAnimation } from "./othello/animation";

// ═══════════════════════════════════════════════════
//  Composable
// ═══════════════════════════════════════════════════

export function useOthello() {
  // ── Reactive State ──
  const black = ref("0");
  const white = ref("0");
  const legalMoves = ref("0");
  const currentTurn = ref("black");
  const gameOver = ref(false);
  const blackScore = ref(0);
  const whiteScore = ref(0);
  const winner = ref<string | null>(null);
  const errorMsg = ref("");

  // ── Animation State ──
  const flipAnim = ref<FlipAnimation | null>(null);
  let cancelAnim: (() => void) | null = null;

  // ── Replay State ──
  const isReplaying = ref(false);
  const replayingGameId = ref<number | null>(null);
  let replayTimerId: ReturnType<typeof setTimeout> | null = null;

  // ── Move history (for saving games) ──
  const moveHistory: MoveRecord[] = [];
  let currentGameSaved = false;

  const canvasRef = ref<HTMLCanvasElement | null>(null);

  // ── Build DrawBoardOptions ──
  function buildDrawOptions(): DrawBoardOptions {
    const anim = flipAnim.value;
    return {
      blackBits: anim ? anim.preBlack : bb(black.value),
      whiteBits: anim ? anim.preWhite : bb(white.value),
      legalMoveBits: anim ? 0n : bb(legalMoves.value),
      currentTurn: currentTurn.value as "black" | "white",
      gameOver: gameOver.value,
      anim,
    };
  }

  // ── Board Drawing ──
  function drawBoard() {
    const canvas = canvasRef.value;
    if (!canvas) return;
    renderBoard(canvas, buildDrawOptions());
  }

  // ── Start Flip Animation ──
  function startFlipAnimation(animData: FlipAnimation, finalState: GameState) {
    // Cancel previous animation
    if (cancelAnim) cancelAnim();

    flipAnim.value = animData;
    drawBoard();

    cancelAnim = runFlipAnimation(
      animData,
      finalState,
      drawBoard,
      (s) => {
        flipAnim.value = null;
        cancelAnim = null;
        applyState(s);
      },
    );
  }

  // ── Apply State ──
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

  // ── Start Game ──
  async function startGame() {
    if (cancelAnim) {
      cancelAnim();
      cancelAnim = null;
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

  // ── Board Click Handler ──
  async function handleClick(e: MouseEvent) {
    if (gameOver.value || flipAnim.value) return;
    const canvas = canvasRef.value;
    if (!canvas) return;

    const bitIndex = canvasToBitIndex(canvas, e);
    if (bitIndex < 0) return;

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

      moveHistory.push({
        pos_index: bitIndex,
        is_black_turn: currentTurn.value === "black",
      });

      const flipsBB = bb(res.flips);
      if (flipsBB === 0n) {
        applyState(res);
        return;
      }

      const animData: FlipAnimation = {
        progress: 0,
        flipBits: flipsBB,
        flipFrom,
        newPieceSide,
        newPieceIdx: bitIndex,
        preBlack,
        preWhite,
      };

      startFlipAnimation(animData, res);
    } catch (e) {
      errorMsg.value = `落子失败: ${e}`;
    }
  }

  // ── AI Move ──
  const isAiThinking = ref(false);
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

      if (res.ai_move_index === null || res.ai_move_index === undefined) {
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

      const animData: FlipAnimation = {
        progress: 0,
        flipBits: flipsBB,
        flipFrom,
        newPieceSide: aiSide,
        newPieceIdx: res.ai_move_index,
        preBlack,
        preWhite,
      };

      startFlipAnimation(animData, res);
      isAiThinking.value = false;
      return true;
    } catch (e) {
      errorMsg.value = `AI 落子失败: ${e}`;
      isAiThinking.value = false;
      return false;
    }
  }

  // ── Replay Feature ──
  async function replayGame(moves: MoveRecord[], gameId: number) {
    stopReplay();

    await startGame();
    isReplaying.value = true;
    replayingGameId.value = gameId;

    let moveIndex = 0;

    async function playNext() {
      if (!isReplaying.value) return;
      if (moveIndex >= moves.length) {
        stopReplay();
        return;
      }
      if (gameOver.value) {
        stopReplay();
        return;
      }

      const m = moves[moveIndex];
      moveIndex++;

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
          const animData: FlipAnimation = {
            progress: 0,
            flipBits: flipsBB,
            flipFrom,
            newPieceSide,
            newPieceIdx: m.pos_index,
            preBlack,
            preWhite,
          };

          flipAnim.value = animData;
          drawBoard();

          await runReplayAnimation(
            animData,
            res,
            drawBoard,
            (s) => {
              flipAnim.value = null;
              applyState(s);
            },
            () => isReplaying.value,
          );
        }

        if (isReplaying.value && !gameOver.value) {
          replayTimerId = setTimeout(() => playNext(), 600);
        }
      } catch (e) {
        errorMsg.value = `回放出错: ${e}`;
        stopReplay();
      }
    }

    replayTimerId = setTimeout(() => playNext(), 500);
  }

  function stopReplay() {
    isReplaying.value = false;
    replayingGameId.value = null;
    if (replayTimerId !== null) {
      clearTimeout(replayTimerId);
      replayTimerId = null;
    }
    if (cancelAnim) {
      cancelAnim();
      cancelAnim = null;
    }
    flipAnim.value = null;
  }

  // ── Save Current Game ──
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

  // ── Online Battle: Apply an arbitrary move ──
  async function applyMove(
    blackStr: string,
    whiteStr: string,
    posIndex: number,
    isBlackTurn: boolean,
  ): Promise<GameState | null> {
    if (gameOver.value || flipAnim.value) return null;

    const preBlack = bb(blackStr);
    const preWhite = bb(whiteStr);
    const flipFrom: "black" | "white" = isBlackTurn ? "white" : "black";
    const newPieceSide: "black" | "white" = isBlackTurn ? "black" : "white";

    try {
      const res = await invoke<GameState>("make_move", {
        black: blackStr,
        white: whiteStr,
        posIndex,
        isBlackTurn,
      });

      const flipsBB = bb(res.flips);
      if (flipsBB === 0n) {
        applyState(res);
        return res;
      }

      const animData: FlipAnimation = {
        progress: 0,
        flipBits: flipsBB,
        flipFrom,
        newPieceSide,
        newPieceIdx: posIndex,
        preBlack,
        preWhite,
      };

      return new Promise((resolve) => {
        flipAnim.value = animData;
        drawBoard();

        cancelAnim = runFlipAnimation(
          animData,
          res,
          drawBoard,
          (s) => {
            flipAnim.value = null;
            cancelAnim = null;
            applyState(s);
            resolve(s);
          },
        );
      });
    } catch (e) {
      errorMsg.value = `落子失败: ${e}`;
      return null;
    }
  }

  // ── UI Helpers ──
  function turnLabel(): string {
    return currentTurn.value === "black" ? "⚫ 黑方" : "⚪ 白方";
  }

  function winnerLabel(): string {
    if (winner.value === "black") return "黑方胜利！";
    if (winner.value === "white") return "白方胜利！";
    return "平局";
  }

  // ── Lifecycle ──
  onMounted(async () => {
    await nextTick();
    drawBoard();
    await startGame();
  });

  onUnmounted(() => {
    stopReplay();
  });

  // ── Exports ──
  return {
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
    startGame,
    handleClick,
    requestAiMove,
    applyMove,
    applyState,
    turnLabel,
    winnerLabel,
    replayGame,
    stopReplay,
    saveCurrentGame,
    drawBoard,
  };
}

// ═══════════════════════════════════════════════════
//  Unified exports (keep original import paths unchanged)
// ═══════════════════════════════════════════════════

export type { GameState, FlipAnimation, MoveRecord } from "./othello/types";
export { CELL_SIZE, PADDING, BOARD_PX } from "./othello/constants";
export { bb, canvasToBitIndex, hasBit, cellBitIndex } from "./othello/helpers";
