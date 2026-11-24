import type { TileConfig } from "./types";

export interface PopularAppPreset {
  id: string;
  name: string;
  title: string;
  subtitle: string;
  icon: string;
  color: string;
  program: string;
  args: string[];
}

export const POPULAR_APPS: PopularAppPreset[] = [
  {
    id: "obs",
    name: "OBS Studio",
    title: "OBS",
    subtitle: "ABRIR",
    icon: "obs",
    color: "#38BDF8",
    program: "C:\\Program Files\\obs-studio\\bin\\64bit\\obs64.exe",
    args: [],
  },
  {
    id: "discord",
    name: "Discord",
    title: "Discord",
    subtitle: "ABRIR",
    icon: "discord",
    color: "#5865F2",
    program: "%LOCALAPPDATA%\\Discord\\Update.exe",
    args: ["--processStart", "Discord.exe"],
  },
  {
    id: "spotify",
    name: "Spotify",
    title: "Spotify",
    subtitle: "ABRIR",
    icon: "spotify",
    color: "#1DB954",
    program: "%APPDATA%\\Spotify\\Spotify.exe",
    args: [],
  },
  {
    id: "chrome",
    name: "Google Chrome",
    title: "Chrome",
    subtitle: "ABRIR",
    icon: "chrome",
    color: "#4285F4",
    program: "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
    args: [],
  },
  {
    id: "edge",
    name: "Microsoft Edge",
    title: "Edge",
    subtitle: "ABRIR",
    icon: "edge",
    color: "#0EA5E9",
    program: "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
    args: [],
  },
  {
    id: "vscode",
    name: "VS Code",
    title: "VS Code",
    subtitle: "ABRIR",
    icon: "vscode",
    color: "#007ACC",
    program: "%LOCALAPPDATA%\\Programs\\Microsoft VS Code\\Code.exe",
    args: [],
  },
  {
    id: "steam",
    name: "Steam",
    title: "Steam",
    subtitle: "ABRIR",
    icon: "steam",
    color: "#66C0F4",
    program: "C:\\Program Files (x86)\\Steam\\steam.exe",
    args: [],
  },
  {
    id: "twitch",
    name: "Twitch",
    title: "Twitch",
    subtitle: "WEB",
    icon: "twitch",
    color: "#9146FF",
    program: "https://www.twitch.tv",
    args: [],
  },
  {
    id: "youtube",
    name: "YouTube",
    title: "YouTube",
    subtitle: "WEB",
    icon: "youtube",
    color: "#EF4444",
    program: "https://www.youtube.com",
    args: [],
  },
  {
    id: "terminal",
    name: "Windows Terminal",
    title: "Terminal",
    subtitle: "ABRIR",
    icon: "terminal",
    color: "#22D3EE",
    program: "wt.exe",
    args: [],
  },
  {
    id: "explorer",
    name: "Explorador de Arquivos",
    title: "Pastas",
    subtitle: "ABRIR",
    icon: "explorer",
    color: "#FBBF24",
    program: "explorer.exe",
    args: [],
  },
];

export interface PageTemplate {
  id: string;
  defaultSlug: string;
  label: string;
  description: string;
  badge: string;
  color: string;
  buildTiles: () => Record<string, TileConfig>;
}

const sensorTile = (
  title: string,
  id: string,
  unit: string,
  max: number,
  group: string,
  style?: "ring" | "card",
  is_temp = false,
): TileConfig => ({
  type: "sensor",
  title,
  subtitle: "",
  id,
  unit,
  decimals: 0,
  min: 0,
  max,
  group,
  style: style ?? "card",
  is_temp,
});

const widgetTile = (widget: string, action?: TileConfig["action"]): TileConfig => ({
  type: "button",
  title: "",
  subtitle: "",
  widget,
  unit: "",
  decimals: 0,
  is_temp: false,
  action: action ?? null,
});

const buttonTile = (
  title: string,
  subtitle: string,
  icon: string,
  action: TileConfig["action"],
  state?: string,
): TileConfig => ({
  type: "button",
  title,
  subtitle,
  icon,
  unit: "",
  decimals: 0,
  is_temp: false,
  state: state ?? null,
  action,
});

export const PAGE_TEMPLATES: PageTemplate[] = [
  {
    id: "monitor",
    defaultSlug: "monitor",
    label: "Monitor de Hardware (Pré-definido)",
    description: "Grade completa de telemetria: GPU, CPU, RAM, VRAM, Rede, FPS, Relógio, OBS e Clima.",
    badge: "MON",
    color: "#22C55E",
    buildTiles: () => ({
      "0,0": sensorTile("GPU", "GPU_UTIL", "%", 100, "gpu", "ring"),
      "1,0": sensorTile("GPU °C", "GPU_TEMP", "°", 100, "gpu", "card", true),
      "2,0": sensorTile("GPU Watt", "GPU_WATT", "W", 450, "gpu", "card"),
      "3,0": sensorTile("VRAM", "VRAM_PCT", "%", 100, "vram", "ring"),
      "4,0": sensorTile("GPU Fan", "GPU_FAN", "%", 100, "gpu", "ring"),
      "5,0": widgetTile("clock"),
      "0,1": sensorTile("CPU", "CPU_UTIL", "%", 100, "cpu", "ring"),
      "1,1": sensorTile("CPU °C", "CPU_TEMP", "°", 100, "cpu", "card", true),
      "2,1": sensorTile("CPU Watt", "CPU_WATT", "W", 200, "cpu", "card"),
      "3,1": sensorTile("RAM", "RAM_UTIL", "%", 100, "ram", "ring"),
      "4,1": sensorTile("Core", "CPU_CLOCK", "MHz", 5500, "cpu", "card"),
      "5,1": widgetTile("obs_status"),
      "0,2": sensorTile("DOWN", "NET_DL", "Mbps", 1000, "net_down", "card"),
      "1,2": sensorTile("FPS", "FPS", "", 240, "fps", "card"),
      "2,2": sensorTile("UP", "NET_UL", "Mbps", 500, "net_up", "card"),
      "3,2": widgetTile("obs_logo", { kind: "page", target: "obs" }),
      "4,2": widgetTile("nav_next", { kind: "page", target: "next" }),
      "5,2": widgetTile("weather"),
    }),
  },
  {
    id: "obs",
    defaultSlug: "obs",
    label: "OBS Studio (Pré-definido)",
    description: "Controles de Gravação, Pausa, Áudio, Cenas OBS, Saúde CPU/GPU, Discord e Abrir OBS.",
    badge: "OBS",
    color: "#38BDF8",
    buildTiles: () => ({
      "0,0": buttonTile("Gravar", "CLIQUE", "record", { kind: "obs", op: "toggle_record" }, "record"),
      "1,0": buttonTile("Pausar", "REC ATIVO", "pause", { kind: "obs", op: "toggle_record_pause" }, "record_pause"),
      "2,0": buttonTile("Som Jogo", "ATIVO", "audio", { kind: "obs", op: "toggle_mute", input: "Áudio do Jogo (A50 X Game)" }, "mute"),
      "3,0": buttonTile("Cena 1", "OBS", "track", { kind: "obs", op: "scene", name: "AC-TrackTrainee" }),
      "4,0": buttonTile("Logo", "CANAL", "logo", { kind: "obs", op: "scene", name: "logo canal" }),
      "5,0": widgetTile("clock"),
      "0,1": buttonTile("Pro Studio", "CENA 1", "studio", { kind: "obs", op: "scene", name: "1. ACC - Pro Studio (16:9)" }),
      "1,1": buttonTile("Jogo+Cam", "CENA 2", "game_cam", { kind: "obs", op: "scene", name: "2. ACC - Jogo Puro + Cam Sobreposta (16:9)" }),
      "2,1": buttonTile("Split", "CENA 3", "split", { kind: "obs", op: "scene", name: "3. ACC - Lateral Split (16:9)" }),
      "3,1": buttonTile("Foco Total", "CENA 4", "focus", { kind: "obs", op: "scene", name: "4. Vídeos - Foco Total (Centro 16:9)" }),
      "4,1": buttonTile("Cmd Center", "CENA 5", "multi", { kind: "obs", op: "scene", name: "5. Vídeos - Command Center (Multi-Screen)" }),
      "5,1": widgetTile("obs_status"),
      "0,2": widgetTile("cpu_health"),
      "1,2": widgetTile("gpu_health"),
      "2,2": buttonTile("Cam 4K", "CENA 6", "cam4k", { kind: "obs", op: "scene", name: "6. Câmera Tela Cheia (4K)" }),
      "3,2": widgetTile("nav_prev", { kind: "page", target: "monitor" }),
      "4,2": buttonTile("Discord", "ABRIR", "discord", { kind: "run", program: "%LOCALAPPDATA%\\Discord\\Update.exe", args: ["--processStart", "Discord.exe"] }),
      "5,2": widgetTile("weather"),
    }),
  },
  {
    id: "apps",
    defaultSlug: "apps",
    label: "Launcher de Apps & Mídia (Pré-definido)",
    description: "Atalhos rápidos com logos oficiais: OBS, Discord, Spotify, Chrome, VS Code, Steam, Twitch, YouTube e Mídia.",
    badge: "APP",
    color: "#5865F2",
    buildTiles: () => ({
      "0,0": buttonTile("OBS", "ABRIR", "obs", { kind: "run", program: "C:\\Program Files\\obs-studio\\bin\\64bit\\obs64.exe", args: [] }),
      "1,0": buttonTile("Discord", "ABRIR", "discord", { kind: "run", program: "%LOCALAPPDATA%\\Discord\\Update.exe", args: ["--processStart", "Discord.exe"] }),
      "2,0": buttonTile("Spotify", "ABRIR", "spotify", { kind: "run", program: "%APPDATA%\\Spotify\\Spotify.exe", args: [] }),
      "3,0": buttonTile("Chrome", "ABRIR", "chrome", { kind: "run", program: "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe", args: [] }),
      "4,0": buttonTile("VS Code", "ABRIR", "vscode", { kind: "run", program: "%LOCALAPPDATA%\\Programs\\Microsoft VS Code\\Code.exe", args: [] }),
      "5,0": widgetTile("clock"),
      "0,1": buttonTile("Steam", "ABRIR", "steam", { kind: "run", program: "C:\\Program Files (x86)\\Steam\\steam.exe", args: [] }),
      "1,1": buttonTile("Twitch", "WEB", "twitch", { kind: "run", program: "https://www.twitch.tv", args: [] }),
      "2,1": buttonTile("YouTube", "WEB", "youtube", { kind: "run", program: "https://www.youtube.com", args: [] }),
      "3,1": buttonTile("Terminal", "ABRIR", "terminal", { kind: "run", program: "wt.exe", args: [] }),
      "4,1": buttonTile("Pastas", "ABRIR", "explorer", { kind: "run", program: "explorer.exe", args: [] }),
      "5,1": widgetTile("obs_status"),
      "0,2": buttonTile("Anterior", "MÍDIA", "prev", { kind: "media", op: "prev" }),
      "1,2": buttonTile("Play/Pause", "MÍDIA", "play", { kind: "media", op: "play_pause" }),
      "2,2": buttonTile("Próxima", "MÍDIA", "next", { kind: "media", op: "next" }),
      "3,2": widgetTile("nav_prev", { kind: "page", target: "prev" }),
      "4,2": widgetTile("nav_next", { kind: "page", target: "next" }),
      "5,2": widgetTile("weather"),
    }),
  },
  {
    id: "blank",
    defaultSlug: "nova",
    label: "Página em Branco",
    description: "Comece do zero e arraste sensores, ações ou apps da biblioteca.",
    badge: "+",
    color: "#94A3B8",
    buildTiles: () => ({}),
  },
];
