import { useEffect, useRef, useState } from "react";
import { initial, step, State } from "./machine";
import { flightPosition, tailOrigin, travelFacing } from "./motion";
import idleUrl from "./assets/pulsar/idle.png";
import lockedUrl from "./assets/pulsar/locked.png";
import targetUrl from "./assets/pulsar/target.png";
import hitUrl from "./assets/pulsar/hit.png";
import {
  SkinBundle,
  Config,
  defaults,
  Frame,
  Target,
  invoke,
  listen,
  native,
  zh,
  Outcome,
} from "./api";
const builtinImages = Object.fromEntries(
  Object.entries({
    idle: idleUrl,
    locked: lockedUrl,
    target: targetUrl,
    hit: hitUrl,
  }).map(([key, source]) => {
    const image = new Image();
    image.src = source;
    return [key, image];
  }),
) as Record<string, HTMLImageElement>;
export function Flight({
  preview = false,
  config = defaults,
}: {
  preview?: boolean;
  config?: Config;
}) {
  const ref = useRef<HTMLCanvasElement>(null);
  const [label, setLabel] = useState("");
  const skinRef = useRef(
    new Map<
      string,
      { bundle: SkinBundle; images: Record<string, HTMLImageElement> }
    >(),
  );
  useEffect(() => {
    let disposed = false;
    skinRef.current = new Map();
    const ids = [...new Set([config.skin, ...config.skins])].filter(
      (id) => id && id !== "builtin",
    );
    if (native) {
      ids.forEach(
        (id) =>
          void invoke<SkinBundle>("load_skin", { id })
            .then(async (bundle) => {
              const images: Record<string, HTMLImageElement> = {};
              await Promise.all(
                Object.entries(bundle.assets).map(async ([key, src]) => {
                  const image = new Image();
                  image.src = src;
                  await image.decode();
                  images[key] = image;
                }),
              );
              if (!disposed) skinRef.current.set(id, { bundle, images });
            })
            .catch(() => {
              if (!disposed) setLabel("部分皮肤无效，已回退到原创星脉");
            }),
      );
    }
    return () => {
      disposed = true;
    };
  }, [config.skin, config.skins]);
  useEffect(() => {
    const canvas = ref.current!,
      ctx = canvas.getContext("2d")!;
    let stopped = false,
      raf = 0,
      last = 0,
      clock = 0,
      prev = performance.now(),
      pending = false,
      probeAt = 0,
      target: Target | null = null,
      targetAt = 0,
      state: State = initial();
    let pos = { x: 0, y: 0 },
      activeIndex: number | null = null,
      positions = Array.from({ length: 10 }, () => ({ x: 0, y: 0 })),
      trails = Array.from({ length: 10 }, () => createTrailState()),
      facings = Array.from({ length: 10 }, () => 1 as 1 | -1),
      p: Frame | null = null;
    let audio: AudioContext | null = null;
    const sound = (kind: "capture" | "locked" | "launch") => {
      if (!config.sound || preview) return;
      try {
        audio ??= new AudioContext();
        const start = audio.currentTime;
        const notes =
          kind === "capture"
            ? [420, 580]
            : kind === "locked"
              ? [640, 880, 1120]
              : [920, 620, 300];
        notes.forEach((frequency, index) => {
          const oscillator = audio!.createOscillator();
          const gain = audio!.createGain();
          oscillator.type = kind === "launch" ? "sawtooth" : "sine";
          oscillator.frequency.setValueAtTime(frequency, start + index * 0.055);
          gain.gain.setValueAtTime(0.0001, start + index * 0.055);
          gain.gain.exponentialRampToValueAtTime(
            0.07,
            start + index * 0.055 + 0.012,
          );
          gain.gain.exponentialRampToValueAtTime(
            0.0001,
            start + index * 0.055 + 0.16,
          );
          oscillator.connect(gain).connect(audio!.destination);
          oscillator.start(start + index * 0.055);
          oscillator.stop(start + index * 0.055 + 0.17);
        });
      } catch {
        // Audio feedback must never interfere with capture or recycle behavior.
      }
    };
    const unlisteners: (() => void)[] = [];
    if (native && !preview) {
      for (const [event, fn] of [
        [
          "cancelled",
          () => {
            state = initial();
            target = null;
            activeIndex = null;
            setLabel("");
          },
        ],
        [
          "operation-result",
          (e: { payload: Outcome }) => {
            state = e.payload.recycled
              ? { ...initial(), phase: "hit", since: performance.now() }
              : initial();
            if (e.payload.recycled) sound("launch");
            if (!e.payload.recycled) activeIndex = null;
            setLabel(e.payload.recycled ? zh.hit : "操作未完成");
          },
        ],
      ] as const) {
        void listen(event, fn as never).then((u) =>
          stopped ? u() : unlisteners.push(u),
        );
      }
    }
    const sample = async (now: number) => {
      if (!native || preview || pending || stopped) return;
      pending = true;
      try {
        p = await invoke<Frame>("input_frame");
        if (stopped) return;
        const x = (p.x - p.origin_x) / p.scale,
          y = (p.y - p.origin_y) / p.scale;
        const captureEnabled =
          state.phase === "idle" &&
          !config.paused &&
          !(config.fullscreen_pause && p.fullscreen);
        await invoke("publish_sprites", {
          regions: captureEnabled
            ? positions.slice(0, config.count).map((position, index) => ({
                index,
                x: p!.origin_x + position.x * p!.scale,
                y: p!.origin_y + position.y * p!.scale,
                radius: 52 * config.size * p!.scale,
              }))
            : [],
        });
        if (
          state.phase === "idle" &&
          p.captured !== null &&
          p.captured < config.count
        ) {
          activeIndex = p.captured;
          pos = { ...positions[activeIndex] };
          sound("capture");
        }
        const locked = state.phase === "locked" || state.phase === "target";
        if (locked && p.left && config.deletion && now - probeAt > 32) {
          probeAt = now;
          try {
            target = await invoke<Target | null>("probe");
            targetAt = performance.now();
          } catch {
            target = null;
          }
        }
        if (performance.now() - targetAt > 220) target = null;
        const before = state.phase;
        state = step(state, {
          now: performance.now(),
          down: p.left,
          cancel:
            p.cancel ||
            config.paused ||
            (config.fullscreen_pause && p.fullscreen),
          over:
            activeIndex !== null &&
            Math.hypot(x - pos.x, y - pos.y) < 58 * config.size,
          target: target?.path ?? null,
        });
        if (state.phase === "idle" && before !== "idle") activeIndex = null;
        if (before !== state.phase) {
          if (state.phase === "locked") sound("locked");
          setLabel(
            state.phase === "locked"
              ? zh.locked
              : state.phase === "target"
                ? zh.hover
                : state.phase === "pressing"
                  ? "正在捕获…"
                  : state.phase === "confirming"
                    ? "正在移入回收站…"
                    : "",
          );
        }
        if (state.phase === "confirming" && before !== "confirming") {
          try {
            await invoke("drop_recycle");
          } catch {
            state = initial();
            activeIndex = null;
            setLabel("目标无法可靠解析，已取消");
          }
        }
      } catch {
        state = initial();
        target = null;
        setLabel("桌面连接不可用");
      } finally {
        pending = false;
      }
    };
    const draw = (now: number) => {
      raf = requestAnimationFrame(draw);
      if (now - last < 1000 / config.fps - 0.5) return;
      const dt = Math.min((now - prev) / 1000, 0.05);
      prev = now;
      last = now;
      const r = canvas.getBoundingClientRect(),
        d = devicePixelRatio;
      if (
        canvas.width !== Math.round(r.width * d) ||
        canvas.height !== Math.round(r.height * d)
      ) {
        canvas.width = Math.round(r.width * d);
        canvas.height = Math.round(r.height * d);
      }
      const pause =
        config.paused || (!preview && config.fullscreen_pause && p?.fullscreen);
      if (!pause) clock += dt * config.speed;
      void sample(now);
      ctx.setTransform(d, 0, 0, d, 0, 0);
      ctx.clearRect(0, 0, r.width, r.height);
      if (pause && !preview) return;
      if ((state.phase === "locked" || state.phase === "target") && p) {
        pos.x = (p.x - p.origin_x) / p.scale;
        pos.y = (p.y - p.origin_y) / p.scale;
      }
      for (let n = 0; n < config.count; n++) {
        const idle = flightPosition(
          config.motion,
          n,
          config.count,
          clock,
          r.width,
          r.height,
        );
        const isActive = activeIndex === n && !preview;
        const position = isActive && state.phase !== "idle" ? pos : idle;
        facings[n] = travelFacing(facings[n], position.x, trails[n].lastX);
        positions[n] = { ...position };
        const phase = isActive ? state.phase : "idle";
        const skinId = config.skins[n] || config.skin;
        const skin =
          skinId === "builtin" ? undefined : skinRef.current.get(skinId);
        updateAndDrawTrail(
          ctx,
          trails[n],
          position,
          facings[n],
          config.size * (skin ? skin.bundle.manifest.assetScale : 1.55),
          now,
          n,
          config.trail,
          dt,
        );
        ctx.save();
        ctx.translate(position.x, position.y);
        ctx.scale(
          facings[n] * config.size * (phase === "locked" ? 1.1 : 1),
          config.size * (phase === "locked" ? 1.1 : 1),
        );
        if (skin) {
          const key = ["locked", "target", "hit"].includes(phase)
            ? phase
            : "idle";
          const a = skin.bundle.manifest.animations[key],
            img = skin.images[a.file];
          const fw = img.width / a.frames,
            frame = Math.floor(clock * a.fps) % a.frames;
          const size = 60 * skin.bundle.manifest.assetScale;
          drawMeshSprite(
            ctx,
            img,
            size,
            frame * fw,
            fw,
            img.height,
            clock,
            n,
            phase,
          );
        } else {
          drawBuiltinSprite(ctx, builtinImages, phase, clock, n);
        }
        ctx.restore();
      }
      if (state.phase === "pressing") {
        const progress = Math.min(1, (now - state.since) / 250);
        ctx.strokeStyle = "#73efff";
        ctx.lineWidth = 3;
        ctx.beginPath();
        ctx.arc(
          pos.x,
          pos.y,
          40 * config.size,
          -Math.PI / 2,
          -Math.PI / 2 + progress * Math.PI * 2,
        );
        ctx.stroke();
      }
      if (labelRef.current && !preview)
        drawStatusPill(ctx, pos.x, pos.y + 62, labelRef.current, state.phase);
    };
    raf = requestAnimationFrame(draw);
    return () => {
      stopped = true;
      cancelAnimationFrame(raf);
      unlisteners.forEach((u) => u());
      void audio?.close();
    };
  }, [preview, config]);
  const labelRef = useRef(label);
  labelRef.current = label;
  return (
    <canvas
      aria-label="脉冲飞行物动画"
      ref={ref}
      className={preview ? "flight preview-flight" : "flight desktop-flight"}
    />
  );
}
function drawBuiltinSprite(
  ctx: CanvasRenderingContext2D,
  images: Record<string, HTMLImageElement>,
  phase: string,
  clock: number,
  index: number,
) {
  const key = ["locked", "target", "hit"].includes(phase) ? phase : "idle";
  if (!images[key].complete) return;
  ctx.shadowBlur = phase === "idle" ? 0 : 12;
  ctx.shadowColor = phase === "target" ? "#ff68b8" : "#52eeff";
  drawMeshSprite(
    ctx,
    images[key],
    116,
    0,
    images[key].width,
    images[key].height,
    clock,
    index,
    phase,
  );
  ctx.shadowBlur = 0;
}

type TrailParticle = {
  x: number;
  y: number;
  age: number;
  life: number;
  size: number;
  color: number;
};

type TrailState = {
  particles: TrailParticle[];
  nextEmit: number;
  lastX: number;
  lastY: number;
};

function createTrailState(): TrailState {
  return { particles: [], nextEmit: 0, lastX: Number.NaN, lastY: Number.NaN };
}

function updateAndDrawTrail(
  ctx: CanvasRenderingContext2D,
  trail: TrailState,
  position: { x: number; y: number },
  facing: 1 | -1,
  scale: number,
  now: number,
  index: number,
  amount: number,
  dt: number,
) {
  trail.lastX = position.x;
  trail.lastY = position.y;
  if (amount > 0.01 && now >= trail.nextEmit) {
    trail.nextEmit = now + 55;
    const origin = tailOrigin(position, facing, scale);
    trail.particles.push({
      x: origin.x,
      y: origin.y,
      age: 0,
      life: 0.58,
      size: 2 + ((index * 7 + trail.particles.length) % 3),
      color: (index + trail.particles.length) % 3,
    });
  }
  ctx.save();
  ctx.globalCompositeOperation = "lighter";
  const colors = ["255 210 92", "111 239 255", "255 121 190"];
  for (let i = trail.particles.length - 1; i >= 0; i--) {
    const particle = trail.particles[i];
    particle.age += dt;
    if (particle.age >= particle.life) {
      trail.particles.splice(i, 1);
      continue;
    }
    const alpha = (1 - particle.age / particle.life) * amount;
    const radius = particle.size * (0.6 + particle.age / particle.life);
    ctx.fillStyle = `rgb(${colors[particle.color]} / ${alpha})`;
    ctx.beginPath();
    ctx.arc(particle.x, particle.y, radius, 0, Math.PI * 2);
    ctx.fill();
  }
  if (trail.particles.length > 12)
    trail.particles.splice(0, trail.particles.length - 12);
  ctx.restore();
}

function drawMeshSprite(
  ctx: CanvasRenderingContext2D,
  image: HTMLImageElement,
  width: number,
  sourceX: number,
  sourceWidth: number,
  sourceHeight: number,
  clock: number,
  index: number,
  phase: string,
) {
  // A lightweight strip mesh gives every existing PNG skin articulated motion:
  // the tail bends most, the body breathes, and the head stays readable.
  const height = (width * sourceHeight) / sourceWidth;
  const strips = 8;
  const sourceStrip = sourceWidth / strips;
  const destinationStrip = width / strips;
  const energy = phase === "idle" ? 1 : 0.35;
  const breathe = 1 + Math.sin(clock * 1.7 + index) * 0.025 * energy;
  ctx.scale(1, breathe);
  for (let strip = 0; strip < strips; strip++) {
    const t = strip / (strips - 1);
    const tailWeight = Math.pow(1 - t, 2.2);
    const wave =
      Math.sin(clock * 3.2 + index * 0.9 + t * 3.4) * 7 * tailWeight * energy;
    const tilt = Math.sin(clock * 1.05 + index) * 2.5 * (t - 0.5) * energy;
    ctx.drawImage(
      image,
      sourceX + strip * sourceStrip,
      0,
      sourceStrip,
      sourceHeight,
      -width / 2 + strip * destinationStrip,
      -height / 2 + wave + tilt,
      destinationStrip + 1,
      height,
    );
  }
}
function drawStatusPill(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  text: string,
  phase: string,
) {
  ctx.save();
  ctx.font = "600 13px 'Microsoft YaHei', 'Segoe UI', sans-serif";
  const width = Math.max(158, ctx.measureText(text).width + 54);
  const left = x - width / 2;
  const gradient = ctx.createLinearGradient(left, y, left + width, y);
  gradient.addColorStop(0, "rgba(7,18,29,.94)");
  gradient.addColorStop(1, "rgba(18,42,58,.94)");
  ctx.shadowBlur = 18;
  ctx.shadowColor = phase === "target" ? "#ff68b8" : "#52eeff";
  ctx.fillStyle = gradient;
  ctx.beginPath();
  ctx.roundRect(left, y - 17, width, 34, 17);
  ctx.fill();
  ctx.shadowBlur = 0;
  ctx.strokeStyle = phase === "target" ? "#ff68b8" : "#52eeff";
  ctx.globalAlpha = 0.8;
  ctx.stroke();
  ctx.globalAlpha = 1;
  ctx.fillStyle = phase === "target" ? "#ff68b8" : "#52eeff";
  ctx.beginPath();
  ctx.arc(left + 17, y, 4, 0, Math.PI * 2);
  ctx.fill();
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillStyle = "#f4fbff";
  ctx.fillText(text, x + 7, y);
  ctx.restore();
}
