import { describe, it, expect } from "vitest";
import { initial, step, bezier } from "./machine";
describe("capture and release safety", () => {
  const input = { now: 0, down: true, cancel: false, over: true, target: null };
  it("locks at 250ms, never at 249", () => {
    const s = step(initial(), input);
    expect(step(s, { ...input, now: 249 }).phase).toBe("pressing");
    expect(step(s, { ...input, now: 250 }).phase).toBe("locked");
  });
  it("requires a new down edge on the sprite", () => {
    let s = step(initial(), { ...input, over: false });
    s = step(s, { ...input, now: 500 });
    expect(s.phase).toBe("idle");
  });
  it("drops immediately on release over the same resolved target", () => {
    const locked = { ...initial(), phase: "locked" as const };
    const s = step(locked, { ...input, target: "a", now: 300 });
    expect(
      step(s, { ...input, down: false, target: "b", now: 1000 }).phase,
    ).toBe("idle");
    expect(
      step(s, { ...input, down: false, target: "a", now: 301 }).phase,
    ).toBe("confirming");
  });
  it("cancel wins over release", () => {
    const s = {
      ...initial(),
      phase: "target" as const,
      target: "a",
      hoverSince: 0,
    };
    expect(
      step(s, { ...input, down: false, cancel: true, target: "a", now: 900 })
        .phase,
    ).toBe("idle");
  });
  it("cancels pressing when pointer leaves", () => {
    const s = step(initial(), input);
    expect(step(s, { ...input, over: false, now: 260 }).phase).toBe("idle");
  });
  it("bezier endpoints are exact", () => {
    expect(bezier(0, 2, 5, 8, 10)).toBe(2);
    expect(bezier(1, 2, 5, 8, 10)).toBe(10);
  });
});
