/**
 * Mock for @tauri-apps/api/core
 *
 * In tests, call `mockInvoke.mockResolvedValue(...)` or
 * `mockInvoke.mockRejectedValue(...)` before triggering the code under test.
 *
 * Shared singleton — each test should call `mockInvoke.mockClear()` in beforeEach.
 */

import { vi } from "vitest";

export const mockInvoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: mockInvoke,
}));
