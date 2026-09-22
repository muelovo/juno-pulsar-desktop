export type Phase =
  "idle" | "pressing" | "locked" | "target" | "confirming" | "hit";
export interface State {
  phase: Phase;
  since: number;
  target: string | null;
  hoverSince: number;
  wasDown: boolean;
}
export interface Input {
  now: number;
  down: boolean;
  cancel: boolean;
  over: boolean;
  target: string | null;
}
export const initial = (): State => ({
  phase: "idle",
  since: 0,
  target: null,
  hoverSince: 0,
  wasDown: false,
});
export function step(s: State, i: Input): State {
  const reset = { ...initial(), wasDown: i.down };
  if (i.cancel) return reset;
  if (s.phase === "confirming") return { ...s, wasDown: i.down };
  if (s.phase === "hit") return i.now - s.since > 650 ? reset : s;
  if (s.phase === "idle")
    return i.down && !s.wasDown && i.over
      ? { ...reset, phase: "pressing", since: i.now }
      : { ...s, wasDown: i.down };
  if (s.phase === "pressing") {
    if (!i.down || !i.over) return reset;
    return i.now - s.since >= 250
      ? { ...s, phase: "locked", since: i.now, wasDown: true }
      : { ...s, wasDown: true };
  }
  if (!i.down)
    return s.phase === "target" && i.target === s.target
      ? { ...s, phase: "confirming", wasDown: false }
      : reset;
  if (!i.target)
    return {
      ...s,
      phase: "locked",
      target: null,
      hoverSince: 0,
      wasDown: true,
    };
  return {
    ...s,
    phase: "target",
    target: i.target,
    hoverSince: s.target === i.target ? s.hoverSince : i.now,
    wasDown: true,
  };
}
export function bezier(t: number, a: number, b: number, c: number, d: number) {
  const u = 1 - t;
  return u * u * u * a + 3 * u * u * t * b + 3 * u * t * t * c + t * t * t * d;
}
