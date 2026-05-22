// ── BigInt Helpers ───────────────────────────────

/**
 * Safely convert a string to bigint, returning 0n on failure
 */
export function bb(s: string): bigint {
  try {
    return BigInt(s);
  } catch {
    return 0n;
  }
}

/** Check if the bit at `index` (0=a1 … 63=h8) of the bitboard is 1 */
export function hasBit(bits: string, index: number): boolean {
  return (bb(bits) & (1n << BigInt(index))) !== 0n;
}

/** Canvas (row,col) → bit index. row=0 is top of board (rank 8), row=7 is bottom (rank 1) */
export function cellBitIndex(row: number, col: number): number {
  return (7 - row) * 8 + col;
}

// ── Easing Functions ─────────────────────────────

export function easeInOutCubic(t: number): number {
  return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
}

export function easeOutBack(t: number): number {
  const c1 = 1.70158;
  const c3 = c1 + 1;
  return 1 + c3 * Math.pow(t - 1, 3) + c1 * Math.pow(t - 1, 2);
}

// ── Color Helpers ────────────────────────────────

export function rgbScale(r: number, g: number, b: number, s: number): string {
  const rr = Math.round(r * s);
  const gg = Math.round(g * s);
  const bb_ = Math.round(b * s);
  return `rgb(${rr},${gg},${bb_})`;
}
