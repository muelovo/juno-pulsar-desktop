import { useEffect, useRef, useState } from "react";
import { initial, step, bezier, State } from "./machine";
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
export function Flight({
  preview = false,
  config = defaults,
}: {
  preview?: boolean;
  config?: Config;
}) {
  const ref = useRef<HTMLCanvasElement>(null);
  const [label, setLabel] = useState("");
  const skinRef = useRef<{
    bundle: SkinBundle;
    images: Record<string, HTMLImageElement>;
  } | null>(null);
  useEffect(() => {
    let disposed = false;
    skinRef.current = null;
    if (native && config.skin !== "builtin") {
      void invoke<SkinBundle>("load_skin", { id: config.skin })
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
          if (!disposed) skinRef.current = { bundle, images };
        })
        .catch(() => {
          if (!disposed) setLabel("皮肤无效，已回退到原创占位");
        });
    }
    return () => {
      disposed = true;
    };
  }, [config.skin]);
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
      velocity = { x: 0, y: 0 },
      p: Frame | null = null;
    const unlisteners: (() => void)[] = [];
    if (native && !preview) {
      for (const [event, fn] of [
        [
          "cancelled",
          () => {
            state = initial();
            target = null;
            setLabel("");
          },
        ],
        [
          "operation-result",
          (e: { payload: Outcome }) => {
            state = e.payload.recycled
              ? { ...initial(), phase: "hit", since: performance.now() }
              : initial();
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
        await invoke("publish_sprite", {
          x: p.origin_x + pos.x * p.scale,
          y: p.origin_y + pos.y * p.scale,
          r:
            state.phase === "idle" &&
            !config.paused &&
            !(config.fullscreen_pause && p.fullscreen)
              ? 30 * config.size * p.scale
              : 0,
        });
        const locked = state.phase === "locked" || state.phase === "target";
        if (locked && p.left && config.deletion && now - probeAt > 100) {
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
          over: Math.hypot(x - pos.x, y - pos.y) < 30 * config.size,
          target: target?.path ?? null,
        });
        if (before !== state.phase) {
          setLabel(
            state.phase === "locked"
              ? zh.locked
              : state.phase === "target"
                ? zh.hover
                : state.phase === "pressing"
                  ? "正在捕获…"
                  : state.phase === "confirming"
                    ? "请在确认窗口中操作"
                    : "",
          );
        }
        if (state.phase === "confirming" && before !== "confirming") {
          try {
            await invoke("prepare");
          } catch {
            state = initial();
            setLabel("目标不可确认，已取消");
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
      if (now - last < 1000 / config.fps) return;
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
      const u = (1 - Math.cos((clock * Math.PI) / 8)) / 2;
      const flight = {
        x: bezier(
          u,
          0.15 * r.width,
          0.3 * r.width,
          0.7 * r.width,
          0.85 * r.width,
        ),
        y: bezier(
          u,
          0.52 * r.height,
          0.08 * r.height,
          0.92 * r.height,
          0.48 * r.height,
        ),
      };
      if (state.phase === "idle" || preview) {
        pos = flight;
        velocity = { x: 0, y: 0 };
      } else if ((state.phase === "locked" || state.phase === "target") && p) {
        const tx = (p.x - p.origin_x) / p.scale,
          ty = (p.y - p.origin_y) / p.scale;
        velocity.x += (tx - pos.x) * 80 * dt;
        velocity.y += (ty - pos.y) * 80 * dt;
        velocity.x *= Math.exp(-13 * dt);
        velocity.y *= Math.exp(-13 * dt);
        pos.x += velocity.x * dt;
        pos.y += velocity.y * dt;
      }
      for (let n = 0; n < config.count; n++) {
        const x =
            n === 0
              ? pos.x
              : r.width * (0.5 + 0.28 * Math.sin(clock / 5 + n * 2)),
          y =
            n === 0 ? pos.y : r.height * (0.5 + 0.2 * Math.sin(clock / 3 + n));
        const angle =
          state.phase === "idle" ? Math.sin(clock / 4 + n) * 0.45 : 0;
        ctx.save();
        ctx.translate(x, y);
        ctx.rotate(angle);
        ctx.scale(
          config.size * (state.phase === "locked" ? 1.1 : 1),
          config.size * (state.phase === "locked" ? 1.1 : 1),
        );
        const skin = skinRef.current;
        if (skin) {
          const key = ["locked", "target", "hit"].includes(state.phase)
            ? state.phase
            : "idle";
          const a = skin.bundle.manifest.animations[key],
            img = skin.images[a.file];
          const fw = img.width / a.frames,
            frame = Math.floor(clock * a.fps) % a.frames;
          const size = 60 * skin.bundle.manifest.assetScale;
          const trail = skin.images[skin.bundle.manifest.trail];
          ctx.globalAlpha = config.trail;
          ctx.drawImage(trail, -130, -12, 110, 24);
          ctx.globalAlpha = 1;
          ctx.drawImage(
            img,
            frame * fw,
            0,
            fw,
            img.height,
            -size / 2,
            (-size * img.height) / fw / 2,
            size,
            (size * img.height) / fw,
          );
        } else {
          drawSprite(ctx, config.trail, state.phase);
        }
        ctx.restore();
      }
      if (state.phase === "pressing" || state.phase === "target") {
        const progress = Math.min(
          1,
          (now -
            (state.phase === "pressing" ? state.since : state.hoverSince)) /
            (state.phase === "pressing" ? 250 : 600),
        );
        ctx.strokeStyle = state.phase === "target" ? "#f9ad65" : "#73efff";
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
      if (labelRef.current && !preview) {
        ctx.font = "14px Microsoft YaHei";
        ctx.textAlign = "center";
        ctx.fillStyle = "#0b1620";
        ctx.fillRect(pos.x - 135, pos.y + 45, 270, 32);
        ctx.fillStyle = "#edf6f7";
        ctx.fillText(labelRef.current, pos.x, pos.y + 66);
      }
    };
    raf = requestAnimationFrame(draw);
    return () => {
      stopped = true;
      cancelAnimationFrame(raf);
      unlisteners.forEach((u) => u());
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
function drawSprite(
  ctx: CanvasRenderingContext2D,
  trail: number,
  phase: string,
) {
  ctx.shadowBlur = 22;
  ctx.shadowColor = phase === "target" ? "#f9ad65" : "#73efff";
  const g = ctx.createLinearGradient(-110, 0, 0, 0);
  g.addColorStop(0, "transparent");
  g.addColorStop(1, `rgba(115,239,255,${trail})`);
  ctx.fillStyle = g;
  ctx.beginPath();
  ctx.moveTo(-115, 0);
  ctx.lineTo(-18, -7);
  ctx.lineTo(-18, 7);
  ctx.fill();
  ctx.fillStyle = "#edf6f7";
  ctx.beginPath();
  ctx.ellipse(0, 0, 25, 13, 0, 0, Math.PI * 2);
  ctx.fill();
  ctx.shadowBlur = 0;
  ctx.fillStyle = "#f9ad65";
  for (const s of [-1, 1]) {
    ctx.beginPath();
    ctx.moveTo(-9, s * 8);
    ctx.lineTo(-20, s * 25);
    ctx.lineTo(9, s * 9);
    ctx.fill();
  }
  ctx.fillStyle = "#193044";
  ctx.beginPath();
  ctx.ellipse(8, 0, 9, 9, 0, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = "#73efff";
  ctx.beginPath();
  ctx.arc(10, 0, 5, 0, Math.PI * 2);
  ctx.fill();
  if (phase === "hit") {
    ctx.strokeStyle = "#73efff";
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.arc(0, 0, 45, 0, Math.PI * 2);
    ctx.stroke();
  }
}
