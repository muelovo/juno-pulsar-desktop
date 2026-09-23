import { describe, expect, it } from "vitest";
import { flightPosition, tailOrigin, travelFacing } from "./motion";

describe("companion motion", () => {
  it("faces the actual travel direction and keeps its tail behind it", () => {
    expect(travelFacing(1, 50, 55)).toBe(-1);
    expect(travelFacing(-1, 60, 55)).toBe(1);
    expect(tailOrigin({ x: 100, y: 50 }, -1, 1).x).toBeGreaterThan(100);
    expect(tailOrigin({ x: 100, y: 50 }, 1, 1).x).toBeLessThan(100);
  });
  it("all flight styles remain finite across ten companions", () => {
    for (const mode of ["cruise", "figure8", "swoop", "hover"] as const) {
      for (let index = 0; index < 10; index++) {
        const point = flightPosition(mode, index, 10, 42, 1920, 1080);
        expect(Number.isFinite(point.x) && Number.isFinite(point.y)).toBe(true);
      }
    }
  });
});
