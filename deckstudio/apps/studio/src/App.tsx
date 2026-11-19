import React, { useEffect, useMemo, useState } from "react";
import { api } from "./api";
import { Button, Card, DeckLogo, KpiCard, StatusPill } from "./design-system";
import type {
  ActionConfig,
  ActionStep,
  AppConfig,
  EngineHealth,
  GridPreviewResponse,
  ObsCatalog,
  SensorReading,
  SourceStatus,
  TileConfig,
} from "./types";

type TabId = "editor" | "themes" | "profiles" | "sources" | "health" | "about";

const GRID_KEYS: string[] = [];
for (let r = 0; r < 3; r++) {
  for (let c = 0; c < 5; c++) GRID_KEYS.push(`${c},${r}`);
}
const STRIP_KEYS = ["5,0", "5,1", "5,2"];

const LIBRARY_TEMPLATES: Array<{ label: string; badge: string; color: string; tile: TileConfig }> = [
  {
    label: "Sensor Hardware (AIDA/NVML)",
    badge: "SNS",
    color: "#22C55E",
    tile: { type: "sensor", title: "GPU °C", subtitle: "", id: "GPU_TEMP", unit: "°", decimals: 0, min: 0, max: 100, group: "gpu", is_temp: true },
  },
  {
    label: "Rede (Download / Upload)",
    badge: "NET",
    color: "#22D3EE",
    tile: { type: "sensor", title: "DOWN", subtitle: "", id: "NET_DL", unit: "Mbps", decimals: 0, min: 0, max: 1000, group: "net_down", is_temp: false },
  },
  {
    label: "OBS: Gravar / Parar",
    badge: "REC",
    color: "#EF4444",
    tile: { type: "button", title: "Gravar", subtitle: "CLIQUE", icon: "record", unit: "", decimals: 0, is_temp: false, state: "record", action: { kind: "obs", op: "toggle_record" } },
  },
  {
    label: "OBS: Pausar Gravação",
    badge: "PAU",
    color: "#EAB308",
    tile: { type: "button", title: "Pausar", subtitle: "REC ATIVO", icon: "pause", unit: "", decimals: 0, is_temp: false, state: "record_pause", action: { kind: "obs", op: "toggle_record_pause" } },
  },
  {
    label: "OBS: Trocar Cena",
    badge: "SCN",
    color: "#38BDF8",
    tile: { type: "button", title: "Cena", subtitle: "OBS", icon: "studio", unit: "", decimals: 0, is_temp: false, action: { kind: "obs", op: "scene", name: "" } },
  },
  {
    label: "OBS: Mutar Áudio",
    badge: "VOL",
    color: "#38BDF8",
    tile: { type: "button", title: "Som", subtitle: "ATIVO", icon: "audio", unit: "", decimals: 0, is_temp: false, state: "mute", action: { kind: "obs", op: "toggle_mute", input: "Áudio do Jogo (A50 X Game)" } },
  },
  {
    label: "Multi-Action (Sequência)",
    badge: "M+",
    color: "#A855F7",
    tile: { type: "button", title: "Macro", subtitle: "SEQ", icon: "multi", unit: "", decimals: 0, is_temp: false, action: { kind: "multi_action", name: "Nova Sequência", steps: [{ step_kind: "delay", ms: 250 }] } },
  },
  {
    label: "Atalho de Teclado (Hotkey)",
    badge: "KEY",
    color: "#94A3B8",
    tile: { type: "button", title: "Atalho", subtitle: "CTRL+S", unit: "", decimals: 0, is_temp: false, action: { kind: "hotkey", keys: ["CTRL", "S"] } },
  },
  {
    label: "Abrir Programa / Discord",
    badge: "APP",
    color: "#5865F2",
    tile: { type: "button", title: "Discord", subtitle: "ABRIR", icon: "discord", unit: "", decimals: 0, is_temp: false, action: { kind: "run", program: "C:\\Users\\wagne\\AppData\\Local\\Discord\\Update.exe", args: ["--processStart", "Discord.exe"] } },
  },
  {
    label: "Trocar Página",
    badge: "PAG",
    color: "#F59E0B",
    tile: { type: "button", title: "Página", subtitle: "IR", widget: "nav_next", unit: "", decimals: 0, is_temp: false, action: { kind: "page", target: "next" } },
  },
  {
    label: "Soundboard (WAV)",
    badge: "WAV",
    color: "#34D399",
    tile: { type: "button", title: "Som", subtitle: "WAV", icon: "audio", unit: "", decimals: 0, is_temp: false, action: { kind: "soundboard", path: "" } },
  },
];

export default function App() {
  const [tab, setTab] = useState<TabId>("editor");
  const [cfg, setCfg] = useState<AppConfig | null>(null);
  const [health, setHealth] = useState<EngineHealth | null>(null);
  const [preview, setPreview] = useState<GridPreviewResponse | null>(null);
  const [sources, setSources] = useState<SourceStatus[]>([]);
  const [readings, setReadings] = useState<SensorReading[]>([]);
  const [obsCat, setObsCat] = useState<ObsCatalog>({ connected: false, current_scene: "", scenes: [], audio_inputs: [] });
  const [selectedKey, setSelectedKey] = useState<string>("1,0");
  const [dragKey, setDragKey] = useState<string | null>(null);
  const [dragLibIdx, setDragLibIdx] = useState<number | null>(null);
  const [libFilter, setLibFilter] = useState("");
  const [sensorSearch, setSensorSearch] = useState("");
  const [sensorSourceFilter, setSensorSourceFilter] = useState("all");
  const [ajazzPath, setAjazzPath] = useState(
    "C:\\Users\\wagne\\AppData\\Roaming\\HotSpot\\StreamDock\\profiles\\VA6217PE-UUVC-466T-47R9-T80MS52R5QEC.sdProfile\\manifest.json",
  );
  const [toast, setToast] = useState<string>("");
  const [multiSteps, setMultiSteps] = useState<ActionStep[]>([
    { step_kind: "action", action: { kind: "obs", op: "scene", name: "2. ACC - Jogo Puro + Cam Sobreposta (16:9)" } },
    { step_kind: "delay", ms: 500 },
    { step_kind: "action", action: { kind: "obs", op: "toggle_record" } },
  ]);
  const [multiName, setMultiName] = useState("Modo Gravar Corrida (ACC)");

  const notify = (msg: string) => {
    setToast(msg);
    setTimeout(() => setToast(""), 3200);
  };

  const refreshAll = async () => {
    try {
      const [c, h, p, s, r, o] = await Promise.all([
        api.getConfig(),
        api.getHealth(),
        api.getPreviewGrid(),
        api.getSources(),
        api.getReadings(),
        api.getObsCatalog(),
      ]);
      setCfg(c);
      setHealth(h);
      setPreview(p);
      setSources(s);
      setReadings(r);
      setObsCat(o);
    } catch {
      // Engine pode estar inicializando
    }
  };

  useEffect(() => {
    refreshAll();
    const id = setInterval(async () => {
      try {
        const [h, p, s, r] = await Promise.all([
          api.getHealth(),
          api.getPreviewGrid(),
          api.getSources(),
          api.getReadings(),
        ]);
        setHealth(h);
        setPreview(p);
        setSources(s);
        setReadings(r);
      } catch {
        // Ignora falha transitória
      }
    }, 1000);
    return () => clearInterval(id);
  }, []);

  const persistConfig = async (next: AppConfig, okMsg = "Salvo e aplicado ao vivo!") => {
    setCfg(next);
    try {
      const saved = await api.saveConfig(next);
      setCfg(saved);
      notify(okMsg);
    } catch (e) {
      notify(`Erro ao salvar: ${String(e)}`);
    }
  };

  const activeProfileId = health?.active_profile ?? cfg?.active_profile ?? "default";
  const activePageId = health?.active_page ?? (cfg ? Object.keys(cfg.profiles[activeProfileId]?.pages ?? {})[0] : "monitor");
  const currentProfile = cfg?.profiles[activeProfileId];
  const currentPageTiles = currentProfile?.pages[activePageId] ?? {};
  const selectedTile = currentPageTiles[selectedKey];

  const updateSelectedTile = (patch: Partial<TileConfig>) => {
    if (!cfg || !currentProfile) return;
    const base: TileConfig = selectedTile ?? {
      type: "button",
      title: "Novo",
      subtitle: "",
      unit: "",
      decimals: 0,
      is_temp: false,
    };
    const nextCfg: AppConfig = {
      ...cfg,
      profiles: {
        ...cfg.profiles,
        [activeProfileId]: {
          ...currentProfile,
          pages: {
            ...currentProfile.pages,
            [activePageId]: {
              ...currentPageTiles,
              [selectedKey]: { ...base, ...patch },
            },
          },
        },
      },
    };
    persistConfig(nextCfg, "Tecla atualizada!");
  };

  const handleDropOnKey = (targetKey: string) => {
    if (!cfg || !currentProfile) return;
    const pageMap = { ...currentPageTiles };
    if (dragKey && dragKey !== targetKey) {
      const src = pageMap[dragKey];
      const dst = pageMap[targetKey];
      if (src) pageMap[targetKey] = src;
      else delete pageMap[targetKey];
      if (dst) pageMap[dragKey] = dst;
      else delete pageMap[dragKey];
    } else if (dragLibIdx !== null) {
      const tpl = LIBRARY_TEMPLATES[dragLibIdx];
      if (tpl) pageMap[targetKey] = JSON.parse(JSON.stringify(tpl.tile));
    }
    setDragKey(null);
    setDragLibIdx(null);
    setSelectedKey(targetKey);
    persistConfig({
      ...cfg,
      profiles: {
        ...cfg.profiles,
        [activeProfileId]: {
          ...currentProfile,
          pages: { ...currentProfile.pages, [activePageId]: pageMap },
        },
      },
    });
  };

  const filteredReadings = useMemo(() => {
    const q = sensorSearch.trim().toLowerCase();
    return readings.filter((r) => {
      if (sensorSourceFilter !== "all" && r.source !== sensorSourceFilter) return false;
      if (!q) return true;
      return r.id.toLowerCase().includes(q) || r.label.toLowerCase().includes(q);
    });
  }, [readings, sensorSearch, sensorSourceFilter]);

  if (!cfg) {
    return (
      <div className="app-shell" style={{ alignItems: "center", justifyContent: "center", gap: 12 }}>
        <DeckLogo size={52} />
        <h2>Conectando ao DeckStudio Engine (127.0.0.1:19876)...</h2>
        <p style={{ color: "var(--ds-text-muted)", fontSize: 13 }}>Certifique-se de que o <code>deck-engine.exe</code> está rodando na bandeja.</p>
      </div>
    );
  }

  return (
    <div className="app-shell">
      {/* Top Status Bar */}
      <div className="status-bar">
        <div className="brand">
          <DeckLogo size={26} />
          <strong>DeckStudio</strong>
          <StatusPill>v0.2.0 · HID 0300:1010 (512B)</StatusPill>
        </div>
        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <StatusPill dot={health?.hid_connected ? "green" : "red"}>
            Engine: {health?.ram_mb.toFixed(1) ?? "?"} MB RAM
          </StatusPill>
          <StatusPill dot={health?.nic_discards === 0 ? "green" : "cyan"}>
            Descartes NIC: {health?.nic_discards ?? 0}
          </StatusPill>
          <StatusPill dot={health?.obs_connected ? "green" : "gray"}>
            OBS v5: {health?.obs_connected ? "Conectado" : "Offline"}
          </StatusPill>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          {toast && <StatusPill accent="var(--ds-brand-cyan)">{toast}</StatusPill>}
          <span style={{ fontSize: 11.5, color: "var(--ds-text-subtle)" }}>Fechar editor = 0 MB WebView2</span>
        </div>
      </div>

      <div className="main-area">
        {/* Left Nav Rail */}
        <div className="nav-rail">
          {[
            { id: "editor" as TabId, label: "Editor", code: "⊞" },
            { id: "themes" as TabId, label: "Temas", code: "◐" },
            { id: "profiles" as TabId, label: "Perfis", code: "⚡" },
            { id: "sources" as TabId, label: "Fontes", code: "🔌" },
            { id: "health" as TabId, label: "Saúde", code: "♡" },
            { id: "about" as TabId, label: "Sobre", code: "ⓘ" },
          ].map((item) => (
            <button
              key={item.id}
              className={`nav-btn ${tab === item.id ? "active" : ""}`}
              onClick={() => setTab(item.id)}
            >
              <span style={{ fontSize: 17 }}>{item.code}</span>
              <span>{item.label}</span>
            </button>
          ))}
        </div>

        <div className="workspace">
          {/* ==================== TAB 1: EDITOR ==================== */}
          {tab === "editor" && (
            <>
              <div className="topbar">
                <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
                  <select
                    value={activeProfileId}
                    onChange={(e) => api.setProfile(e.target.value).then(refreshAll)}
                  >
                    {Object.entries(cfg.profiles).map(([pid, p]) => (
                      <option key={pid} value={pid}>Perfil: {p.name}</option>
                    ))}
                  </select>
                  <select
                    value={cfg.active_theme}
                    onChange={(e) => api.setTheme(e.target.value).then(refreshAll)}
                  >
                    {Object.keys(cfg.themes).map((t) => (
                      <option key={t} value={t}>Tema: {t === "llm_dash" ? "LLM Dashboard" : t}</option>
                    ))}
                  </select>
                </div>
                <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
                  <span style={{ fontSize: 12, color: "#94A3B8" }}>Brilho {health?.brightness ?? cfg.device.brightness}%</span>
                  <input
                    type="range"
                    min={0}
                    max={100}
                    value={health?.brightness ?? cfg.device.brightness}
                    onChange={(e) => api.setBrightness(Number(e.target.value))}
                  />
                  <button
                    className="btn"
                    onClick={() => {
                      const blob = new Blob([JSON.stringify(cfg, null, 2)], { type: "application/json" });
                      const a = document.createElement("a");
                      a.href = URL.createObjectURL(blob);
                      a.download = "deckstudio-config.json";
                      a.click();
                    }}
                  >
                    Exportar JSON
                  </button>
                </div>
              </div>

              <div className="editor-layout">
                <div className="editor-center">
                  <div className="deck-shell">
                    <div style={{ width: "100%", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                      <div className="page-bar">
                        {Object.keys(currentProfile?.pages ?? {}).map((pg) => (
                          <button
                            key={pg}
                            className={`page-pill ${activePageId === pg ? "active" : ""}`}
                            onClick={() => api.setPage(pg).then(refreshAll)}
                          >
                            {pg.toUpperCase()}
                          </button>
                        ))}
                        <button
                          className="page-pill"
                          onClick={() => {
                            const name = prompt("Nome da nova página (sem espaços):", "extras");
                            if (!name || !currentProfile) return;
                            persistConfig({
                              ...cfg,
                              profiles: {
                                ...cfg.profiles,
                                [activeProfileId]: {
                                  ...currentProfile,
                                  pages: { ...currentProfile.pages, [name]: {} },
                                },
                              },
                            });
                          }}
                        >
                          + Nova Página
                        </button>
                      </div>
                      <span style={{ fontSize: 11, color: "#94A3B8" }}>Arraste para trocar de posição · Clique para editar</span>
                    </div>

                    <div className="deck-grid-wrap">
                      <div className="deck-keys">
                        {GRID_KEYS.map((k) => {
                          const b64 = preview?.tiles_b64[k];
                          return (
                            <div
                              key={k}
                              className={`tile-slot ${selectedKey === k ? "selected" : ""}`}
                              draggable
                              onDragStart={() => setDragKey(k)}
                              onDragOver={(e) => e.preventDefault()}
                              onDrop={() => handleDropOnKey(k)}
                              onClick={() => setSelectedKey(k)}
                            >
                              {b64 ? (
                                <img src={`data:image/png;base64,${b64}`} alt={k} />
                              ) : (
                                <span style={{ fontSize: 11, color: "#475569" }}>{k}</span>
                              )}
                            </div>
                          );
                        })}
                      </div>

                      <div className="deck-strip">
                        {STRIP_KEYS.map((k) => {
                          const b64 = preview?.tiles_b64[k];
                          return (
                            <div
                              key={k}
                              className={`tile-slot strip-slot ${selectedKey === k ? "selected" : ""}`}
                              onClick={() => setSelectedKey(k)}
                            >
                              {b64 ? (
                                <img src={`data:image/png;base64,${b64}`} alt={k} />
                              ) : (
                                <span style={{ fontSize: 11, color: "#475569" }}>{k}</span>
                              )}
                            </div>
                          );
                        })}
                      </div>
                    </div>
                  </div>

                  {/* Property Inspector */}
                  <div className="inspector">
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                      <strong>
                        <span className="pill" style={{ marginRight: 8, color: "#22D3EE" }}>Posição {selectedKey}</span>
                        {selectedTile ? `${selectedTile.type.toUpperCase()} — ${selectedTile.title || selectedTile.widget || "Slot"}` : "Tecla Vazia"}
                      </strong>
                      <div style={{ display: "flex", gap: 8 }}>
                        {selectedTile?.action && (
                          <button className="btn" onClick={() => api.testAction(selectedTile.action!)}>
                            Testar Ação
                          </button>
                        )}
                        <button
                          className="btn danger"
                          onClick={() => {
                            if (!currentProfile) return;
                            const p = { ...currentPageTiles };
                            delete p[selectedKey];
                            persistConfig({
                              ...cfg,
                              profiles: {
                                ...cfg.profiles,
                                [activeProfileId]: {
                                  ...currentProfile,
                                  pages: { ...currentProfile.pages, [activePageId]: p },
                                },
                              },
                            }, "Tecla limpa!");
                          }}
                        >
                          Limpar Tecla
                        </button>
                      </div>
                    </div>

                    <div className="insp-grid">
                      <div className="field">
                        <label>Tipo</label>
                        <select
                          value={selectedTile?.type ?? "button"}
                          onChange={(e) => updateSelectedTile({ type: e.target.value })}
                        >
                          <option value="sensor">Sensor</option>
                          <option value="button">Botão / Ação</option>
                          <option value="folder">Pasta</option>
                        </select>
                      </div>

                      <div className="field">
                        <label>Título na Tecla</label>
                        <input
                          className="input"
                          value={selectedTile?.title ?? ""}
                          onChange={(e) => updateSelectedTile({ title: e.target.value })}
                        />
                      </div>

                      <div className="field">
                        <label>Subtítulo / Widget</label>
                        <input
                          className="input"
                          value={selectedTile?.widget ?? selectedTile?.subtitle ?? ""}
                          placeholder="ex.: clock, obs_status, weather"
                          onChange={(e) => updateSelectedTile({ subtitle: e.target.value })}
                        />
                      </div>

                      {selectedTile?.type === "sensor" ? (
                        <>
                          <div className="field">
                            <label>Sensor (Alias ou ID)</label>
                            <select
                              value={selectedTile.id ?? "GPU_TEMP"}
                              onChange={(e) => updateSelectedTile({ id: e.target.value })}
                            >
                              <optgroup label="Aliases Universais (Fallback Automático)">
                                {["GPU_UTIL", "GPU_TEMP", "GPU_WATT", "GPU_FAN", "VRAM_PCT", "CPU_UTIL", "CPU_TEMP", "CPU_WATT", "CPU_CLOCK", "RAM_UTIL", "NET_DL", "NET_UL", "FPS"].map((id) => (
                                  <option key={id} value={id}>{id}</option>
                                ))}
                              </optgroup>
                              <optgroup label="Sensores Detectados ao Vivo">
                                {readings.map((r) => (
                                  <option key={`${r.source}_${r.id}`} value={r.id}>
                                    {r.id} ({r.label}) [{r.source}]
                                  </option>
                                ))}
                              </optgroup>
                            </select>
                          </div>

                          <div className="field">
                            <label>Estilo &amp; Cor Temperatura</label>
                            <div style={{ display: "flex", gap: 8 }}>
                              <select
                                style={{ flex: 1 }}
                                value={selectedTile.style ?? "card"}
                                onChange={(e) => updateSelectedTile({ style: e.target.value })}
                              >
                                <option value="card">Card Métrico</option>
                                <option value="ring">Anel de Progresso</option>
                              </select>
                              <button
                                className={`btn ${selectedTile.is_temp ? "success" : ""}`}
                                onClick={() => updateSelectedTile({ is_temp: !selectedTile.is_temp })}
                              >
                                {selectedTile.is_temp ? "Temp: ON" : "Temp: OFF"}
                              </button>
                            </div>
                          </div>

                          <div className="field">
                            <label>Unidade / Máximo</label>
                            <div style={{ display: "flex", gap: 6 }}>
                              <input
                                className="input"
                                style={{ width: 70 }}
                                value={selectedTile.unit ?? ""}
                                onChange={(e) => updateSelectedTile({ unit: e.target.value })}
                              />
                              <input
                                className="input"
                                type="number"
                                style={{ flex: 1 }}
                                value={selectedTile.max ?? 100}
                                onChange={(e) => updateSelectedTile({ max: Number(e.target.value) })}
                              />
                            </div>
                          </div>
                        </>
                      ) : (
                        <>
                          <div className="field">
                            <label>Ícone Vetorial ou Imagem PNG</label>
                            <div style={{ display: "flex", gap: 6 }}>
                              <select
                                style={{ flex: 1 }}
                                value={selectedTile?.icon ?? ""}
                                onChange={(e) => updateSelectedTile({ icon: e.target.value || null, custom_image_b64: null })}
                              >
                                <option value="">Sem ícone</option>
                                {["record", "pause", "audio", "discord", "studio", "game_cam", "split", "focus", "multi", "cam4k", "track", "logo", "theme", "folder"].map((ic) => (
                                  <option key={ic} value={ic}>{ic}</option>
                                ))}
                              </select>
                              <label className="btn" style={{ cursor: "pointer" }}>
                                PNG
                                <input
                                  type="file"
                                  accept="image/*"
                                  style={{ display: "none" }}
                                  onChange={(e) => {
                                    const file = e.target.files?.[0];
                                    if (!file) return;
                                    const rd = new FileReader();
                                    rd.onload = () => updateSelectedTile({ custom_image_b64: String(rd.result) });
                                    rd.readAsDataURL(file);
                                  }}
                                />
                              </label>
                            </div>
                          </div>

                          <div className="field">
                            <label>Ação ao Pressionar</label>
                            <select
                              value={selectedTile?.action?.kind ?? "obs"}
                              onChange={(e) => {
                                const k = e.target.value;
                                const defaults: Record<string, ActionConfig> = {
                                  obs: { kind: "obs", op: "scene", name: obsCat.scenes[0] ?? "" },
                                  page: { kind: "page", target: "next" },
                                  hotkey: { kind: "hotkey", keys: ["CTRL", "SHIFT", "M"] },
                                  run: { kind: "run", program: "", args: [] },
                                  soundboard: { kind: "soundboard", path: "" },
                                  webhook: { kind: "webhook", url: "http://127.0.0.1:8080/hook", method: "POST" },
                                  theme: { kind: "theme", target: "next" },
                                };
                                updateSelectedTile({ action: defaults[k] ?? defaults.obs });
                              }}
                            >
                              <option value="obs">OBS Studio</option>
                              <option value="page">Trocar Página</option>
                              <option value="hotkey">Atalho (Hotkey)</option>
                              <option value="run">Abrir App / Comando</option>
                              <option value="soundboard">Soundboard (WAV)</option>
                              <option value="webhook">Webhook HTTP</option>
                              <option value="theme">Trocar Tema</option>
                            </select>
                          </div>

                          <div className="field">
                            <label>Parâmetro da Ação</label>
                            {selectedTile?.action?.kind === "obs" && selectedTile.action.op === "scene" ? (
                              <select
                                value={selectedTile.action.name ?? ""}
                                onChange={(e) =>
                                  updateSelectedTile({ action: { kind: "obs", op: "scene", name: e.target.value } })
                                }
                              >
                                <option value="">Selecionar cena do OBS...</option>
                                {obsCat.scenes.map((sc) => (
                                  <option key={sc} value={sc}>{sc}</option>
                                ))}
                              </select>
                            ) : selectedTile?.action?.kind === "obs" && selectedTile.action.op === "toggle_mute" ? (
                              <select
                                value={selectedTile.action.input ?? ""}
                                onChange={(e) =>
                                  updateSelectedTile({ action: { kind: "obs", op: "toggle_mute", input: e.target.value } })
                                }
                              >
                                {obsCat.audio_inputs.map((inp) => (
                                  <option key={inp} value={inp}>{inp}</option>
                                ))}
                              </select>
                            ) : (
                              <input
                                className="input"
                                value={
                                  selectedTile?.action?.kind === "hotkey"
                                    ? selectedTile.action.keys.join("+")
                                    : selectedTile?.action?.kind === "run"
                                      ? selectedTile.action.program
                                      : selectedTile?.action?.kind === "page"
                                        ? selectedTile.action.target
                                        : selectedTile?.action?.kind === "soundboard"
                                          ? selectedTile.action.path
                                          : selectedTile?.action?.kind === "webhook"
                                            ? selectedTile.action.url
                                            : ""
                                }
                                onChange={(e) => {
                                  const v = e.target.value;
                                  const act = selectedTile?.action;
                                  if (!act) return;
                                  if (act.kind === "hotkey") {
                                    updateSelectedTile({ action: { kind: "hotkey", keys: v.split("+").map((x) => x.trim()).filter(Boolean) } });
                                  } else if (act.kind === "run") {
                                    updateSelectedTile({ action: { ...act, program: v } });
                                  } else if (act.kind === "page") {
                                    updateSelectedTile({ action: { kind: "page", target: v } });
                                  } else if (act.kind === "soundboard") {
                                    updateSelectedTile({ action: { kind: "soundboard", path: v } });
                                  } else if (act.kind === "webhook") {
                                    updateSelectedTile({ action: { ...act, url: v } });
                                  }
                                }}
                              />
                            )}
                          </div>
                        </>
                      )}
                    </div>
                  </div>
                </div>

                {/* Right Action Library */}
                <div className="library">
                  <input
                    className="input"
                    placeholder="Buscar ação na biblioteca..."
                    value={libFilter}
                    onChange={(e) => setLibFilter(e.target.value)}
                  />
                  {LIBRARY_TEMPLATES.filter((t) => t.label.toLowerCase().includes(libFilter.toLowerCase())).map((tpl, idx) => (
                    <div
                      key={tpl.label}
                      className="act-item"
                      draggable
                      onDragStart={() => {
                        setDragLibIdx(idx);
                        setDragKey(null);
                      }}
                      onClick={() => updateSelectedTile(JSON.parse(JSON.stringify(tpl.tile)))}
                    >
                      <span className="act-badge" style={{ background: `${tpl.color}22`, color: tpl.color }}>{tpl.badge}</span>
                      <span>{tpl.label}</span>
                    </div>
                  ))}
                </div>
              </div>
            </>
          )}

          {/* ==================== TAB 2: TEMAS & CORES ==================== */}
          {tab === "themes" && (
            <div className="panel-scroll">
              <div className="grid-2">
                <div className="card">
                  <h2>Temas Visuais (Troca Instantânea)</h2>
                  <div className="grid-2">
                    {Object.entries(cfg.themes).map(([tname, th]) => (
                      <div
                        key={tname}
                        className="row-box"
                        style={{
                          flexDirection: "column",
                          alignItems: "flex-start",
                          borderColor: cfg.active_theme === tname ? "#22D3EE" : "#22324F",
                          cursor: "pointer",
                        }}
                        onClick={() => api.setTheme(tname).then(refreshAll)}
                      >
                        <strong>{tname === "llm_dash" ? "LLM Dashboard" : "AJAZZ Clássico"}</strong>
                        <span style={{ fontSize: 12, color: "#94A3B8" }}>
                          {th.style === "llm_dash" ? "Relevo 3D sem borda + Anéis + Faixa contínua" : "Sparkline degradê magenta + Unidades sobrescritas"}
                        </span>
                        <span className="pill" style={{ color: cfg.active_theme === tname ? "#22D3EE" : "#64748B" }}>
                          {cfg.active_theme === tname ? "ATIVO" : "Selecionar"}
                        </span>
                      </div>
                    ))}
                  </div>
                </div>

                <div className="card">
                  <h2>Regras Globais de Cor &amp; Modo Noturno</h2>
                  <div className="row-box">
                    <div>
                      <strong>Temperaturas (°C) — Amarelo / Vermelho</strong>
                      <div style={{ fontSize: 11, color: "#94A3B8" }}>Verde abaixo · Amarelo entre · Vermelho acima</div>
                    </div>
                    <div style={{ display: "flex", gap: 6 }}>
                      <input
                        className="input"
                        type="number"
                        style={{ width: 64 }}
                        value={cfg.color_rules.temp_yellow}
                        onChange={(e) => persistConfig({ ...cfg, color_rules: { ...cfg.color_rules, temp_yellow: Number(e.target.value) } })}
                      />
                      <input
                        className="input"
                        type="number"
                        style={{ width: 64 }}
                        value={cfg.color_rules.temp_red}
                        onChange={(e) => persistConfig({ ...cfg, color_rules: { ...cfg.color_rules, temp_red: Number(e.target.value) } })}
                      />
                    </div>
                  </div>
                  <div className="row-box">
                    <div>
                      <strong>Memórias (RAM / VRAM %) — Amarelo / Vermelho</strong>
                    </div>
                    <div style={{ display: "flex", gap: 6 }}>
                      <input
                        className="input"
                        type="number"
                        style={{ width: 64 }}
                        value={cfg.color_rules.mem_yellow}
                        onChange={(e) => persistConfig({ ...cfg, color_rules: { ...cfg.color_rules, mem_yellow: Number(e.target.value) } })}
                      />
                      <input
                        className="input"
                        type="number"
                        style={{ width: 64 }}
                        value={cfg.color_rules.mem_red}
                        onChange={(e) => persistConfig({ ...cfg, color_rules: { ...cfg.color_rules, mem_red: Number(e.target.value) } })}
                      />
                    </div>
                  </div>
                  <div className="row-box">
                    <div>
                      <strong>Brilho Automático (Dia {cfg.device.night_mode.day_brightness}% / Noite {cfg.device.night_mode.night_brightness}%)</strong>
                      <div style={{ fontSize: 11, color: "#94A3B8" }}>Escurece automaticamente após {cfg.device.night_mode.night_hour}h</div>
                    </div>
                    <button
                      className={`btn ${cfg.device.night_mode.enabled ? "success" : ""}`}
                      onClick={() =>
                        persistConfig({
                          ...cfg,
                          device: {
                            ...cfg.device,
                            night_mode: { ...cfg.device.night_mode, enabled: !cfg.device.night_mode.enabled },
                          },
                        })
                      }
                    >
                      {cfg.device.night_mode.enabled ? "ATIVADO" : "DESATIVADO"}
                    </button>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* ==================== TAB 3: PERFIS & MULTI-ACTIONS ==================== */}
          {tab === "profiles" && (
            <div className="panel-scroll">
              <div className="grid-2">
                <div className="card">
                  <h2>
                    Smart Profiles (SetWinEventHook · 0% Polling)
                    <button
                      className={`btn ${cfg.smart_profiles.enabled ? "success" : ""}`}
                      onClick={() =>
                        persistConfig({
                          ...cfg,
                          smart_profiles: { ...cfg.smart_profiles, enabled: !cfg.smart_profiles.enabled },
                        })
                      }
                    >
                      {cfg.smart_profiles.enabled ? "ATIVO" : "DESLIGADO"}
                    </button>
                  </h2>
                  {cfg.smart_profiles.rules.map((r, i) => (
                    <div key={r.id} className="row-box">
                      <div>
                        <strong>{r.name}</strong>
                        <div style={{ fontSize: 11.5, color: "#94A3B8" }}>
                          Exe: <code>{r.exe_name}</code> &rarr; {r.profile}/{r.page ?? "padrão"}
                        </div>
                      </div>
                      <button
                        className={`btn ${r.enabled ? "success" : ""}`}
                        onClick={() => {
                          const rules = [...cfg.smart_profiles.rules];
                          rules[i] = { ...r, enabled: !r.enabled };
                          persistConfig({ ...cfg, smart_profiles: { ...cfg.smart_profiles, rules } });
                        }}
                      >
                        {r.enabled ? "ON" : "OFF"}
                      </button>
                    </div>
                  ))}
                  <div className="row-box" style={{ flexDirection: "column", alignItems: "stretch" }}>
                    <strong style={{ fontSize: 13 }}>Importar Perfil do Stream Dock AJAZZ</strong>
                    <div style={{ display: "flex", gap: 8 }}>
                      <input className="input" style={{ flex: 1 }} value={ajazzPath} onChange={(e) => setAjazzPath(e.target.value)} />
                      <button
                        className="btn primary"
                        onClick={() =>
                          api
                            .importAjazz(ajazzPath, "ajazz_imported", "Importado AJAZZ")
                            .then((c) => {
                              setCfg(c);
                              notify("Perfil AJAZZ importado!");
                            })
                            .catch((e) => notify(`Erro: ${String(e)}`))
                        }
                      >
                        Importar
                      </button>
                    </div>
                  </div>
                </div>

                <div className="card">
                  <h2>
                    Construtor de Multi-Action
                    <button
                      className="btn primary"
                      onClick={() => setMultiSteps([...multiSteps, { step_kind: "delay", ms: 300 }])}
                    >
                      + Delay 300ms
                    </button>
                  </h2>
                  <div className="field">
                    <label>Nome da Sequência</label>
                    <input className="input" value={multiName} onChange={(e) => setMultiName(e.target.value)} />
                  </div>
                  {multiSteps.map((st, idx) => (
                    <div key={idx} className="row-box">
                      <span>
                        <strong>#{idx + 1}</strong>{" "}
                        {st.step_kind === "delay"
                          ? `Aguardar ${st.ms} ms`
                          : `Ação: ${JSON.stringify(st.action)}`}
                      </span>
                      <button
                        className="btn danger"
                        onClick={() => setMultiSteps(multiSteps.filter((_, i) => i !== idx))}
                      >
                        Remover
                      </button>
                    </div>
                  ))}
                  <div style={{ display: "flex", gap: 10, justifyContent: "flex-end" }}>
                    <button
                      className="btn"
                      onClick={() => api.testAction({ kind: "multi_action", name: multiName, steps: multiSteps })}
                    >
                      Testar Sequência
                    </button>
                    <button
                      className="btn success"
                      onClick={() =>
                        updateSelectedTile({
                          type: "button",
                          title: multiName.slice(0, 8),
                          subtitle: "MULTI",
                          icon: "multi",
                          action: { kind: "multi_action", name: multiName, steps: multiSteps },
                        })
                      }
                    >
                      Gravar na Tecla {selectedKey}
                    </button>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* ==================== TAB 4: FONTES DE DADOS & GUIAS ==================== */}
          {tab === "sources" && (
            <div className="panel-scroll">
              <div className="grid-2">
                <div className="card">
                  <h2>Provedores de Telemetria &amp; Guia Passo a Passo</h2>
                  <p style={{ fontSize: 12, color: "#94A3B8" }}>
                    Se a fonte #1 estiver fechada, os aliases (<code>CPU_TEMP</code>, <code>GPU_TEMP</code>, etc.) caem automaticamente para a próxima fonte na lista.
                  </p>
                  {sources.map((src, idx) => (
                    <div
                      key={src.id}
                      className="row-box"
                      style={{
                        flexDirection: "column",
                        alignItems: "stretch",
                        borderColor: src.active ? "#22D3EE" : "#22324F",
                      }}
                    >
                      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                        <strong>{idx + 1}. {src.name}</strong>
                        <span className="pill" style={{ color: src.active ? "#22C55E" : "#94A3B8" }}>
                          <span className={`dot ${src.active ? "green" : "gray"}`} />
                          {src.status_text}
                        </span>
                      </div>
                      {src.endpoint && <code>{src.endpoint}</code>}
                      <ol style={{ paddingLeft: 18, fontSize: 11.5, color: "#CBD5E1", lineHeight: 1.5 }}>
                        {src.setup_guide.map((step, si) => (
                          <li key={si}>{step}</li>
                        ))}
                      </ol>
                      {src.id === "librehwm" && (
                        <div style={{ display: "flex", gap: 8 }}>
                          <input
                            className="input"
                            style={{ flex: 1 }}
                            value={cfg.sensor_sources.librehwm_url}
                            onChange={(e) =>
                              persistConfig({
                                ...cfg,
                                sensor_sources: { ...cfg.sensor_sources, librehwm_url: e.target.value },
                              })
                            }
                          />
                        </div>
                      )}
                    </div>
                  ))}
                </div>

                <div className="card">
                  <h2>Explorador de Sensores ao Vivo ({filteredReadings.length})</h2>
                  <div style={{ display: "flex", gap: 8 }}>
                    <input
                      className="input"
                      style={{ flex: 1 }}
                      placeholder="Buscar por ID ou nome (ex.: TCPU, NVML, NET)..."
                      value={sensorSearch}
                      onChange={(e) => setSensorSearch(e.target.value)}
                    />
                    <select value={sensorSourceFilter} onChange={(e) => setSensorSourceFilter(e.target.value)}>
                      <option value="all">Todas Fontes</option>
                      <option value="aida64">AIDA64</option>
                      <option value="native_nvml">Windows + NVML</option>
                      <option value="librehwm">LibreHardwareMonitor</option>
                      <option value="hwinfo">HWiNFO64</option>
                    </select>
                  </div>
                  <div style={{ display: "flex", flexDirection: "column", gap: 6, overflowY: "auto", maxHeight: 580 }}>
                    {filteredReadings.slice(0, 60).map((r) => (
                      <div key={`${r.source}_${r.id}`} className="row-box" style={{ padding: "7px 10px" }}>
                        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                          <code>{r.id}</code>
                          <span style={{ fontSize: 12 }}>{r.label}</span>
                        </div>
                        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                          <span className="pill" style={{ fontSize: 10 }}>{r.source}</span>
                          <strong style={{ minWidth: 66, textAlign: "right" }}>
                            {r.value.toFixed(r.unit === "V" ? 2 : 0)} {r.unit}
                          </strong>
                          <button
                            className="btn"
                            onClick={() => {
                              updateSelectedTile({
                                type: "sensor",
                                title: r.label.slice(0, 9),
                                id: r.id,
                                unit: r.unit,
                                is_temp: r.unit.includes("C"),
                              });
                              setTab("editor");
                            }}
                          >
                            Mapear ({selectedKey})
                          </button>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* ==================== TAB 5: SAÚDE & REDE ==================== */}
          {tab === "health" && (
            <div className="panel-scroll">
              <div className="kpi-grid">
                <KpiCard
                  label="RAM do Engine (24/7)"
                  value={`${health?.ram_mb.toFixed(1) ?? "0"} MB`}
                  hint="App oficial: ~252–450 MB"
                  accent="var(--ds-semantic-success)"
                />
                <KpiCard
                  label="Envio HID (512B)"
                  value={`${health?.hid_tiles_per_s.toFixed(1) ?? "0"} /s`}
                  hint="Rate limit 15/s · Hash diff"
                />
                <KpiCard
                  label="Descartes Realtek"
                  value={health?.nic_discards ?? 0}
                  hint={health?.nic_name || "NIC Ethernet"}
                  accent="var(--ds-semantic-success)"
                />
                <KpiCard
                  label="OBS WebSocket v5"
                  value={<span style={{ fontSize: 18 }}>{health?.obs_connected ? "CONECTADO" : "OFFLINE"}</span>}
                  hint={health?.obs_scene || "---"}
                  accent={health?.obs_connected ? "var(--ds-semantic-success)" : "var(--ds-text-muted)"}
                />
                <KpiCard
                  label="Clima Local"
                  value={
                    <span style={{ fontSize: 20 }}>
                      {health?.weather_temp_c !== null && health?.weather_temp_c !== undefined
                        ? `${health.weather_temp_c.toFixed(1)}°C`
                        : "--"}
                    </span>
                  }
                  hint={cfg.weather.city}
                />
              </div>

              <div className="grid-2">
                <div className="card">
                  <h2>Configuração de Clima &amp; OBS</h2>
                  <div className="field">
                    <label>Cidade (Exibida no Card 5,2)</label>
                    <input
                      className="input"
                      value={cfg.weather.city}
                      onChange={(e) => persistConfig({ ...cfg, weather: { ...cfg.weather, city: e.target.value } })}
                    />
                  </div>
                  <div className="grid-2">
                    <div className="field">
                      <label>Latitude</label>
                      <input
                        className="input"
                        type="number"
                        step="0.0001"
                        value={cfg.weather.lat}
                        onChange={(e) => persistConfig({ ...cfg, weather: { ...cfg.weather, lat: Number(e.target.value) } })}
                      />
                    </div>
                    <div className="field">
                      <label>Longitude</label>
                      <input
                        className="input"
                        type="number"
                        step="0.0001"
                        value={cfg.weather.lon}
                        onChange={(e) => persistConfig({ ...cfg, weather: { ...cfg.weather, lon: Number(e.target.value) } })}
                      />
                    </div>
                  </div>
                  <div className="grid-2">
                    <div className="field">
                      <label>OBS Host</label>
                      <input
                        className="input"
                        value={cfg.obs.host}
                        onChange={(e) => persistConfig({ ...cfg, obs: { ...cfg.obs, host: e.target.value } })}
                      />
                    </div>
                    <div className="field">
                      <label>OBS Porta WebSocket</label>
                      <input
                        className="input"
                        type="number"
                        value={cfg.obs.port}
                        onChange={(e) => persistConfig({ ...cfg, obs: { ...cfg.obs, port: Number(e.target.value) } })}
                      />
                    </div>
                  </div>
                </div>

                <div className="card">
                  <h2>Log ao Vivo do Engine</h2>
                  <div
                    style={{
                      background: "#070B12",
                      border: "1px solid #1E293B",
                      borderRadius: 10,
                      padding: 12,
                      fontFamily: "Consolas, monospace",
                      fontSize: 11.5,
                      lineHeight: 1.6,
                      color: "#94A3B8",
                      height: 260,
                      overflowY: "auto",
                    }}
                  >
                    {(health?.recent_logs ?? []).map((l, i) => (
                      <div key={i}>{l}</div>
                    ))}
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* ==================== TAB 6: SOBRE & COMPATIBILIDADE ==================== */}
          {tab === "about" && (
            <div className="panel-scroll">
              <Card>
                <div className="about-hero">
                  <DeckLogo size={76} />
                  <div style={{ flex: 1 }}>
                    <h1 style={{ fontSize: 22, fontWeight: 800, color: "var(--ds-text-bright)" }}>DeckStudio v0.2.0</h1>
                    <p style={{ fontSize: 13.5, color: "var(--ds-text-muted)", marginTop: 4 }}>
                      Alternativa nativa, leve e 100% gratuita ao software oficial da AJAZZ — sem WebView residente e com zero perda de pacotes em rede.
                    </p>
                    <div className="badge-row">
                      <StatusPill dot="green" accent="var(--ds-semantic-success)">100% Gratuito</StatusPill>
                      <StatusPill accent="var(--ds-brand-cyan)">Licença MIT</StatusPill>
                      <StatusPill>Rust + Tauri v2</StatusPill>
                      <StatusPill>0 MB WebView2 em idle</StatusPill>
                      <StatusPill>Design System</StatusPill>
                    </div>
                  </div>
                  <div style={{ display: "flex", gap: 8 }}>
                    <Button variant="primary" onClick={() => setTab("editor")}>Abrir Editor</Button>
                    <Button onClick={() => setTab("sources")}>Configurar Sensores</Button>
                  </div>
                </div>
              </Card>

              <div className="grid-2">
                <Card title="Compatibilidade de Hardware">
                  <p style={{ fontSize: 12.5, color: "var(--ds-text-muted)", lineHeight: 1.55 }}>
                    Compatível com o protocolo HID vendor-defined (<code>usage_page &gt;= 0xFF00</code>, pacotes 512 bytes) utilizado no hardware de mesa com teclas LCD.
                  </p>
                  <table className="compat-table">
                    <thead>
                      <tr>
                        <th>Modelo / Família</th>
                        <th>VID : PID</th>
                        <th>Layout / Status</th>
                      </tr>
                    </thead>
                    <tbody>
                      <tr>
                        <td><strong>AJAZZ AKP153E</strong></td>
                        <td><code>0300:1010</code></td>
                        <td><StatusPill dot="green">Testado &amp; Verificado (5×3 + faixa 3×1)</StatusPill></td>
                      </tr>
                      <tr>
                        <td>AJAZZ AKP153</td>
                        <td><code>5548:6674</code></td>
                        <td><StatusPill dot="cyan">OEM Mirabox 15+3 (Candidato)</StatusPill></td>
                      </tr>
                      <tr>
                        <td>AJAZZ AKP153E / AKP153R (Rev.)</td>
                        <td><code>0300:1020 / 3010 / 3011</code></td>
                        <td><StatusPill dot="cyan">Revisões irmãs (Candidato)</StatusPill></td>
                      </tr>
                      <tr>
                        <td>Mirabox HSV293S / HSV293SV3</td>
                        <td><code>5548:6670 · 6603:1014 / 1005</code></td>
                        <td><StatusPill dot="cyan">Plataforma base 15+3 (Candidato)</StatusPill></td>
                      </tr>
                      <tr>
                        <td>Rebadges OEM Mirabox (15 teclas)</td>
                        <td><code>Mars Gaming / Soomfon / Womier / RiseMode</code></td>
                        <td><StatusPill dot="cyan">Em análise na doc</StatusPill></td>
                      </tr>
                      <tr>
                        <td>AJAZZ AKP03 / VSDinside N3 · N4</td>
                        <td><code>0300:1001 · 5548:1001 · 6603:1002/1007</code></td>
                        <td><StatusPill dot="gray">Roadmap (Encoders rotativos)</StatusPill></td>
                      </tr>
                    </tbody>
                  </table>
                </Card>

                <Card title="Projeto, Arquitetura &amp; Licença MIT">
                  <div className="row-box">
                    <span>Autor &amp; Mantenedor</span>
                    <strong>Wagner Gadea Lorenz</strong>
                  </div>
                  <div className="row-box">
                    <span>Licença de Uso</span>
                    <strong>MIT (100% Gratuita e Aberta)</strong>
                  </div>
                  <div className="row-box">
                    <span>Arquitetura (Opção 2)</span>
                    <span><code>deck-engine.exe</code> (bandeja) + <code>deck-studio.exe</code> (editor)</span>
                  </div>
                  <div className="row-box">
                    <span>Fontes de Sensores</span>
                    <span>AIDA64 · LibreHWM · HWiNFO64 · Win32 + NVML</span>
                  </div>
                  <div style={{ fontSize: 12, color: "var(--ds-text-muted)", lineHeight: 1.6, marginTop: 4 }}>
                    Consulte <code>docs/COMPATIBILITY.md</code>, <code>docs/DESIGN_SYSTEM.md</code>, <code>docs/EVOLUTION.md</code> e os 10 diagramas Archify em <code>docs/archify/</code> antes de qualquer modificação. Projeto comunitário independente, sem afiliação oficial com AJAZZ ou Mirabox.
                  </div>
                </Card>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
