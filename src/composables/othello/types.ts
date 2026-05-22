// ── Type Definitions ────────────────────────────────
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

export interface FlipAnimation {
  progress: number;
  flipBits: bigint;
  flipFrom: "black" | "white";
  newPieceSide: "black" | "white";
  newPieceIdx: number;
  preBlack: bigint;
  preWhite: bigint;
}

/** Move record for replay */
export interface MoveRecord {
  pos_index: number;
  is_black_turn: boolean;
}
