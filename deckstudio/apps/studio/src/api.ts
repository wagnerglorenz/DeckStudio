import type {
  ActionConfig,
  AppConfig,
  AutostartStatus,
  BackupCandidate,
  EngineHealth,
  GridPreviewResponse,
  ObsCatalog,
  SensorReading,
  SourceStatus,
  UpdateCheckResponse,
  UpdateInstallStatus,
} from "./types";

const BASE = "http://127.0.0.1:19876";

let cachedToken: string | null = null;

async function getToken(): Promise<string> {
  if (cachedToken) return cachedToken;
  const tauri = (window as unknown as { __TAURI_INTERNALS__?: { invoke?: <T>(cmd: string) => Promise<T> } })
    .__TAURI_INTERNALS__;
  if (tauri?.invoke) {
    try {
      cachedToken = await tauri.invoke<string>("get_ipc_token");
      return cachedToken ?? "";
    } catch {
      return "";
    }
  }
  return "";
}

async function req<T>(path: string, init?: RequestInit, retried = false): Promise<T> {
  const token = await getToken();
  const res = await fetch(`${BASE}${path}`, {
    ...init,
    headers: {
      "Content-Type": "application/json",
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...(init?.headers ?? {}),
    },
  });
  if (res.status === 401 && !retried) {
    cachedToken = null;
    return req<T>(path, init, true);
  }
  if (!res.ok) {
    throw new Error(`${res.status} ${await res.text()}`);
  }
  const ct = res.headers.get("content-type") ?? "";
  if (ct.includes("application/json")) {
    return (await res.json()) as T;
  }
  return undefined as T;
}

export const api = {
  getHealth: () => req<EngineHealth>("/api/health"),
  getConfig: () => req<AppConfig>("/api/config"),
  saveConfig: (cfg: AppConfig) =>
    req<AppConfig>("/api/config", { method: "PUT", body: JSON.stringify(cfg) }),
  getSources: () => req<SourceStatus[]>("/api/sensors/sources"),
  getReadings: () => req<SensorReading[]>("/api/sensors/readings"),
  getObsCatalog: () => req<ObsCatalog>("/api/obs/catalog"),
  getPreviewGrid: () => req<GridPreviewResponse>("/api/preview/grid"),
  testAction: (act: ActionConfig) =>
    req<void>("/api/actions/test", { method: "POST", body: JSON.stringify(act) }),
  setTheme: (name: string) => req<void>(`/api/theme/${encodeURIComponent(name)}`, { method: "POST" }),
  setBrightness: (val: number) => req<void>(`/api/brightness/${val}`, { method: "POST" }),
  setPage: (name: string) => req<void>(`/api/page/${encodeURIComponent(name)}`, { method: "POST" }),
  setProfile: (name: string) => req<void>(`/api/profile/${encodeURIComponent(name)}`, { method: "POST" }),
  importManifest: (path: string, profile_id: string, profile_name: string) =>
    req<AppConfig>("/api/import/manifest", {
      method: "POST",
      body: JSON.stringify({ path, profile_id, profile_name }),
    }),
  lookupWeatherCep: (cep: string) =>
    req<AppConfig>("/api/weather/cep", {
      method: "POST",
      body: JSON.stringify({ cep }),
    }),
  exportBackup: () => req<{ exported_at: string; version: string; config: AppConfig }>("/api/backup/export"),
  importBackup: (payload: unknown) =>
    req<AppConfig>("/api/backup/import", { method: "POST", body: JSON.stringify(payload) }),
  getBackupSources: () => req<BackupCandidate[]>("/api/backup/sources"),
  restoreBackup: (path: string) =>
    req<AppConfig>("/api/backup/restore", { method: "POST", body: JSON.stringify({ path }) }),
  getAutostart: () => req<AutostartStatus>("/api/system/autostart"),
  setAutostart: (engine_enabled: boolean, dash_enabled: boolean) =>
    req<AutostartStatus>("/api/system/autostart", {
      method: "POST",
      body: JSON.stringify({ engine_enabled, dash_enabled }),
    }),
  runSensorDoctor: () => req<void>("/api/system/sensor-doctor", { method: "POST" }),
  checkUpdate: () => req<UpdateCheckResponse>("/api/system/update/check"),
  installUpdate: (download_url: string, sha256?: string | null) =>
    req<void>("/api/system/update/install", {
      method: "POST",
      body: JSON.stringify({ download_url, sha256: sha256 ?? null }),
    }),
  getUpdateStatus: () => req<UpdateInstallStatus>("/api/system/update/status"),
};
