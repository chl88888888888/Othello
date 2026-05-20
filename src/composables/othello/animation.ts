import type { FlipAnimation, GameState } from "./types";

/**
 * 运行翻转动画。
 *
 * @param animData    - 动画数据（会原地更新 progress）
 * @param finalState  - 动画完成后要应用的最终状态
 * @param drawFn      - 每次帧调用的绘制函数
 * @param applyFn     - 动画完成时调用的状态应用函数
 * @returns 取消函数，调用后可中止动画
 */
export function runFlipAnimation(
  animData: FlipAnimation,
  finalState: GameState,
  drawFn: () => void,
  applyFn: (state: GameState) => void,
): () => void {
  const duration = 420;
  const startTime = performance.now();
  let animFrameId: number | null = null;
  let cancelled = false;

  function frame(now: number) {
    if (cancelled) return;
    const elapsed = now - startTime;
    const progress = Math.min(elapsed / duration, 1);

    animData.progress = progress;
    drawFn();

    if (progress < 1) {
      animFrameId = requestAnimationFrame(frame);
    } else {
      animFrameId = null;
      applyFn(finalState);
    }
  }

  animFrameId = requestAnimationFrame(frame);

  return () => {
    cancelled = true;
    if (animFrameId !== null) {
      cancelAnimationFrame(animFrameId);
      animFrameId = null;
    }
  };
}

/**
 * 创建回放专用的动画 Promise 版本。
 * 返回一个 Promise，在动画完成后 resolve。
 * 通过 `isActive` 函数判断回放是否仍在进行。
 */
export function runReplayAnimation(
  animData: FlipAnimation,
  finalState: GameState,
  drawFn: () => void,
  applyFn: (state: GameState) => void,
  isActive: () => boolean,
): Promise<void> {
  return new Promise<void>((resolve) => {
    const duration = 420;
    const startTime = performance.now();
    let animFrameId: number | null = null;

    function frame(now: number) {
      if (!isActive()) {
        if (animFrameId !== null) cancelAnimationFrame(animFrameId);
        resolve();
        return;
      }
      const elapsed = now - startTime;
      const progress = Math.min(elapsed / duration, 1);

      animData.progress = progress;
      drawFn();

      if (progress < 1) {
        animFrameId = requestAnimationFrame(frame);
      } else {
        animFrameId = null;
        applyFn(finalState);
        resolve();
      }
    }

    animFrameId = requestAnimationFrame(frame);
  });
}
