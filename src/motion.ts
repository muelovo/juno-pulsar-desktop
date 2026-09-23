export type MotionMode = "cruise" | "figure8" | "swoop" | "hover";
export type Point = { x: number; y: number };

export function flightPosition(
  mode: MotionMode,
  index: number,
  count: number,
  clock: number,
  width: number,
  height: number,
): Point {
  const phase = (index * 0.61803398875) % 1;
  const laneCount = Math.min(5, Math.max(1, count));
  const lane = index % laneCount;
  const row = Math.floor(index / laneCount);
  const baseY =
    height *
    (count > 5
      ? 0.3 + row * 0.4 + (lane - (laneCount - 1) / 2) * 0.025
      : 0.2 + ((lane + 0.5) / laneCount) * 0.6);
  const jitter = directionalJitter(clock * 0.72 + index * 0.57);
  const traverse = (rate: number) => {
    const cycle = (clock * rate + phase) % 2;
    return cycle <= 1 ? cycle : 2 - cycle;
  };
  if (mode === "hover") {
    const homeX = width * (0.15 + ((index % 5) / 4) * 0.7);
    return {
      x: homeX + Math.sin(clock * 0.68 + index) * 18 + jitter.x,
      y: baseY + Math.sin(clock * 1.05 + index) * 15 + jitter.y,
    };
  }
  const progress = traverse(
    mode === "swoop" ? 0.055 : 0.065 + (index % 3) * 0.006,
  );
  const x = width * (-0.08 + progress * 1.16) + jitter.x;
  if (mode === "figure8") {
    return {
      x,
      y: baseY + Math.sin(progress * Math.PI * 4 + index * 0.6) * 44 + jitter.y,
    };
  }
  if (mode === "swoop") {
    return {
      x,
      y: baseY + Math.sin(progress * Math.PI * 2 + index * 0.5) * 78 + jitter.y,
    };
  }
  return {
    x,
    y: baseY + Math.sin(progress * Math.PI) * (index % 2 ? -22 : 22) + jitter.y,
  };
}

export function travelFacing(
  previous: number,
  nextX: number,
  previousX: number,
): 1 | -1 {
  const delta = nextX - previousX;
  // Avoid flicker at the turning point and during tiny hover motions.
  return Math.abs(delta) < 0.35 ? (previous < 0 ? -1 : 1) : delta < 0 ? -1 : 1;
}

export function tailOrigin(
  position: Point,
  facing: 1 | -1,
  scale: number,
): Point {
  return { x: position.x - facing * 37 * scale, y: position.y + 2 * scale };
}

function directionalJitter(clock: number): Point {
  const points = [
    { x: 0, y: -8 },
    { x: 8, y: 0 },
    { x: 0, y: 8 },
    { x: -8, y: 0 },
  ];
  const segment = Math.floor(clock) % points.length;
  const raw = clock - Math.floor(clock);
  const t = raw * raw * (3 - 2 * raw);
  const from = points[segment],
    to = points[(segment + 1) % points.length];
  return { x: from.x + (to.x - from.x) * t, y: from.y + (to.y - from.y) * t };
}
