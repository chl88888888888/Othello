// ── BigInt 辅助 ──────────────────────────────────

/**
 * 安全地将字符串转换为 bigint，失败时返回 0n
 */
export function bb(s: string): bigint {
  try {
    return BigInt(s);
  } catch {
    return 0n;
  }
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

export function easeInOutCubic(t: number): number {
  return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
}

export function easeOutBack(t: number): number {
  const c1 = 1.70158;
  const c3 = c1 + 1;
  return 1 + c3 * Math.pow(t - 1, 3) + c1 * Math.pow(t - 1, 2);
}

// ── 颜色辅助 ──────────────────────────────────────

export function rgbScale(r: number, g: number, b: number, s: number): string {
  const rr = Math.round(r * s);
  const gg = Math.round(g * s);
  const bb_ = Math.round(b * s);
  return `rgb(${rr},${gg},${bb_})`;
}
