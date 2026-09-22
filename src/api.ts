import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
export { invoke, listen };
export const native = isTauri();
export interface Config {
  speed: number;
  count: number;
  size: number;
  fps: number;
  trail: number;
  sound: boolean;
  monitor: string;
  deletion: boolean;
  autostart: boolean;
  fullscreen_pause: boolean;
  paused: boolean;
  skin: string;
}
export const defaults: Config = {
  speed: 1,
  count: 1,
  size: 1,
  fps: 30,
  trail: 0.7,
  sound: false,
  monitor: "primary",
  deletion: false,
  autostart: false,
  fullscreen_pause: true,
  paused: false,
  skin: "builtin",
};
export interface Target {
  path: string;
  name: string;
  kind: string;
}
export interface Proposal {
  token: string;
  target: Target;
  expires_seconds: number;
}
export interface Outcome {
  status: string;
  code: string;
  recycled: boolean;
}
export interface Frame {
  x: number;
  y: number;
  left: boolean;
  cancel: boolean;
  captured: number | null;
  origin_x: number;
  origin_y: number;
  scale: number;
  fullscreen: boolean;
}
export const zh = {
  locked: "脉冲飞雷已锁定",
  hover: "悬停目标 600ms 后松开",
  confirm: "移入回收站",
  cancel: "取消",
  idle: "自由飞行",
  pressing: "正在捕获",
  target: "目标锁定",
  hit: "已移入回收站",
};
export interface SkinAnimation {
  file: string;
  frames: number;
  fps: number;
}
export interface SkinManifest {
  schemaVersion: number;
  id: string;
  name: string;
  author: string;
  version: string;
  license: string;
  assetScale: number;
  animations: Record<string, SkinAnimation>;
  preview: string;
  trail: string;
}
export interface SkinBundle {
  manifest: SkinManifest;
  assets: Record<string, string>;
}
export interface Display {
  id: string;
  label: string;
  scale: number;
  width: number;
  height: number;
  x: number;
  y: number;
}
