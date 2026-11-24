import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api } from "./api";
import { PAGE_TEMPLATES, POPULAR_APPS } from "./appCatalog";
import { AppLogoIcon, Button, Card, DeckLogo, ICON_CATALOG, KpiCard, ObsSourcesInspector, StatusPill, UiIcon, type UiIconName } from "./design-system";
import type {
  ActionConfig,
  ActionStep,
  AppConfig,
  AutostartStatus,
  BackupCandidate,
  EngineHealth,
  GridPreviewResponse,
  ObsCatalog,
  SensorReading,
  SourceStatus,
  TileConfig,
  UpdateCheckResponse,
  UpdateInstallStatus,
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
    label: "OBS: Ativar/Desativar Fonte",
    badge: "SRC",
    color: "#38BDF8",
    tile: { type: "button", title: "Fonte", subtitle: "TOGGLE", icon: "cam4k", unit: "", decimals: 0, is_temp: false, action: { kind: "obs", op: "toggle_source", source: "" } },
  },
  {
    label: "OBS: Mutar Áudio",
    badge: "VOL",
    color: "#38BDF8",
    tile: { type: "button", title: "Som", subtitle: "ATIVO", icon: "audio", unit: "", decimals: 0, is_temp: false, state: "mute", action: { kind: "obs", op: "toggle_mute", input: "Áudio do Jogo (A50 X Game)" } },
  },
  {
    label: "OBS: Transmissão (Live)",
    badge: "LIVE",
    color: "#EC4899",
    tile: { type: "button", title: "Live", subtitle: "STREAM", icon: "studio", unit: "", decimals: 0, is_temp: false, action: { kind: "obs", op: "toggle_stream" } },
  },
  {
    label: "OBS: Câmera Virtual",
    badge: "CAM",
    color: "#22D3EE",
    tile: { type: "button", title: "Cam Virt", subtitle: "LIGAR", icon: "cam4k", unit: "", decimals: 0, is_temp: false, action: { kind: "obs", op: "toggle_virtualcam" } },
  },
  {
    label: "OBS: Salvar Replay",
    badge: "RPL",
    color: "#F59E0B",
    tile: { type: "button", title: "Replay", subtitle: "SALVAR", icon: "multi", unit: "", decimals: 0, is_temp: false, action: { kind: "obs", op: "save_replay" } },
  },
  {
    label: "OBS: Modo Estúdio",
    badge: "STD",
    color: "#8B5CF6",
    tile: { type: "button", title: "Estúdio", subtitle: "MODO", icon: "studio", unit: "", decimals: 0, is_temp: false, action: { kind: "obs", op: "toggle_studio_mode" } },
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
    label: "Abrir App: OBS Studio",
    badge: "OBS",
    color: "#38BDF8",
    tile: { type: "button", title: "OBS", subtitle: "ABRIR", icon: "obs", unit: "", decimals: 0, is_temp: false, action: { kind: "run", program: "C:\\Program Files\\obs-studio\\bin\\64bit\\obs64.exe", args: [] } },
  },
  {
    label: "Abrir App: Discord",
    badge: "APP",
    color: "#5865F2",
    tile: { type: "button", title: "Discord", subtitle: "ABRIR", icon: "discord", unit: "", decimals: 0, is_temp: false, action: { kind: "run", program: "%LOCALAPPDATA%\\Discord\\Update.exe", args: ["--processStart", "Discord.exe"] } },
  },
  {
    label: "Abrir App: Spotify",
    badge: "APP",
    color: "#1DB954",
    tile: { type: "button", title: "Spotify", subtitle: "ABRIR", icon: "spotify", unit: "", decimals: 0, is_temp: false, action: { kind: "run", program: "%APPDATA%\\Spotify\\Spotify.exe", args: [] } },
  },
  {
    label: "Abrir App: Google Chrome",
    badge: "WEB",
    color: "#4285F4",
    tile: { type: "button", title: "Chrome", subtitle: "ABRIR", icon: "chrome", unit: "", decimals: 0, is_temp: false, action: { kind: "run", program: "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe", args: [] } },
  },
  {
    label: "Abrir App: VS Code",
    badge: "DEV",
    color: "#007ACC",
    tile: { type: "button", title: "VS Code", subtitle: "ABRIR", icon: "vscode", unit: "", decimals: 0, is_temp: false, action: { kind: "run", program: "%LOCALAPPDATA%\\Programs\\Microsoft VS Code\\Code.exe", args: [] } },
  },
  {
    label: "Abrir App: Steam",
    badge: "GAM",
    color: "#66C0F4",
    tile: { type: "button", title: "Steam", subtitle: "ABRIR", icon: "steam", unit: "", decimals: 0, is_temp: false, action: { kind: "run", program: "C:\\Program Files (x86)\\Steam\\steam.exe", args: [] } },
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
  const [savedCfg, setSavedCfg] = useState<AppConfig | null>(null);
  const [cfg, setCfg] = useState<AppConfig | null>(null);
  const [history, setHistory] = useState<AppConfig[]>([]);
  const [future, setFuture] = useState<AppConfig[]>([]);
  const [saving, setSaving] = useState(false);
  const [draftPageId, setDraftPageId] = useState<string | null>(null);
  const [showIconPicker, setShowIconPicker] = useState(false);
  const [showPageModal, setShowPageModal] = useState(false);
  const [selectedTemplateId, setSelectedTemplateId] = useState<string>("obs");
  const [newPageSlug, setNewPageSlug] = useState<string>("obs");

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
  const [manifestPath, setManifestPath] = useState(
    "%APPDATA%\\HotSpot\\StreamDock\\profiles\\<seu-perfil>.sdProfile\\manifest.json",
  );
  const [toast, setToast] = useState<string>("");
  const [multiSteps, setMultiSteps] = useState<ActionStep[]>([
    { step_kind: "action", action: { kind: "obs", op: "scene", name: "2. ACC - Jogo Puro + Cam Sobreposta (16:9)" } },
    { step_kind: "delay", ms: 500 },
    { step_kind: "action", action: { kind: "obs", op: "toggle_record" } },
  ]);
  const [multiName, setMultiName] = useState("Modo Gravar Corrida (ACC)");
  const [autostart, setAutostart] = useState<AutostartStatus | null>(null);
  const [updateInfo, setUpdateInfo] = useState<UpdateCheckResponse | null>(null);
  const [updateStatus, setUpdateStatus] = useState<UpdateInstallStatus | null>(null);
  const [updateDismissed, setUpdateDismissed] = useState(false);
  const [checkingUpdate, setCheckingUpdate] = useState(false);
  const [cepSearching, setCepSearching] = useState(false);
  const [copiedTile, setCopiedTile] = useState<TileConfig | null>(null);
  const [backupSources, setBackupSources] = useState<BackupCandidate[]>([]);

  const lastEditGroupRef = useRef<{ key: string; ts: number }>({ key: "", ts: 0 });
  const toastTimerRef = useRef<number | null>(null);

  const notify = useCallback((msg: string) => {
    setToast(msg);
    if (toastTimerRef.current !== null) window.clearTimeout(toastTimerRef.current);
    toastTimerRef.current = window.setTimeout(() => setToast(""), 3200);
  }, []);

  const refreshRuntime = useCallback(async () => {
    try {
      const [h, p] = await Promise.all([api.getHealth(), api.getPreviewGrid()]);
      setHealth(h);
      setPreview(p);
    } catch {
      // Ignora falha transitória
    }
  }, []);

  const refreshAll = useCallback(async () => {
    try {
      const [c, h, p, s, r, o] = await Promise.all([
        api.getConfig(),
        api.getHealth(),
        api.getPreviewGrid(),
        api.getSources(),
        api.getReadings(),
        api.getObsCatalog(),
      ]);
      setSavedCfg(c);
      setCfg(c);
      setHistory([]);
      setFuture([]);
      setHealth(h);
      setPreview(p);
      setSources(s);
      setReadings(r);
      setObsCat(o);
      api.getAutostart().then(setAutostart).catch(() => {});
      api.checkUpdate().then(setUpdateInfo).catch(() => {});
      api.getBackupSources().then(setBackupSources).catch(() => {});
    } catch {
      // Engine pode estar inicializando
    }
  }, []);

  useEffect(() => {
    refreshAll();
    const id = setInterval(async () => {
      try {
        const [h, p, s, r, o] = await Promise.all([
          api.getHealth(),
          api.getPreviewGrid(),
          api.getSources(),
          api.getReadings(),
          api.getObsCatalog(),
        ]);
        setHealth(h);
        setPreview(p);
        setSources(s);
        setReadings(r);
        setObsCat(o);
      } catch {
        // Ignora falha transitória
      }
    }, 1000);
    return () => clearInterval(id);
  }, [refreshAll]);

  const isDirty = useMemo(() => {
    if (!cfg || !savedCfg) return false;
    return JSON.stringify(cfg) !== JSON.stringify(savedCfg);
  }, [cfg, savedCfg]);

  const applyDraft = useCallback(
    (next: AppConfig, okMsg?: string, groupKey?: string) => {
      if (!cfg) return;
      const now = Date.now();
      const shouldCoalesce =
        Boolean(groupKey) &&
        lastEditGroupRef.current.key === groupKey &&
        now - lastEditGroupRef.current.ts < 700;

      if (!shouldCoalesce) {
        setHistory((prev) => [...prev.slice(-59), cfg]);
      }
      lastEditGroupRef.current = { key: groupKey ?? "", ts: now };
      setFuture([]);
      setCfg(next);
      if (okMsg) notify(okMsg);
    },
    [cfg, notify],
  );

  const undo = useCallback(() => {
    if (!cfg || history.length === 0) return;
    const prev = history[history.length - 1];
    setHistory((h) => h.slice(0, -1));
    setFuture((f) => [cfg, ...f]);
    setCfg(prev);
    lastEditGroupRef.current = { key: "", ts: 0 };
    notify("Ação desfeita (clique em Salvar para aplicar)");
  }, [cfg, history, notify]);

  const redo = useCallback(() => {
    if (!cfg || future.length === 0) return;
    const next = future[0];
    setFuture((f) => f.slice(1));
    setHistory((h) => [...h, cfg]);
    setCfg(next);
    lastEditGroupRef.current = { key: "", ts: 0 };
    notify("Ação refeita (clique em Salvar para aplicar)");
  }, [cfg, future, notify]);

  const saveAll = useCallback(async () => {
    if (!cfg || saving) return;
    setSaving(true);
    try {
      const saved = await api.saveConfig(cfg);
      setSavedCfg(saved);
      setCfg(saved);
      await refreshRuntime();
      notify("Alterações salvas e aplicadas ao vivo");
    } catch (e) {
      notify(`Erro ao salvar: ${String(e)}`);
    } finally {
      setSaving(false);
    }
  }, [cfg, saving, refreshRuntime, notify]);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey)) return;
      const k = e.key.toLowerCase();
      if (k === "z" && !e.shiftKey) {
        e.preventDefault();
        undo();
      } else if (k === "y" || (k === "z" && e.shiftKey)) {
        e.preventDefault();
        redo();
      } else if (k === "s") {
        e.preventDefault();
        saveAll();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [undo, redo, saveAll]);

  const activeProfileId = health?.active_profile ?? cfg?.active_profile ?? "default";
  const currentProfile = cfg?.profiles[activeProfileId];
  const profilePageKeys = Object.keys(currentProfile?.pages ?? {});
  const activePageId =
    draftPageId && currentProfile?.pages[draftPageId]
      ? draftPageId
      : health?.active_page && currentProfile?.pages[health.active_page]
        ? health.active_page
        : profilePageKeys[0] ?? "monitor";
  const currentPageTiles = currentProfile?.pages[activePageId] ?? {};
  const selectedTile = currentPageTiles[selectedKey];

  const updateSelectedTile = (patch: Partial<TileConfig>, groupKey?: string) => {
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
    applyDraft(nextCfg, undefined, groupKey);
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
    applyDraft(
      {
        ...cfg,
        profiles: {
          ...cfg.profiles,
          [activeProfileId]: {
            ...currentProfile,
            pages: { ...currentProfile.pages, [activePageId]: pageMap },
          },
        },
      },
      "Posição atualizada (rascunho)",
    );
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
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          {toast && <StatusPill accent="var(--ds-brand-cyan)">{toast}</StatusPill>}
          <Button
            onClick={undo}
            disabled={history.length === 0}
            title="Voltar / Desfazer última ação (Ctrl+Z)"
            style={{ display: "inline-flex", alignItems: "center", gap: 5 }}
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M9 14L4 9l5-5" />
              <path d="M4 9h10.5a5.5 5.5 0 0 1 0 11H11" />
            </svg>
            <span>Desfazer</span>
          </Button>
          <Button
            onClick={redo}
            disabled={future.length === 0}
            title="Ir para frente / Refazer ação (Ctrl+Y)"
            style={{ display: "inline-flex", alignItems: "center", gap: 5 }}
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M15 14l5-5-5-5" />
              <path d="M20 9H9.5a5.5 5.5 0 0 0 0 11H13" />
            </svg>
            <span>Refazer</span>
          </Button>
          <Button
            variant={isDirty ? "success" : "default"}
            onClick={saveAll}
            disabled={!isDirty || saving}
            title="Salvar alterações no Engine (Ctrl+S)"
            style={{ display: "inline-flex", alignItems: "center", gap: 5 }}
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" />
              <polyline points="17 21 17 13 7 13 7 21" />
              <polyline points="7 3 7 8 15 8" />
            </svg>
            <span>{saving ? "Salvando..." : isDirty ? "Salvar*" : "Salvo"}</span>
          </Button>
        </div>
      </div>

      <div className="main-area">
        {/* Left Nav Rail */}
        <div className="nav-rail">
          {[
            { id: "editor" as TabId, label: "Editor", icon: "grid" as UiIconName },
            { id: "themes" as TabId, label: "Temas", icon: "sliders" as UiIconName },
            { id: "profiles" as TabId, label: "Perfis", icon: "activity" as UiIconName },
            { id: "sources" as TabId, label: "Fontes", icon: "plug" as UiIconName },
            { id: "health" as TabId, label: "Saúde", icon: "heart" as UiIconName },
            { id: "about" as TabId, label: "Sobre", icon: "info" as UiIconName },
          ].map((item) => (
            <button
              key={item.id}
              className={`nav-btn ${tab === item.id ? "active" : ""}`}
              onClick={() => setTab(item.id)}
            >
              <UiIcon name={item.icon} size={18} />
              <span>{item.label}</span>
            </button>
          ))}
        </div>

        <div className="workspace">
          {updateInfo?.update_available && !updateDismissed && (
            <div
              style={{
                background: "var(--ds-bg-elevated)",
                borderBottom: "1px solid var(--ds-border-default)",
                padding: "8px 20px",
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                gap: 12,
                fontSize: 12.5,
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <UiIcon name="download" size={16} />
                <span>
                  <strong style={{ fontWeight: 600 }}>Nova versão v{updateInfo.latest_version} disponível</strong>{" "}
                  <span style={{ color: "var(--ds-text-muted)" }}>
                    (Instalada: v{updateInfo.current_version})
                  </span>
                </span>
                {updateStatus && updateStatus.state !== "idle" && (
                  <span style={{ color: "var(--ds-brand-cyan)", fontWeight: 500, fontVariantNumeric: "tabular-nums" }}>
                    {updateStatus.message} ({updateStatus.progress_pct}%)
                  </span>
                )}
              </div>
              <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                <button
                  className="btn"
                  style={{ fontSize: 11, padding: "4px 12px" }}
                  onClick={() => setTab("about")}
                >
                  Ver Novidades
                </button>
                <button
                  className="btn primary"
                  style={{ fontSize: 11, padding: "4px 12px" }}
                  disabled={updateStatus?.state === "downloading" || updateStatus?.state === "verifying" || updateStatus?.state === "installing"}
                  onClick={async () => {
                    try {
                      await api.installUpdate(updateInfo.download_url, updateInfo.sha256);
                      const poll = window.setInterval(async () => {
                        try {
                          const st = await api.getUpdateStatus();
                          setUpdateStatus(st);
                          if (st.state === "error" || st.state === "installing") {
                            window.clearInterval(poll);
                          }
                        } catch {
                          window.clearInterval(poll);
                        }
                      }, 500);
                    } catch (e) {
                      notify(`Erro ao iniciar atualização: ${String(e)}`);
                    }
                  }}
                >
                  {updateStatus?.state === "downloading" || updateStatus?.state === "verifying"
                    ? `Atualizando (${updateStatus.progress_pct}%)...`
                    : "Atualizar Agora"}
                </button>
                <button
                  className="btn"
                  style={{ fontSize: 11, padding: "4px 8px" }}
                  onClick={() => setUpdateDismissed(true)}
                  title="Ocultar aviso nesta sessão"
                  aria-label="Ocultar aviso"
                >
                  <UiIcon name="close" size={14} />
                </button>
              </div>
            </div>
          )}
          {/* ==================== TAB 1: EDITOR ==================== */}
          {tab === "editor" && (
            <>
              <div className="topbar">
                <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
                  <select
                    value={activeProfileId}
                    onChange={(e) => {
                      setDraftPageId(null);
                      api.setProfile(e.target.value).then(refreshRuntime);
                    }}
                  >
                    {Object.entries(cfg.profiles).map(([pid, p]) => (
                      <option key={pid} value={pid}>Perfil: {p.name}</option>
                    ))}
                  </select>
                  <select
                    value={cfg.active_theme}
                    onChange={(e) => {
                      const nextTheme = e.target.value;
                      setCfg({ ...cfg, active_theme: nextTheme });
                      setSavedCfg((prev) => (prev ? { ...prev, active_theme: nextTheme } : prev));
                      api.setTheme(nextTheme).then(refreshRuntime);
                    }}
                  >
                    {Object.keys(cfg.themes).map((t) => {
                      const labels: Record<string, string> = {
                        llm_dash: "LLM Dashboard",
                        classic_spark: "Spark Clássico",
                        cyberpunk: "Cyberpunk",
                        matrix: "Matrix",
                        obsidian_pro: "Obsidian Pro",
                      };
                      return (
                        <option key={t} value={t}>Tema: {labels[t] ?? t}</option>
                      );
                    })}
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
                        {profilePageKeys.map((pg) => (
                          <div key={pg} className="page-pill-wrap">
                            <button
                              className={`page-pill ${activePageId === pg ? "active" : ""}`}
                              onClick={() => {
                                setDraftPageId(pg);
                                if (savedCfg?.profiles[activeProfileId]?.pages[pg]) {
                                  api.setPage(pg).then(refreshRuntime);
                                }
                              }}
                            >
                              {pg.toUpperCase()}
                            </button>
                            {profilePageKeys.length > 1 && (
                              <button
                                className="page-del-btn"
                                title={`Remover página ${pg}`}
                                onClick={() => {
                                  if (!currentProfile) return;
                                  const nextPages = { ...currentProfile.pages };
                                  delete nextPages[pg];
                                  const fallback = Object.keys(nextPages)[0] ?? null;
                                  setDraftPageId(fallback);
                                  applyDraft(
                                    {
                                      ...cfg,
                                      profiles: {
                                        ...cfg.profiles,
                                        [activeProfileId]: { ...currentProfile, pages: nextPages },
                                      },
                                    },
                                    `Página "${pg}" removida (rascunho)`,
                                  );
                                }}
                              >
                                ×
                              </button>
                            )}
                          </div>
                        ))}
                        <button
                          className="page-pill"
                          onClick={() => {
                            setSelectedTemplateId("obs");
                            setNewPageSlug(currentProfile?.pages["obs"] ? "obs-2" : "obs");
                            setShowPageModal(true);
                          }}
                        >
                          + Página / Modelo
                        </button>
                      </div>
                      <span style={{ fontSize: 11, color: "#94A3B8" }}>
                        {isDirty ? "Alterações não salvas · Ctrl+Z desfaz · Ctrl+S salva" : "Arraste para trocar de posição · Clique para editar"}
                      </span>
                    </div>

                    <div className="deck-grid-wrap">
                      <div className="deck-keys">
                        {GRID_KEYS.map((k) => {
                          const draftTile = currentPageTiles[k];
                          const savedTile = savedCfg?.profiles[activeProfileId]?.pages[activePageId]?.[k];
                          const tileChanged =
                            preview?.page !== activePageId ||
                            JSON.stringify(draftTile ?? null) !== JSON.stringify(savedTile ?? null);
                          const b64 = !tileChanged ? preview?.tiles_b64[k] : undefined;
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
                              ) : draftTile ? (
                                <div style={{ display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: 4, padding: 4, textAlign: "center" }}>
                                  {draftTile.custom_image_b64 && /^data:image\/(png|jpeg|jpg|webp|gif);base64,[A-Za-z0-9+/=]+$/i.test(draftTile.custom_image_b64) ? (
                                    <img src={draftTile.custom_image_b64} alt={draftTile.title} style={{ width: 32, height: 32, objectFit: "contain", borderRadius: 4 }} />
                                  ) : (
                                    <AppLogoIcon name={draftTile.icon ?? (draftTile.type === "sensor" ? "nvidia" : "logo")} size={26} />
                                  )}
                                  <span style={{ fontSize: 10.5, fontWeight: 700, color: "var(--ds-text-primary)", lineHeight: 1.1 }}>
                                    {draftTile.title || draftTile.widget || draftTile.id || k}
                                  </span>
                                  {draftTile.subtitle && (
                                    <span style={{ fontSize: 9, color: "var(--ds-brand-cyan)", lineHeight: 1 }}>
                                      {draftTile.subtitle}
                                    </span>
                                  )}
                                </div>
                              ) : (
                                <span style={{ fontSize: 11, color: "#475569" }}>{k}</span>
                              )}
                            </div>
                          );
                        })}
                      </div>

                      <div className="deck-strip">
                        {STRIP_KEYS.map((k) => {
                          const draftTile = currentPageTiles[k];
                          const savedTile = savedCfg?.profiles[activeProfileId]?.pages[activePageId]?.[k];
                          const tileChanged =
                            preview?.page !== activePageId ||
                            JSON.stringify(draftTile ?? null) !== JSON.stringify(savedTile ?? null);
                          const b64 = !tileChanged ? preview?.tiles_b64[k] : undefined;
                          return (
                            <div
                              key={k}
                              className={`tile-slot strip-slot ${selectedKey === k ? "selected" : ""}`}
                              onClick={() => setSelectedKey(k)}
                            >
                              {b64 ? (
                                <img src={`data:image/png;base64,${b64}`} alt={k} />
                              ) : draftTile ? (
                                <span style={{ fontSize: 10, fontWeight: 700, color: "var(--ds-brand-cyan)", textAlign: "center", padding: 4 }}>
                                  {draftTile.widget ?? draftTile.title ?? k}
                                </span>
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
                        {selectedTile && (
                          <button
                            className="btn"
                            onClick={() => {
                              setCopiedTile(JSON.parse(JSON.stringify(selectedTile)));
                              notify("Tecla copiada");
                            }}
                          >
                            Copiar
                          </button>
                        )}
                        {copiedTile && currentProfile && (
                          <button
                            className="btn"
                            onClick={() => {
                              const p = { ...currentPageTiles, [selectedKey]: JSON.parse(JSON.stringify(copiedTile)) };
                              applyDraft({
                                ...cfg,
                                profiles: {
                                  ...cfg.profiles,
                                  [activeProfileId]: {
                                    ...currentProfile,
                                    pages: { ...currentProfile.pages, [activePageId]: p },
                                  },
                                },
                              }, "Tecla colada (rascunho)");
                            }}
                          >
                            Colar
                          </button>
                        )}
                        <button
                          className="btn danger"
                          onClick={() => {
                            if (!currentProfile) return;
                            const p = { ...currentPageTiles };
                            delete p[selectedKey];
                            applyDraft({
                              ...cfg,
                              profiles: {
                                ...cfg.profiles,
                                [activeProfileId]: {
                                  ...currentProfile,
                                  pages: { ...currentProfile.pages, [activePageId]: p },
                                },
                              },
                            }, "Tecla limpa (rascunho)");
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
                          onChange={(e) => updateSelectedTile({ title: e.target.value }, `title-${selectedKey}`)}
                        />
                      </div>

                      <div className="field">
                        <label>Subtítulo / Widget</label>
                        <input
                          className="input"
                          value={selectedTile?.widget ?? selectedTile?.subtitle ?? ""}
                          placeholder="ex.: clock, obs_status, weather"
                          onChange={(e) => updateSelectedTile({ subtitle: e.target.value }, `sub-${selectedKey}`)}
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
                                onChange={(e) => updateSelectedTile({ unit: e.target.value }, `unit-${selectedKey}`)}
                              />
                              <input
                                className="input"
                                type="number"
                                style={{ flex: 1 }}
                                value={selectedTile.max ?? 100}
                                onChange={(e) => updateSelectedTile({ max: Number(e.target.value) }, `max-${selectedKey}`)}
                              />
                            </div>
                          </div>
                        </>
                      ) : (
                        <>
                          <div className="field" style={{ gridColumn: showIconPicker ? "1 / -1" : undefined }}>
                            <label>Logo / Ícone ou Imagem PNG</label>
                            <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
                              <button
                                type="button"
                                className="btn"
                                style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "space-between", gap: 8 }}
                                onClick={() => setShowIconPicker((v) => !v)}
                              >
                                <span style={{ display: "flex", alignItems: "center", gap: 8 }}>
                                  {selectedTile?.icon ? <AppLogoIcon name={selectedTile.icon} size={18} /> : null}
                                  <span>{selectedTile?.icon ? ICON_CATALOG.find((c) => c.id === selectedTile.icon)?.label ?? selectedTile.icon : "Sem ícone"}</span>
                                </span>
                                <span style={{ fontSize: 10, color: "var(--ds-text-muted)" }}>{showIconPicker ? "▲" : "▼"}</span>
                              </button>
                              <label className="btn" style={{ cursor: "pointer" }}>
                                PNG
                                <input
                                  type="file"
                                  accept="image/png,image/jpeg,image/webp,image/gif"
                                  style={{ display: "none" }}
                                  onChange={(e) => {
                                    const file = e.target.files?.[0];
                                    if (!file) return;
                                    if (file.size > 512 * 1024) {
                                      notify("Imagem excede 512 KiB");
                                      return;
                                    }
                                    const rd = new FileReader();
                                    rd.onload = () => updateSelectedTile({ custom_image_b64: String(rd.result) });
                                    rd.readAsDataURL(file);
                                  }}
                                />
                              </label>
                            </div>
                            {showIconPicker && (
                              <div className="icon-picker-grid" style={{ marginTop: 6 }}>
                                <button
                                  type="button"
                                  className={`icon-picker-btn ${!selectedTile?.icon ? "active" : ""}`}
                                  onClick={() => {
                                    updateSelectedTile({ icon: null, custom_image_b64: null });
                                    setShowIconPicker(false);
                                  }}
                                >
                                  <span style={{ fontSize: 16 }}>∅</span>
                                  <span>Nenhum</span>
                                </button>
                                {ICON_CATALOG.map((ic) => (
                                  <button
                                    type="button"
                                    key={ic.id}
                                    className={`icon-picker-btn ${selectedTile?.icon === ic.id ? "active" : ""}`}
                                    onClick={() => {
                                      updateSelectedTile({ icon: ic.id, custom_image_b64: null });
                                      setShowIconPicker(false);
                                    }}
                                  >
                                    <AppLogoIcon name={ic.id} size={22} />
                                    <span>{ic.label}</span>
                                  </button>
                                ))}
                              </div>
                            )}
                          </div>

                          <div className="field" style={{ gridColumn: "1 / -1" }}>
                            <label>Key Creator — Cores Customizadas da Tecla</label>
                            <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
                              {(
                                [
                                  ["bg_color", "Fundo", "#04070E"],
                                  ["border_color", "Borda", "#22D3EE"],
                                  ["text_color", "Texto", "#FFFFFF"],
                                  ["icon_color", "Ícone", "#FFFFFF"],
                                ] as const
                              ).map(([field, lbl, def]) => (
                                <label
                                  key={field}
                                  style={{
                                    display: "inline-flex",
                                    alignItems: "center",
                                    gap: 5,
                                    fontSize: 11,
                                    color: "var(--ds-text-muted)",
                                    background: "var(--ds-bg-input)",
                                    border: "1px solid var(--ds-border-default)",
                                    borderRadius: "var(--ds-radius-xs)",
                                    padding: "4px 8px",
                                    cursor: "pointer",
                                  }}
                                >
                                  <span>{lbl}</span>
                                  <input
                                    type="color"
                                    value={(selectedTile?.[field] as string | undefined) || def}
                                    onChange={(e) => updateSelectedTile({ [field]: e.target.value })}
                                    style={{ width: 20, height: 20, border: "none", background: "transparent", padding: 0, cursor: "pointer" }}
                                  />
                                </label>
                              ))}
                              {(selectedTile?.bg_color || selectedTile?.border_color || selectedTile?.text_color || selectedTile?.icon_color) && (
                                <button
                                  type="button"
                                  className="btn"
                                  onClick={() =>
                                    updateSelectedTile({
                                      bg_color: null,
                                      border_color: null,
                                      text_color: null,
                                      icon_color: null,
                                    })
                                  }
                                >
                                  Usar Tema
                                </button>
                              )}
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
                                  run: { kind: "run", program: "C:\\Program Files\\obs-studio\\bin\\64bit\\obs64.exe", args: [] },
                                  soundboard: { kind: "soundboard", path: "" },
                                  webhook: { kind: "webhook", url: "http://127.0.0.1:8080/hook", method: "POST" },
                                  theme: { kind: "theme", target: "next" },
                                  audio_app: { kind: "audio_app", op: "mute", process: "spotify.exe", step: 0.08 },
                                  timer: { kind: "timer", op: "toggle", seconds: 1500 },
                                };
                                updateSelectedTile({
                                  action: defaults[k] ?? defaults.obs,
                                  widget: k === "timer" ? "timer" : selectedTile?.widget === "timer" ? null : selectedTile?.widget,
                                });
                              }}
                            >
                              <option value="obs">OBS Studio</option>
                              <option value="page">Trocar Página</option>
                              <option value="hotkey">Atalho (Hotkey)</option>
                              <option value="run">Abrir App / Comando</option>
                              <option value="audio_app">Mixer de Áudio por App (WASAPI)</option>
                              <option value="timer">Cronômetro / Pomodoro</option>
                              <option value="soundboard">Soundboard (WAV)</option>
                              <option value="webhook">Webhook HTTP</option>
                              <option value="theme">Trocar Tema</option>
                            </select>
                          </div>

                          {selectedTile?.action?.kind === "audio_app" && (
                            <div className="field" style={{ gridColumn: "1 / -1" }}>
                              <label>Mixer WASAPI (App .exe ou vazio para Master)</label>
                              <div style={{ display: "flex", gap: 6 }}>
                                <select
                                  value={selectedTile.action.op}
                                  onChange={(e) =>
                                    updateSelectedTile({
                                      action: {
                                        ...(selectedTile.action as Extract<ActionConfig, { kind: "audio_app" }>),
                                        op: e.target.value,
                                      },
                                    })
                                  }
                                >
                                  <option value="mute">Mutar / Desmutar</option>
                                  <option value="vol_up">Aumentar Volume (+8%)</option>
                                  <option value="vol_down">Diminuir Volume (-8%)</option>
                                </select>
                                <input
                                  className="input"
                                  style={{ flex: 1 }}
                                  placeholder="ex: spotify.exe, discord.exe (vazio = Master)"
                                  value={selectedTile.action.process}
                                  onChange={(e) =>
                                    updateSelectedTile({
                                      action: {
                                        ...(selectedTile.action as Extract<ActionConfig, { kind: "audio_app" }>),
                                        process: e.target.value,
                                      },
                                    })
                                  }
                                />
                              </div>
                            </div>
                          )}

                          {selectedTile?.action?.kind === "timer" && (
                            <div className="field" style={{ gridColumn: "1 / -1" }}>
                              <label>Cronômetro / Pomodoro</label>
                              <div style={{ display: "flex", gap: 6 }}>
                                <select
                                  value={selectedTile.action.op}
                                  onChange={(e) =>
                                    updateSelectedTile({
                                      action: {
                                        ...(selectedTile.action as Extract<ActionConfig, { kind: "timer" }>),
                                        op: e.target.value,
                                      },
                                    })
                                  }
                                >
                                  <option value="toggle">Iniciar / Pausar</option>
                                  <option value="pomodoro">Iniciar Pomodoro (25 min)</option>
                                  <option value="reset">Resetar</option>
                                </select>
                                <input
                                  className="input"
                                  type="number"
                                  style={{ width: 90 }}
                                  value={selectedTile.action.seconds}
                                  onChange={(e) =>
                                    updateSelectedTile({
                                      action: {
                                        ...(selectedTile.action as Extract<ActionConfig, { kind: "timer" }>),
                                        seconds: Number(e.target.value),
                                      },
                                    })
                                  }
                                />
                              </div>
                            </div>
                          )}

                          <div className="field" style={{ gridColumn: "1 / -1" }}>
                            <label>Segurar Tecla (Long Press ≥ 500ms)</label>
                            <select
                              value={selectedTile?.long_press_action?.kind ?? "none"}
                              onChange={(e) => {
                                const k = e.target.value;
                                const lpDefaults: Record<string, ActionConfig | null> = {
                                  none: null,
                                  timer_reset: { kind: "timer", op: "reset", seconds: 1500 },
                                  audio_mute: { kind: "audio_app", op: "mute", process: "", step: 0.08 },
                                  theme_next: { kind: "theme", target: "next" },
                                  page_next: { kind: "page", target: "next" },
                                  obs_record: { kind: "obs", op: "toggle_record" },
                                };
                                updateSelectedTile({ long_press_action: lpDefaults[k] ?? null });
                              }}
                            >
                              <option value="none">Desativado (apenas clique normal)</option>
                              <option value="timer_reset">Resetar Cronômetro / Pomodoro</option>
                              <option value="audio_mute">Mutar Áudio Master (WASAPI)</option>
                              <option value="theme_next">Alternar Próximo Tema</option>
                              <option value="page_next">Ir para Próxima Página</option>
                              <option value="obs_record">OBS: Gravar / Parar</option>
                            </select>
                          </div>

                          {selectedTile?.action?.kind === "obs" && (
                            <div className="field">
                              <label>Operação OBS</label>
                              <select
                                value={selectedTile.action.op}
                                onChange={(e) => {
                                  const op = e.target.value;
                                  if (op === "scene") {
                                    updateSelectedTile({
                                      action: { kind: "obs", op: "scene", name: obsCat.scenes[0] ?? "" },
                                    });
                                  } else if (op === "toggle_source" || op === "show_source" || op === "hide_source") {
                                    updateSelectedTile({
                                      action: {
                                        kind: "obs",
                                        op,
                                        source: obsCat.sources?.[0] ?? "",
                                        scene: null,
                                      },
                                    });
                                  } else if (op === "toggle_mute") {
                                    updateSelectedTile({
                                      action: { kind: "obs", op: "toggle_mute", input: obsCat.audio_inputs[0] ?? "Mic/Aux" },
                                    });
                                  } else {
                                    updateSelectedTile({
                                      action: { kind: "obs", op },
                                    });
                                  }
                                }}
                              >
                                <optgroup label="Cenas">
                                  <option value="scene">Trocar Cena</option>
                                  <option value="toggle_studio_mode">Modo Estúdio (Ligar/Desligar)</option>
                                  <option value="trigger_studio_transition">Transição do Modo Estúdio</option>
                                </optgroup>
                                <optgroup label="Fontes &amp; Elementos">
                                  <option value="toggle_source">Ativar / Desativar Fonte na Cena</option>
                                  <option value="show_source">Exibir Fonte (Ligar)</option>
                                  <option value="hide_source">Ocultar Fonte (Desligar)</option>
                                </optgroup>
                                <optgroup label="Gravação &amp; Transmissão">
                                  <option value="toggle_record">Gravar / Parar Gravação</option>
                                  <option value="toggle_record_pause">Pausar / Retomar Gravação</option>
                                  <option value="toggle_stream">Iniciar / Parar Transmissão (Live)</option>
                                  <option value="toggle_virtualcam">Câmera Virtual (Ligar/Desligar)</option>
                                  <option value="save_replay">Salvar Replay Instantâneo</option>
                                  <option value="toggle_replay">Iniciar / Parar Replay Buffer</option>
                                </optgroup>
                                <optgroup label="Áudio">
                                  <option value="toggle_mute">Mutar / Desmutar Áudio</option>
                                </optgroup>
                              </select>
                            </div>
                          )}

                          <div className="field">
                            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                              <label>
                                {selectedTile?.action?.kind === "run"
                                  ? "App Rápido ou Caminho"
                                  : selectedTile?.action?.kind === "obs" && selectedTile.action.op === "scene"
                                    ? "Cena do OBS"
                                    : selectedTile?.action?.kind === "obs" &&
                                      (selectedTile.action.op === "toggle_source" ||
                                        selectedTile.action.op === "show_source" ||
                                        selectedTile.action.op === "hide_source")
                                      ? "Configuração da Fonte"
                                      : selectedTile?.action?.kind === "obs" && selectedTile.action.op === "toggle_mute"
                                        ? "Fonte de Áudio OBS"
                                        : "Parâmetro da Ação"}
                              </label>
                              {selectedTile?.action?.kind === "obs" && (
                                <button
                                  type="button"
                                  className="btn"
                                  style={{ fontSize: 10, padding: "2px 6px" }}
                                  onClick={async () => {
                                    try {
                                      const o = await api.getObsCatalog();
                                      setObsCat(o);
                                      notify(`OBS atualizado: ${o.scenes.length} cenas e ${o.sources?.length ?? 0} fontes`);
                                    } catch (e) {
                                      notify(`Erro ao consultar OBS: ${String(e)}`);
                                    }
                                  }}
                                  title="Recarregar cenas e fontes do OBS agora"
                                >
                                  <UiIcon name="refresh" size={14} /> Atualizar ({obsCat.scenes.length} cenas / {obsCat.sources?.length ?? 0} fontes)
                                </button>
                              )}
                            </div>

                            {selectedTile?.action?.kind === "obs" && selectedTile.action.op === "scene" ? (
                              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                                <select
                                  value={selectedTile.action.name ?? ""}
                                  onChange={(e) =>
                                    updateSelectedTile({ action: { kind: "obs", op: "scene", name: e.target.value } })
                                  }
                                >
                                  <option value="">Selecionar cena do OBS...</option>
                                  {selectedTile.action.name && !obsCat.scenes.includes(selectedTile.action.name) && (
                                    <option value={selectedTile.action.name}>
                                      {selectedTile.action.name} (Atual/Manual)
                                    </option>
                                  )}
                                  {obsCat.scenes.map((sc) => (
                                    <option key={sc} value={sc}>{sc}</option>
                                  ))}
                                </select>
                                <input
                                  className="input"
                                  placeholder="Ou digite o nome exato da cena no OBS"
                                  value={selectedTile.action.name ?? ""}
                                  onChange={(e) =>
                                    updateSelectedTile(
                                      { action: { kind: "obs", op: "scene", name: e.target.value } },
                                      `obs-sc-${selectedKey}`,
                                    )
                                  }
                                />
                              </div>
                            ) : selectedTile?.action?.kind === "obs" &&
                              (selectedTile.action.op === "toggle_source" ||
                                selectedTile.action.op === "show_source" ||
                                selectedTile.action.op === "hide_source") ? (
                              (() => {
                                const obsAct = selectedTile.action as Extract<ActionConfig, { kind: "obs" }>;
                                return (
                                  <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                                    <div>
                                      <span style={{ fontSize: 11, color: "var(--ds-text-secondary)", marginBottom: 3, display: "block" }}>
                                        Fonte / Elemento
                                      </span>
                                      <select
                                        value={obsAct.source ?? obsAct.name ?? ""}
                                        onChange={(e) =>
                                          updateSelectedTile({
                                            action: { ...obsAct, source: e.target.value, name: e.target.value },
                                          })
                                        }
                                      >
                                        <option value="">Selecionar fonte do OBS...</option>
                                        {(obsAct.source || obsAct.name) &&
                                          !obsCat.sources?.includes(obsAct.source || obsAct.name || "") && (
                                            <option value={obsAct.source || obsAct.name || ""}>
                                              {obsAct.source || obsAct.name} (Atual/Manual)
                                            </option>
                                          )}
                                        {obsCat.sources?.map((s) => (
                                          <option key={s} value={s}>{s}</option>
                                        ))}
                                      </select>
                                      <input
                                        className="input"
                                        style={{ marginTop: 4 }}
                                        placeholder="Ou digite o nome da fonte (ex: Brio 4k, Canal, Borda)"
                                        value={obsAct.source ?? obsAct.name ?? ""}
                                        onChange={(e) =>
                                          updateSelectedTile(
                                            { action: { ...obsAct, source: e.target.value, name: e.target.value } },
                                            `obs-src-${selectedKey}`,
                                          )
                                        }
                                      />
                                    </div>
                                    <div>
                                      <span style={{ fontSize: 11, color: "var(--ds-text-secondary)", marginBottom: 3, display: "block" }}>
                                        Cena Específica (Opcional)
                                      </span>
                                      <select
                                        value={obsAct.scene ?? ""}
                                        onChange={(e) =>
                                          updateSelectedTile({
                                            action: { ...obsAct, scene: e.target.value || null },
                                          })
                                        }
                                      >
                                        <option value="">[Cena Ativa no Momento]</option>
                                        {obsCat.scenes.map((sc) => (
                                          <option key={sc} value={sc}>{sc}</option>
                                        ))}
                                      </select>
                                    </div>

                                    <ObsSourcesInspector
                                      sources={obsCat.sources ?? []}
                                      selectedSource={obsAct.source ?? obsAct.name ?? ""}
                                      activeScene={obsAct.scene || obsCat.current_scene}
                                      onSelectSource={(src) => {
                                        updateSelectedTile({
                                          action: { ...obsAct, source: src, name: src },
                                          title: selectedTile.title === "Fonte" || !selectedTile.title ? src : selectedTile.title,
                                        });
                                      }}
                                      onRefresh={async () => {
                                        try {
                                          const o = await api.getObsCatalog();
                                          setObsCat(o);
                                          notify(`OBS atualizado: ${o.scenes.length} cenas e ${o.sources?.length ?? 0} fontes`);
                                        } catch (e) {
                                          notify(`Erro ao consultar OBS: ${String(e)}`);
                                        }
                                      }}
                                    />
                                  </div>
                                );
                              })()
                            ) : selectedTile?.action?.kind === "obs" && selectedTile.action.op === "toggle_mute" ? (
                              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                                <select
                                  value={selectedTile.action.input ?? ""}
                                  onChange={(e) =>
                                    updateSelectedTile({ action: { kind: "obs", op: "toggle_mute", input: e.target.value } })
                                  }
                                >
                                  <option value="">Selecionar entrada de áudio...</option>
                                  {selectedTile.action.input && !obsCat.audio_inputs.includes(selectedTile.action.input) && (
                                    <option value={selectedTile.action.input}>
                                      {selectedTile.action.input} (Atual)
                                    </option>
                                  )}
                                  {obsCat.audio_inputs.map((inp) => (
                                    <option key={inp} value={inp}>{inp}</option>
                                  ))}
                                </select>
                                <input
                                  className="input"
                                  placeholder="Ou digite o nome da fonte de áudio"
                                  value={selectedTile.action.input ?? ""}
                                  onChange={(e) =>
                                    updateSelectedTile(
                                      { action: { kind: "obs", op: "toggle_mute", input: e.target.value } },
                                      `obs-in-${selectedKey}`,
                                    )
                                  }
                                />
                              </div>
                            ) : selectedTile?.action?.kind === "obs" ? (
                              <div style={{ fontSize: 12, color: "var(--ds-text-muted)", padding: "4px 0" }}>
                                {selectedTile.action.op === "toggle_record" && "Alterna gravação do OBS (Gravar / Parar com feedback visual)."}
                                {selectedTile.action.op === "toggle_record_pause" && "Pausa ou retoma a gravação atual do OBS."}
                                {selectedTile.action.op === "toggle_stream" && "Inicia ou interrompe a transmissão (live) no OBS."}
                                {selectedTile.action.op === "toggle_virtualcam" && "Ativa ou desativa a Câmera Virtual do OBS."}
                                {selectedTile.action.op === "save_replay" && "Salva o buffer de replay instantâneo do OBS."}
                                {selectedTile.action.op === "toggle_replay" && "Inicia ou interrompe a captura do buffer de replay."}
                                {selectedTile.action.op === "toggle_studio_mode" && "Alterna a visualização entre Modo Normal e Modo Estúdio."}
                                {selectedTile.action.op === "trigger_studio_transition" && "Dispara a transição da cena prévia para o programa no Modo Estúdio."}
                              </div>
                            ) : selectedTile?.action?.kind === "run" ? (
                              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                                <select
                                  value={
                                    POPULAR_APPS.find(
                                      (a) => selectedTile.action?.kind === "run" && a.program === selectedTile.action.program,
                                    )?.id ?? ""
                                  }
                                  onChange={(e) => {
                                    const preset = POPULAR_APPS.find((a) => a.id === e.target.value);
                                    if (!preset) return;
                                    updateSelectedTile({
                                      type: "button",
                                      title: preset.title,
                                      subtitle: preset.subtitle,
                                      icon: preset.icon,
                                      custom_image_b64: null,
                                      action: { kind: "run", program: preset.program, args: [...preset.args] },
                                    });
                                  }}
                                >
                                  <option value="">Escolher app popular (OBS, Discord, Spotify...)</option>
                                  {POPULAR_APPS.map((a) => (
                                    <option key={a.id} value={a.id}>{a.name}</option>
                                  ))}
                                </select>
                                <input
                                  className="input"
                                  placeholder="Caminho do .exe ou URL"
                                  value={selectedTile.action.program}
                                  onChange={(e) =>
                                    updateSelectedTile(
                                      { action: { kind: "run", program: e.target.value, args: selectedTile.action?.kind === "run" ? selectedTile.action.args : [] } },
                                      `run-prog-${selectedKey}`,
                                    )
                                  }
                                />
                                <input
                                  className="input"
                                  placeholder="Argumentos (separados por espaço)"
                                  value={selectedTile.action.args.join(" ")}
                                  onChange={(e) =>
                                    updateSelectedTile(
                                      {
                                        action: {
                                          kind: "run",
                                          program: selectedTile.action?.kind === "run" ? selectedTile.action.program : "",
                                          args: e.target.value.split(/\s+/).filter(Boolean),
                                        },
                                      },
                                      `run-args-${selectedKey}`,
                                    )
                                  }
                                />
                              </div>
                            ) : (
                              <input
                                className="input"
                                value={
                                  selectedTile?.action?.kind === "hotkey"
                                    ? selectedTile.action.keys.join("+")
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
                                    updateSelectedTile({ action: { kind: "hotkey", keys: v.split("+").map((x) => x.trim()).filter(Boolean) } }, `act-${selectedKey}`);
                                  } else if (act.kind === "page") {
                                    updateSelectedTile({ action: { kind: "page", target: v } }, `act-${selectedKey}`);
                                  } else if (act.kind === "soundboard") {
                                    updateSelectedTile({ action: { kind: "soundboard", path: v } }, `act-${selectedKey}`);
                                  } else if (act.kind === "webhook") {
                                    updateSelectedTile({ action: { ...act, url: v } }, `act-${selectedKey}`);
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
                  {LIBRARY_TEMPLATES.map((tpl, idx) => ({ tpl, idx }))
                    .filter(({ tpl }) => tpl.label.toLowerCase().includes(libFilter.toLowerCase()))
                    .map(({ tpl, idx }) => (
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
                        {tpl.tile.icon ? (
                          <AppLogoIcon name={tpl.tile.icon} size={20} />
                        ) : (
                          <span className="act-badge" style={{ background: `${tpl.color}22`, color: tpl.color }}>{tpl.badge}</span>
                        )}
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
                    {Object.entries(cfg.themes).map(([tname, th]) => {
                      const meta: Record<string, { label: string; desc: string }> = {
                        llm_dash: {
                          label: "LLM Dashboard (OLED)",
                          desc: "Fundo preto profundo sem penumbra + anéis + moldura ciano",
                        },
                        classic_spark: {
                          label: "Spark Clássico",
                          desc: "Sparkline degradê magenta + unidades sobrescritas",
                        },
                        cyberpunk: {
                          label: "Cyberpunk · Night City",
                          desc: "HUD chanfrado + Amarelo Elétrico + Ciano Neon + barras em blocos",
                        },
                        matrix: {
                          label: "Matrix · Terminal TTY",
                          desc: "Scanlines sutis + Verde Fósforo + medidores estilo terminal Unix",
                        },
                        obsidian_pro: {
                          label: "Obsidian Studio Pro",
                          desc: "Preto OLED puro + Titânio + Ouro Quente (conforto noturno)",
                        },
                      };
                      const info = meta[tname] ?? {
                        label: th.name || tname,
                        desc: th.surface_style || th.style,
                      };
                      const isActive = cfg.active_theme === tname;
                      return (
                        <div
                          key={tname}
                          className="row-box"
                          style={{
                            flexDirection: "column",
                            alignItems: "flex-start",
                            gap: 8,
                            background: th.background,
                            borderColor: isActive ? th.accent : th.card_border,
                            boxShadow: isActive ? `0 0 14px ${th.accent}33` : undefined,
                            cursor: "pointer",
                          }}
                          onClick={() => api.setTheme(tname).then(refreshAll)}
                        >
                          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", width: "100%" }}>
                            <strong style={{ color: th.text }}>{info.label}</strong>
                            <div style={{ display: "flex", gap: 4 }}>
                              {[th.background, th.card_border, th.accent, th.chart].map((c, idx) => (
                                <span
                                  key={idx}
                                  style={{
                                    width: 12,
                                    height: 12,
                                    borderRadius: 3,
                                    background: c,
                                    border: "1px solid rgba(255,255,255,0.22)",
                                  }}
                                />
                              ))}
                            </div>
                          </div>
                          <span style={{ fontSize: 12, color: "var(--ds-text-muted)" }}>{info.desc}</span>
                          <span className="pill" style={{ color: isActive ? th.accent : "var(--ds-text-subtle)" }}>
                            {isActive ? "ATIVO" : "Selecionar"}
                          </span>
                        </div>
                      );
                    })}
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
                        onChange={(e) => applyDraft({ ...cfg, color_rules: { ...cfg.color_rules, temp_yellow: Number(e.target.value) } })}
                      />
                      <input
                        className="input"
                        type="number"
                        style={{ width: 64 }}
                        value={cfg.color_rules.temp_red}
                        onChange={(e) => applyDraft({ ...cfg, color_rules: { ...cfg.color_rules, temp_red: Number(e.target.value) } })}
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
                        onChange={(e) => applyDraft({ ...cfg, color_rules: { ...cfg.color_rules, mem_yellow: Number(e.target.value) } })}
                      />
                      <input
                        className="input"
                        type="number"
                        style={{ width: 64 }}
                        value={cfg.color_rules.mem_red}
                        onChange={(e) => applyDraft({ ...cfg, color_rules: { ...cfg.color_rules, mem_red: Number(e.target.value) } })}
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
                        applyDraft({
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
                        applyDraft({
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
                          applyDraft({ ...cfg, smart_profiles: { ...cfg.smart_profiles, rules } });
                        }}
                      >
                        {r.enabled ? "ON" : "OFF"}
                      </button>
                    </div>
                  ))}
                  <div className="row-box" style={{ flexDirection: "column", alignItems: "stretch", gap: 8 }}>
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: 8 }}>
                      <strong style={{ fontSize: 13 }}>Backup Completo, Exportar / Importar &amp; Instalações Anteriores</strong>
                      <div style={{ display: "flex", gap: 6 }}>
                        <button
                          className="btn"
                          onClick={() => api.getBackupSources().then(setBackupSources).catch((e) => notify(`Erro: ${String(e)}`))}
                        >
                          Buscar Anteriores
                        </button>
                        <button
                          className="btn primary"
                          onClick={async () => {
                            try {
                              const pkg = await api.exportBackup();
                              const blob = new Blob([JSON.stringify(pkg, null, 2)], { type: "application/json" });
                              const url = URL.createObjectURL(blob);
                              const a = document.createElement("a");
                              a.href = url;
                              a.download = `DeckStudio-Backup-${new Date().toISOString().slice(0, 10)}.json`;
                              a.click();
                              URL.revokeObjectURL(url);
                              notify("Backup completo exportado");
                            } catch (e) {
                              notify(`Erro ao exportar: ${String(e)}`);
                            }
                          }}
                        >
                          Exportar (.json)
                        </button>
                        <label className="btn success" style={{ cursor: "pointer" }}>
                          Importar (.json)
                          <input
                            type="file"
                            accept=".json,application/json"
                            style={{ display: "none" }}
                            onChange={(e) => {
                              const f = e.target.files?.[0];
                              if (!f) return;
                              const rd = new FileReader();
                              rd.onload = () => {
                                try {
                                  const parsed = JSON.parse(String(rd.result));
                                  api
                                    .importBackup(parsed)
                                    .then((c) => {
                                      setCfg(c);
                                      setSavedCfg(c);
                                      notify("Backup importado e aplicado");
                                    })
                                    .catch((err) => notify(`Erro: ${String(err)}`));
                                } catch {
                                  notify("Arquivo JSON inválido");
                                }
                              };
                              rd.readAsText(f);
                            }}
                          />
                        </label>
                      </div>
                    </div>
                    {backupSources.length > 0 && (
                      <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                        {backupSources.map((b) => (
                          <div key={b.path} className="row-box" style={{ padding: "6px 10px" }}>
                            <div style={{ fontSize: 11.5 }}>
                              <strong>{b.source}</strong> · {b.profiles_count} perfil(is)
                              <div style={{ color: "var(--ds-text-muted)", fontSize: 10.5 }}>{b.path}</div>
                            </div>
                            <button
                              className="btn primary"
                              onClick={() =>
                                api
                                  .restoreBackup(b.path)
                                  .then((c) => {
                                    setCfg(c);
                                    setSavedCfg(c);
                                    notify("Configuração restaurada");
                                  })
                                  .catch((err) => notify(`Erro: ${String(err)}`))
                              }
                            >
                              Restaurar
                            </button>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                  <div className="row-box" style={{ flexDirection: "column", alignItems: "stretch" }}>
                    <strong style={{ fontSize: 13 }}>Importar Perfil (Stream Dock / Mirabox)</strong>
                    <div style={{ display: "flex", gap: 8 }}>
                      <input className="input" style={{ flex: 1 }} value={manifestPath} onChange={(e) => setManifestPath(e.target.value)} />
                      <button
                        className="btn primary"
                        onClick={() =>
                          api
                            .importManifest(manifestPath, "imported_profile", "Perfil Importado")
                            .then((c) => {
                              setCfg(c);
                              notify("Perfil importado");
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
                  <h2>
                    <span>Provedores de Telemetria &amp; Guia Passo a Passo</span>
                    <button
                      type="button"
                      className="btn primary"
                      style={{ fontSize: 11, padding: "5px 10px" }}
                      onClick={async () => {
                        try {
                          await api.runSensorDoctor();
                          notify("Sensor Doctor aberto em uma janela PowerShell");
                        } catch (e) {
                          notify(`Erro ao abrir Sensor Doctor: ${String(e)}`);
                        }
                      }}
                      title="Executa scripts/sensor-doctor.ps1 para diagnosticar AIDA64, LibreHardwareMonitor, HWiNFO e NVML"
                    >
                      <UiIcon name="activity" size={14} /> Rodar Sensor Doctor
                    </button>
                  </h2>
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
                      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: 8 }}>
                        <strong style={{ whiteSpace: "nowrap" }}>{idx + 1}. {src.name}</strong>
                        <span
                          className="pill"
                          style={{
                            color: src.active ? "#22C55E" : "#94A3B8",
                            maxWidth: "100%",
                            whiteSpace: "normal",
                            lineHeight: 1.35,
                          }}
                        >
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
                              applyDraft({
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
                  <div style={{ display: "flex", flexDirection: "column", gap: 8, overflowY: "auto", minHeight: 280, maxHeight: "calc(100vh - 240px)", resize: "vertical" }}>
                    {filteredReadings.slice(0, 60).map((r) => (
                      <div key={`${r.source}_${r.id}`} className="row-box" style={{ padding: "8px 12px", flexWrap: "wrap" }}>
                        <div style={{ display: "flex", gap: 8, alignItems: "center", minWidth: 0, flex: "1 1 180px" }}>
                          <code>{r.id}</code>
                          <span style={{ fontSize: 12, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{r.label}</span>
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
                    <label>CEP (preenche cidade e coordenadas)</label>
                    <div style={{ display: "flex", gap: 8 }}>
                      <input
                        className="input"
                        style={{ flex: 1, fontFamily: "var(--ds-font-mono)", fontVariantNumeric: "tabular-nums" }}
                        placeholder="00000-000"
                        value={cfg.weather.cep ?? ""}
                        onChange={(e) => setCfg({ ...cfg, weather: { ...cfg.weather, cep: e.target.value } })}
                        onKeyDown={async (e) => {
                          if (e.key === "Enter" && !cepSearching) {
                            e.preventDefault();
                            setCepSearching(true);
                            try {
                              const updated = await api.lookupWeatherCep(cfg.weather.cep ?? "");
                              setCfg(updated);
                              notify(`Clima atualizado para ${updated.weather.city}`);
                            } catch (err) {
                              notify(`Erro ao buscar CEP: ${String(err)}`);
                            } finally {
                              setCepSearching(false);
                            }
                          }
                        }}
                      />
                      <Button
                        variant="primary"
                        disabled={cepSearching}
                        onClick={async () => {
                          setCepSearching(true);
                          try {
                            const updated = await api.lookupWeatherCep(cfg.weather.cep ?? "");
                            setCfg(updated);
                            notify(`Clima atualizado para ${updated.weather.city}`);
                          } catch (err) {
                            notify(`Erro ao buscar CEP: ${String(err)}`);
                          } finally {
                            setCepSearching(false);
                          }
                        }}
                      >
                        {cepSearching ? "Buscando..." : "Buscar CEP"}
                      </Button>
                    </div>
                  </div>
                  <div className="field">
                    <label>Cidade (Exibida no Card 5,2)</label>
                    <input
                      className="input"
                      value={cfg.weather.city}
                      onChange={(e) => applyDraft({ ...cfg, weather: { ...cfg.weather, city: e.target.value } })}
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
                        onChange={(e) => applyDraft({ ...cfg, weather: { ...cfg.weather, lat: Number(e.target.value) } })}
                      />
                    </div>
                    <div className="field">
                      <label>Longitude</label>
                      <input
                        className="input"
                        type="number"
                        step="0.0001"
                        value={cfg.weather.lon}
                        onChange={(e) => applyDraft({ ...cfg, weather: { ...cfg.weather, lon: Number(e.target.value) } })}
                      />
                    </div>
                  </div>
                  <div className="grid-2">
                    <div className="field">
                      <label>OBS Host</label>
                      <input
                        className="input"
                        value={cfg.obs.host}
                        onChange={(e) => applyDraft({ ...cfg, obs: { ...cfg.obs, host: e.target.value } })}
                      />
                    </div>
                    <div className="field">
                      <label>OBS Porta WebSocket</label>
                      <input
                        className="input"
                        type="number"
                        value={cfg.obs.port}
                        onChange={(e) => applyDraft({ ...cfg, obs: { ...cfg.obs, port: Number(e.target.value) } })}
                      />
                    </div>
                  </div>
                  <div className="field">
                    <label>Atualização do Display &amp; Dash (Polling)</label>
                    <select
                      value={cfg.device.refresh_ms}
                      onChange={(e) =>
                        applyDraft({ ...cfg, device: { ...cfg.device, refresh_ms: Number(e.target.value) } })
                      }
                    >
                      <option value={250}>250 ms (4x por segundo · Ultra rápido)</option>
                      <option value={500}>500 ms (2x por segundo · Responsivo)</option>
                      <option value={1000}>1000 ms (1x por segundo · Padrão)</option>
                      <option value={2000}>2000 ms (A cada 2 segundos · Econômico)</option>
                    </select>
                  </div>
                  <div style={{ borderTop: "1px solid var(--ds-border-subtle)", paddingTop: 10, marginTop: 4, display: "flex", flexDirection: "column", gap: 8 }}>
                    <label style={{ fontSize: 11, fontWeight: 700, color: "var(--ds-text-muted)", textTransform: "uppercase" }}>
                      Inicialização Automática no Boot do Windows
                    </label>
                    <label style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 12.5, cursor: "pointer" }}>
                      <input
                        type="checkbox"
                        checked={autostart?.engine_enabled ?? false}
                        onChange={async (e) => {
                          try {
                            const res = await api.setAutostart(e.target.checked, autostart?.dash_enabled ?? false);
                            setAutostart(res);
                            notify(res.engine_enabled ? "Motor adicionado ao boot do Windows" : "Motor removido do boot do Windows");
                          } catch (err) {
                            notify(`Erro ao configurar boot: ${String(err)}`);
                          }
                        }}
                      />
                      <span>Iniciar Motor na Bandeja com o Windows (acende o LCD ao ligar o PC)</span>
                    </label>
                    <label style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 12.5, cursor: "pointer" }}>
                      <input
                        type="checkbox"
                        checked={autostart?.dash_enabled ?? false}
                        onChange={async (e) => {
                          try {
                            const res = await api.setAutostart(autostart?.engine_enabled ?? true, e.target.checked);
                            setAutostart(res);
                            notify(res.dash_enabled ? "Dash configurada para abrir ao ligar o PC" : "Abertura automática da Dash desativada");
                          } catch (err) {
                            notify(`Erro ao configurar boot da Dash: ${String(err)}`);
                          }
                        }}
                      />
                      <span>Abrir janela da Dash (Editor) automaticamente ao ligar o Windows</span>
                    </label>
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
                    <h1 style={{ fontSize: 22, fontWeight: 600, color: "var(--ds-text-bright)" }}>DeckStudio v0.4.10</h1>
                    <p style={{ fontSize: 13.5, color: "var(--ds-text-muted)", marginTop: 4 }}>
                      Alternativa nativa, leve e 100% gratuita ao software oficial de Stream Docks compatíveis — sem WebView residente e com zero perda de pacotes em rede.
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

              <Card title="Atualizações do Aplicativo (Auto-Updater)">
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: 12, flexWrap: "wrap" }}>
                  <div style={{ fontSize: 12.5, color: "var(--ds-text-secondary)" }}>
                    Versão instalada: <code>v{updateInfo?.current_version ?? "0.4.10"}</code>
                    {updateInfo?.latest_version && (
                      <> · Última verificada: <code>v{updateInfo.latest_version}</code></>
                    )}
                    {updateInfo?.error && (
                      <div style={{ color: "var(--ds-semantic-warning)", fontSize: 12, marginTop: 4, display: "flex", alignItems: "center", gap: 6 }}>
                        <UiIcon name="alert" size={14} /> {updateInfo.error}
                      </div>
                    )}
                  </div>
                  <div style={{ display: "flex", gap: 8 }}>
                    <Button
                      disabled={checkingUpdate}
                      onClick={async () => {
                        setCheckingUpdate(true);
                        try {
                          const res = await api.checkUpdate();
                          setUpdateInfo(res);
                          setUpdateDismissed(false);
                          notify(
                            res.update_available
                              ? `Nova versão v${res.latest_version} disponível`
                              : res.error
                                ? `Aviso: ${res.error}`
                                : `Você já está na versão mais recente (v${res.current_version})`,
                          );
                        } catch (e) {
                          notify(`Erro ao verificar atualização: ${String(e)}`);
                        } finally {
                          setCheckingUpdate(false);
                        }
                      }}
                    >
                      <UiIcon name="refresh" size={14} /> {checkingUpdate ? "Verificando..." : "Verificar Atualizações"}
                    </Button>
                    {updateInfo?.update_available && (
                      <Button
                        variant="primary"
                        disabled={updateStatus?.state === "downloading" || updateStatus?.state === "verifying"}
                        onClick={async () => {
                          await api.installUpdate(updateInfo.download_url, updateInfo.sha256);
                          const poll = window.setInterval(async () => {
                            try {
                              const st = await api.getUpdateStatus();
                              setUpdateStatus(st);
                              if (st.state === "error" || st.state === "installing") window.clearInterval(poll);
                            } catch {
                              window.clearInterval(poll);
                            }
                          }, 500);
                        }}
                      >
                        <UiIcon name="download" size={14} /> Atualizar para v{updateInfo.latest_version}
                      </Button>
                    )}
                  </div>
                </div>
                {updateStatus && updateStatus.state !== "idle" && (
                  <div style={{ marginTop: 10 }}>
                    <div style={{ fontSize: 11.5, marginBottom: 4, color: updateStatus.state === "error" ? "var(--ds-semantic-danger)" : "var(--ds-brand-cyan)" }}>
                      {updateStatus.message} ({updateStatus.progress_pct}%)
                    </div>
                    <div style={{ width: "100%", height: 6, borderRadius: 99, background: "var(--ds-bg-input)", overflow: "hidden" }}>
                      <div style={{ width: `${updateStatus.progress_pct}%`, height: "100%", background: "var(--ds-brand-cyan)", transition: "width 0.2s" }} />
                    </div>
                  </div>
                )}
                {updateInfo?.release_notes && (
                  <pre style={{ marginTop: 10, padding: 10, borderRadius: 8, background: "var(--ds-bg-code)", fontSize: 11.5, color: "var(--ds-text-muted)", whiteSpace: "pre-wrap", maxHeight: 140, overflowY: "auto" }}>
                    {updateInfo.release_notes}
                  </pre>
                )}
                <div className="field" style={{ marginTop: 10 }}>
                  <label>URL do Feed de Releases (GitHub API ou Servidor Próprio)</label>
                  <input
                    className="input"
                    value={cfg.update_feed_url ?? "https://api.github.com/repos/wagnerglorenz/DeckStudio/releases/latest"}
                    onChange={(e) => applyDraft({ ...cfg, update_feed_url: e.target.value })}
                  />
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

      {showPageModal && currentProfile && (
        <div className="modal-backdrop" onClick={() => setShowPageModal(false)}>
          <div className="modal-card" onClick={(e) => e.stopPropagation()}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <strong style={{ fontSize: 16 }}>Adicionar Página Pré-pronta ou em Branco</strong>
              <Button onClick={() => setShowPageModal(false)}>Fechar</Button>
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              {PAGE_TEMPLATES.map((tpl) => {
                const active = selectedTemplateId === tpl.id;
                return (
                  <div
                    key={tpl.id}
                    className="row-box"
                    style={{
                      cursor: "pointer",
                      borderColor: active ? "var(--ds-brand-cyan)" : "var(--ds-border-default)",
                      background: active ? "var(--ds-bg-active)" : "var(--ds-bg-elevated)",
                    }}
                    onClick={() => {
                      setSelectedTemplateId(tpl.id);
                      const base = tpl.defaultSlug;
                      setNewPageSlug(currentProfile.pages[base] ? `${base}-2` : base);
                    }}
                  >
                    <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                      <span className="act-badge" style={{ background: `${tpl.color}22`, color: tpl.color }}>{tpl.badge}</span>
                      <div>
                        <strong style={{ fontSize: 13 }}>{tpl.label}</strong>
                        <div style={{ fontSize: 11.5, color: "var(--ds-text-muted)" }}>{tpl.description}</div>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
            <div className="field">
              <label>Nome / Identificador da Página (ex.: obs, monitor, apps)</label>
              <input
                className="input"
                value={newPageSlug}
                onChange={(e) => setNewPageSlug(e.target.value.replace(/\s+/g, "-").toLowerCase())}
              />
            </div>
            <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
              <Button onClick={() => setShowPageModal(false)}>Cancelar</Button>
              <Button
                variant="primary"
                onClick={() => {
                  const slug = newPageSlug.trim();
                  if (!slug) return;
                  const tpl = PAGE_TEMPLATES.find((t) => t.id === selectedTemplateId) ?? PAGE_TEMPLATES[0];
                  applyDraft(
                    {
                      ...cfg,
                      profiles: {
                        ...cfg.profiles,
                        [activeProfileId]: {
                          ...currentProfile,
                          pages: { ...currentProfile.pages, [slug]: tpl.buildTiles() },
                        },
                      },
                    },
                    `Página "${slug}" adicionada (clique em Salvar para aplicar)`,
                  );
                  setDraftPageId(slug);
                  setShowPageModal(false);
                }}
              >
                Adicionar Página
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
