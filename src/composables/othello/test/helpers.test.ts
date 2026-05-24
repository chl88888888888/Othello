/**
 * Tests for src/composables/othello/helpers.ts — pure utility functions.
 * No DOM or Tauri dependency needed.
 */

import { describe, it, expect } from "vitest";
import {
  bb,
  hasBit,
  cellBitIndex,
  easeInOutCubic,
  easeOutBack,
  rgbScale,
} from "../helpers";

// ═══════════════════════════════════════════════════
// bb()
// ═══════════════════════════════════════════════════

describe("bb", () => {
  it("converts a numeric string to bigint", () => {
    expect(bb("42")).toBe(42n);
  });

  it("converts large u64 value correctly", () => {
    // 2^60
    const val = (1n << 60n).toString();
    expect(bb(val)).toBe(1n << 60n);
  });

  it("returns 0n for empty string", () => {
    expect(bb("")).toBe(0n);
  });

  it("returns 0n for invalid string", () => {
    expect(bb("not-a-number")).toBe(0n);
  });

  it("returns 0n for whitespace string", () => {
    expect(bb("   ")).toBe(0n);
  });

  it("handles '0' string", () => {
    expect(bb("0")).toBe(0n);
  });
});

// ═══════════════════════════════════════════════════
// hasBit()
// ═══════════════════════════════════════════════════

describe("hasBit", () => {
  it("returns true when the bit at index is set", () => {
    // bit 3 set → value = 8
    expect(hasBit("8", 3)).toBe(true);
  });

  it("returns false when the bit at index is not set", () => {
    expect(hasBit("8", 0)).toBe(false);
  });

  it("returns false for empty bitboard string", () => {
    expect(hasBit("0", 5)).toBe(false);
  });

  it("handles bit index 0 (LSB)", () => {
    expect(hasBit("1", 0)).toBe(true);
    expect(hasBit("2", 0)).toBe(false);
  });

  it("handles bit index 63 (MSB)", () => {
    const msb = (1n << 63n).toString();
    expect(hasBit(msb, 63)).toBe(true);
    expect(hasBit(msb, 0)).toBe(false);
  });
});

// ═══════════════════════════════════════════════════
// cellBitIndex()
// ═══════════════════════════════════════════════════

describe("cellBitIndex", () => {
  it("row=0 col=0 is the top-left corner a8 → bit index 56", () => {
    expect(cellBitIndex(0, 0)).toBe(56);
  });

  it("row=7 col=7 is the bottom-right corner h1 → bit index 7", () => {
    expect(cellBitIndex(7, 7)).toBe(7);
  });

  it("row=7 col=0 is a1 → bit index 0", () => {
    expect(cellBitIndex(7, 0)).toBe(0);
  });

  it("row=0 col=7 is h8 → bit index 63", () => {
    expect(cellBitIndex(0, 7)).toBe(63);
  });

  it("initial black piece d5 is row=3 col=3 → bit index 35", () => {
    // d is col 3, rank 5 → row 7-4=3
    expect(cellBitIndex(3, 3)).toBe(35);
  });

  it("initial white piece e4 is row=4 col=4 → bit index 28", () => {
    expect(cellBitIndex(4, 4)).toBe(28);
  });
});

// ═══════════════════════════════════════════════════
// easeInOutCubic()
// ═══════════════════════════════════════════════════

describe("easeInOutCubic", () => {
  it("starts at 0", () => {
    expect(easeInOutCubic(0)).toBe(0);
  });

  it("ends at 1", () => {
    expect(easeInOutCubic(1)).toBe(1);
  });

  it("is monotonic", () => {
    const values = [0, 0.1, 0.25, 0.5, 0.75, 0.9, 1.0];
    for (let i = 1; i < values.length; i++) {
      expect(easeInOutCubic(values[i])).toBeGreaterThanOrEqual(
        easeInOutCubic(values[i - 1]),
      );
    }
  });
});

// ═══════════════════════════════════════════════════
// easeOutBack()
// ═══════════════════════════════════════════════════

describe("easeOutBack", () => {
  it("starts near 0", () => {
    expect(easeOutBack(0)).toBeCloseTo(0, 1);
  });

  it("ends at 1", () => {
    expect(easeOutBack(1)).toBe(1);
  });

  it("overshoots above 1 in the middle (back easing characteristic)", () => {
    const mid = easeOutBack(0.7);
    // easeOutBack typically overshoots past 1 before settling
    expect(mid).toBeGreaterThan(1);
  });
});

// ═══════════════════════════════════════════════════
// rgbScale()
// ═══════════════════════════════════════════════════

describe("rgbScale", () => {
  it("returns black when scaled to 0", () => {
    expect(rgbScale(100, 200, 50, 0)).toBe("rgb(0,0,0)");
  });

  it("returns original at scale 1", () => {
    expect(rgbScale(100, 200, 50, 1)).toBe("rgb(100,200,50)");
  });

  it("scales proportionally", () => {
    expect(rgbScale(100, 100, 100, 0.5)).toBe("rgb(50,50,50)");
  });

  it("rounds values", () => {
    expect(rgbScale(255, 128, 64, 0.333)).toBe("rgb(85,43,21)");
  });
});
