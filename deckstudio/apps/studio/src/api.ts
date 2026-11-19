import type {
  ActionConfig,
  AppConfig,
  EngineHealth,
  GridPreviewResponse,
  ObsCatalog,
  SensorReading,
  SourceStatus,
} from "./types";

const BASE = "http://127.0.0.1:19876";

async function req<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    ...init,
    headers: {
      "Content-Type": "application/json",
      ...(init?.headers ?? {}),
    },
  });
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
  importAjazz: (path: string, profile_id: string, profile_name: string) =>
    req<AppConfig>("/api/import/ajazz", {
      method: "POST",
      body: JSON.stringify({ path, profile_id, profile_name }),
    }),
};
