import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { Flight } from "./Flight";
import {
  Display,
  SkinManifest,
  Config,
  defaults,
  Proposal,
  Outcome,
  invoke,
  listen,
  native,
} from "./api";
import "./style.css";
function App() {
  const [c, setC] = useState(defaults);
  const [msg, setMsg] = useState("");
  useEffect(() => {
    if (!native) return;
    void invoke<Config>("get_config")
      .then(setC)
      .catch(() => setMsg("配置读取失败，使用安全默认值"));
    let dispose: undefined | (() => void),
      gone = false;
    void listen<Config>("config", (e) => setC(e.payload)).then((u) =>
      gone ? u() : (dispose = u),
    );
    return () => {
      gone = true;
      dispose?.();
    };
  }, []);
  const [displays, setDisplays] = useState<Display[]>([]),
    [skins, setSkins] = useState<SkinManifest[]>([]);
  useEffect(() => {
    if (native) {
      void invoke<Display[]>("list_displays")
        .then(setDisplays)
        .catch(() => {});
      void invoke<SkinManifest[]>("list_skins")
        .then(setSkins)
        .catch(() => {});
    }
  }, []);
  const importSkin = async (folder: boolean) => {
    try {
      const m = await invoke<SkinManifest | null>("import_skin", { folder });
      if (m) {
        setSkins(await invoke<SkinManifest[]>("list_skins"));
        setMsg("皮肤导入完成");
      }
    } catch (e) {
      setMsg("皮肤被拒绝：" + String(e));
    }
  };
  const update = async (value: Config) => {
    if (!native) {
      setC(value);
      setMsg("浏览器预览：设置未写入桌面应用");
      return;
    }
    try {
      await invoke("save_config", { value });
      setC(value);
      setMsg("已保存到本机");
    } catch {
      setMsg("设置保存失败，请检查配置与本地权限");
    }
  };
  if (location.search.includes("confirm")) return <Confirm />;
  if (location.search.includes("overlay")) return <Flight config={c} />;
  return (
    <main>
      <header>
        <span>
          <b className="mark">◉</b> JUNO / PULSAR DESKTOP
        </span>
        <small>0.1.0 · 开发预览</small>
      </header>
      <div className="heading">
        <div>
          <span className="eyebrow">你的桌面，一片微型宇宙</span>
          <h1>
            让桌面，<em>有一点引力。</em>
          </h1>
          <p>平滑巡航、长按捕获，把日常操作变成一次小小的发射。</p>
        </div>
        <button onClick={() => void update({ ...c, paused: !c.paused })}>
          {c.paused ? "继续飞行" : "暂停飞行"}
        </button>
      </div>
      <section className="preview">
        <div className="preview-caption">
          <span className="dot" /> {c.paused ? "已暂停" : "自由飞行"}
          <small>原创占位皮肤 / CANVAS 2D</small>
        </div>
        <Flight preview config={c} />
        <div className="preview-bottom">
          按住 250ms 捕获 <span>→</span> 悬停 600ms <span>→</span> 松开并确认
        </div>
      </section>
      <div className="settings-grid">
        <section className="panel">
          <h2>
            飞行参数 <small>FLIGHT</small>
          </h2>
          {(
            [
              ["speed", "飞行速度", 0.2, 3, 0.1, "×"],
              ["size", "飞行物大小", 0.5, 2, 0.1, "×"],
              ["trail", "尾迹强度", 0, 1, 0.1, ""],
            ] as const
          ).map(([key, label, min, max, step, unit]) => (
            <label className="range" key={key}>
              <span>
                {label}
                <output>
                  {c[key].toFixed(1)}
                  {unit}
                </output>
              </span>
              <input
                aria-label={label}
                type="range"
                min={min}
                max={max}
                step={step}
                value={c[key]}
                onChange={(e) => setC({ ...c, [key]: Number(e.target.value) })}
                onPointerUp={() => void update(c)}
                onKeyUp={() => void update(c)}
              />
            </label>
          ))}
          <div className="pair">
            <label>
              数量
              <select
                value={c.count}
                onChange={(e) => void update({ ...c, count: +e.target.value })}
              >
                {[1, 2, 3].map((n) => (
                  <option key={n} value={n}>
                    {n} 枚
                  </option>
                ))}
              </select>
            </label>
            <label>
              帧率
              <select
                value={c.fps}
                onChange={(e) => void update({ ...c, fps: +e.target.value })}
              >
                {[15, 30, 60].map((n) => (
                  <option key={n} value={n}>
                    {n} FPS
                  </option>
                ))}
              </select>
            </label>
          </div>
        </section>
        <section className="panel">
          <h2>
            桌面行为 <small>DESKTOP</small>
          </h2>
          <Toggle
            label="启用回收站操作"
            note="默认关闭；启用后仍需逐次确认"
            checked={c.deletion}
            onChange={(v) => void update({ ...c, deletion: v })}
          />
          <Toggle
            label="全屏应用时暂停"
            note="检测当前屏幕前台窗口"
            checked={c.fullscreen_pause}
            onChange={(v) => void update({ ...c, fullscreen_pause: v })}
          />
          <Toggle
            label="开机启动"
            note="仅当前用户，无需管理员权限"
            checked={c.autostart}
            onChange={(v) => void update({ ...c, autostart: v })}
          />
          <div className="disabled-row">
            <span>音效</span>
            <small>暂无素材 · 未启用</small>
          </div>
        </section>
      </div>
      <section className="panel library">
        <h2>
          皮肤与显示器 <small>PERSONALIZE</small>
        </h2>
        <div className="pair">
          <label>
            当前皮肤
            <select
              value={c.skin}
              onChange={(e) => void update({ ...c, skin: e.target.value })}
            >
              <option value="builtin">原创脉冲 · 默认</option>
              {skins.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.name} / {s.author}
                </option>
              ))}
            </select>
          </label>
          <label>
            显示范围
            <select
              value={c.monitor}
              onChange={(e) => void update({ ...c, monitor: e.target.value })}
            >
              <option value="primary">主显示器</option>
              <option value="all">全部显示器</option>
              {displays.map((d) => (
                <option key={d.id} value={d.id}>
                  {d.label}
                </option>
              ))}
            </select>
          </label>
        </div>
        <div className="import-actions">
          <button disabled={!native} onClick={() => void importSkin(false)}>
            导入 .jpskin
          </button>
          <button disabled={!native} onClick={() => void importSkin(true)}>
            从目录导入
          </button>
          <small>PNG 图集 · 本地校验 · 不含音效</small>
        </div>
      </section>
      <footer>
        <span role="status">
          {msg || "本地运行 · 无遥测 · 文件路径不会上传"}
        </span>
        <button
          className="text-button"
          onClick={() => void update({ ...defaults })}
        >
          恢复默认
        </button>
      </footer>
      <p className="notice">
        非官方开源项目。已有游戏素材仅作本地参考，不随本应用分发。桌面兼容性仍需手工验收。
      </p>
    </main>
  );
}
function Toggle({
  label,
  note,
  checked,
  onChange,
}: {
  label: string;
  note: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="toggle">
      <span>
        {label}
        <small>{note}</small>
      </span>
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
      />
    </label>
  );
}
function Confirm() {
  const [p, setP] = useState<Proposal | null>(null),
    [result, setResult] = useState<Outcome | null>(null),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const cancel = () => {
    if (!busy) void invoke("cancel");
  };
  useEffect(() => {
    void invoke<Proposal | null>("pending")
      .then(setP)
      .catch(() => setError("确认信息不可用"));
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") cancel();
    };
    document.addEventListener("keydown", key);
    const menu = (e: MouseEvent) => {
      e.preventDefault();
      cancel();
    };
    document.addEventListener("contextmenu", menu);
    return () => {
      document.removeEventListener("keydown", key);
      document.removeEventListener("contextmenu", menu);
    };
  }, [busy]);
  useEffect(() => {
    let gone = false;
    let off: (() => void) | undefined;
    void listen<Proposal>("proposal", (e) => {
      setP(e.payload);
      setResult(null);
      setError("");
    }).then((u) => (gone ? u() : (off = u)));
    return () => {
      gone = true;
      off?.();
    };
  }, []);
  const submit = async () => {
    if (!p || busy) return;
    setBusy(true);
    try {
      setResult(await invoke<Outcome>("recycle", { token: p.token }));
    } catch {
      setError("无法执行操作，请重新捕获目标。");
    } finally {
      setBusy(false);
    }
  };
  return (
    <main className="confirmation">
      <span className="eyebrow">请确认目标</span>
      <h1>
        {result?.recycled
          ? "已移入回收站"
          : result
            ? "操作未完成"
            : "将此项目移入回收站？"}
      </h1>
      {p && (
        <dl>
          <dt>文件名</dt>
          <dd>{p.target.name}</dd>
          <dt>类型</dt>
          <dd>{p.target.kind}</dd>
          <dt>完整路径</dt>
          <dd className="path">{p.target.path}</dd>
        </dl>
      )}
      <p role="status">
        {error ||
          (result
            ? result.recycled
              ? "可从 Windows 回收站恢复。"
              : `已拒绝或未确认回收：${result.code}`
            : "确认有效期为 30 秒。不会使用永久删除。")}
      </p>
      <div className="actions">
        <button autoFocus disabled={busy} onClick={cancel}>
          {result ? "关闭" : "取消"}
        </button>
        <button
          className="danger"
          disabled={!p || busy || !!result}
          onClick={() => void submit()}
        >
          {busy ? "正在处理…" : "移入回收站"}
        </button>
      </div>
    </main>
  );
}
createRoot(document.getElementById("root")!).render(<App />);
