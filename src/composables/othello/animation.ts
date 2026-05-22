import type { FlipAnimation, GameState } from "./types";

/**
 * Run a flip animation.
 *
 * @param animData    - Animation data (progress will be updated in-place)
 * @param finalState  - The final state to apply after animation completes
 * @param drawFn      - Draw function called on each frame
 * @param applyFn     - State apply function called when animation finishes
 * @returns Cancel function; calling it aborts the animation
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
 * Promise-based replay animation.
 * Returns a Promise that resolves when animation completes.
 * Uses `isActive` function to check whether replay is still active.
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
