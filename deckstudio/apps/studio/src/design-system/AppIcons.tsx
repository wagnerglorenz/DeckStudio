import React from "react";

export interface IconCatalogEntry {
  id: string;
  label: string;
  category: "apps" | "obs" | "media";
  color: string;
}

export const ICON_CATALOG: IconCatalogEntry[] = [
  // Apps Populares
  { id: "obs", label: "OBS Studio", category: "apps", color: "#38BDF8" },
  { id: "discord", label: "Discord", category: "apps", color: "#5865F2" },
  { id: "spotify", label: "Spotify", category: "apps", color: "#1DB954" },
  { id: "chrome", label: "Chrome", category: "apps", color: "#4285F4" },
  { id: "edge", label: "Edge", category: "apps", color: "#0EA5E9" },
  { id: "vscode", label: "VS Code", category: "apps", color: "#007ACC" },
  { id: "steam", label: "Steam", category: "apps", color: "#66C0F4" },
  { id: "twitch", label: "Twitch", category: "apps", color: "#9146FF" },
  { id: "youtube", label: "YouTube", category: "apps", color: "#EF4444" },
  { id: "nvidia", label: "NVIDIA", category: "apps", color: "#76B900" },
  { id: "terminal", label: "Terminal", category: "apps", color: "#22D3EE" },
  { id: "powershell", label: "PowerShell", category: "apps", color: "#60A5FA" },
  { id: "cursor", label: "Cursor / IDE", category: "apps", color: "#22D3EE" },
  { id: "docker", label: "Docker", category: "apps", color: "#38BDF8" },
  { id: "git", label: "Git", category: "apps", color: "#F97316" },
  { id: "notion", label: "Notion", category: "apps", color: "#E2E8F0" },
  { id: "slack", label: "Slack / Chat", category: "apps", color: "#38BDF8" },
  { id: "teams", label: "Teams", category: "apps", color: "#6366F1" },
  { id: "zoom", label: "Zoom", category: "apps", color: "#3B82F6" },
  { id: "epic", label: "Epic Games", category: "apps", color: "#E2E8F0" },
  { id: "battlenet", label: "Battle.net", category: "apps", color: "#38BDF8" },
  { id: "firefox", label: "Firefox", category: "apps", color: "#F97316" },
  { id: "brave", label: "Brave", category: "apps", color: "#FB923C" },
  { id: "photoshop", label: "Photoshop", category: "apps", color: "#38BDF8" },
  { id: "premiere", label: "Premiere", category: "apps", color: "#A855F7" },
  { id: "davinci", label: "DaVinci", category: "apps", color: "#F59E0B" },
  { id: "blender", label: "Blender", category: "apps", color: "#F97316" },
  { id: "vlc", label: "VLC", category: "apps", color: "#F97316" },
  { id: "explorer", label: "Pastas", category: "apps", color: "#FBBF24" },
  // OBS & Gravação
  { id: "record", label: "Gravar", category: "obs", color: "#EF4444" },
  { id: "pause", label: "Pausar", category: "obs", color: "#EAB308" },
  { id: "studio", label: "Cena", category: "obs", color: "#38BDF8" },
  { id: "game_cam", label: "Jogo+Cam", category: "obs", color: "#38BDF8" },
  { id: "split", label: "Split", category: "obs", color: "#38BDF8" },
  { id: "focus", label: "Foco", category: "obs", color: "#38BDF8" },
  { id: "multi", label: "Multi", category: "obs", color: "#A855F7" },
  { id: "cam4k", label: "Cam 4K", category: "obs", color: "#38BDF8" },
  { id: "track", label: "Track", category: "obs", color: "#38BDF8" },
  // Mídia & Sistema
  { id: "audio", label: "Áudio", category: "media", color: "#38BDF8" },
  { id: "volume_up", label: "Vol +", category: "media", color: "#22D3EE" },
  { id: "volume_down", label: "Vol -", category: "media", color: "#22D3EE" },
  { id: "mute_all", label: "Mutar Tudo", category: "media", color: "#EF4444" },
  { id: "audio_switch", label: "Trocar Saída", category: "media", color: "#38BDF8" },
  { id: "mic", label: "Mic", category: "media", color: "#22C55E" },
  { id: "play", label: "Play/Pause", category: "media", color: "#22D3EE" },
  { id: "next", label: "Próxima", category: "media", color: "#22D3EE" },
  { id: "prev", label: "Anterior", category: "media", color: "#22D3EE" },
  { id: "cpu", label: "CPU", category: "media", color: "#22D3EE" },
  { id: "gpu", label: "GPU", category: "media", color: "#22C55E" },
  { id: "ram", label: "RAM", category: "media", color: "#EAB308" },
  { id: "fan", label: "Fan", category: "media", color: "#38BDF8" },
  { id: "lock_pc", label: "Bloquear PC", category: "media", color: "#F59E0B" },
  { id: "screenshot", label: "Print", category: "media", color: "#22D3EE" },
  { id: "clipboard", label: "Clipboard", category: "media", color: "#94A3B8" },
  { id: "stopwatch", label: "Cronômetro", category: "media", color: "#F59E0B" },
  { id: "web", label: "Web / URL", category: "media", color: "#38BDF8" },
  { id: "lightbulb", label: "Luz / Casa", category: "media", color: "#FBBF24" },
  { id: "theme", label: "Tema", category: "media", color: "#A855F7" },
  { id: "folder", label: "Pasta", category: "media", color: "#F59E0B" },
  { id: "logo", label: "Logo", category: "media", color: "#94A3B8" },
];

export function AppLogoIcon({ name, size = 20 }: { name?: string | null; size?: number }) {
  const s = size;
  switch (name) {
    case "obs":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label="OBS Studio">
          <circle cx="12" cy="12" r="10.5" fill="#0B0E14" stroke="#CBD5E1" strokeWidth="1.5" />
          <path d="M12 4.5a4.5 4.5 0 0 1 3.9 6.75" stroke="#FFFFFF" strokeWidth="2.2" strokeLinecap="round" />
          <path d="M18.5 15.75a4.5 4.5 0 0 1-7.8 0" stroke="#FFFFFF" strokeWidth="2.2" strokeLinecap="round" />
          <path d="M5.5 15.75a4.5 4.5 0 0 1 3.9-6.75" stroke="#FFFFFF" strokeWidth="2.2" strokeLinecap="round" />
        </svg>
      );
    case "discord":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label="Discord">
          <rect x="2" y="2" width="20" height="20" rx="6" fill="#5865F2" />
          <rect x="5" y="7" width="14" height="9.5" rx="4.5" fill="#FFFFFF" />
          <circle cx="9.5" cy="11.5" r="1.6" fill="#5865F2" />
          <circle cx="14.5" cy="11.5" r="1.6" fill="#5865F2" />
        </svg>
      );
    case "spotify":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label="Spotify">
          <circle cx="12" cy="12" r="10.5" fill="#1DB954" />
          <path d="M7 9.5c3.5-1 7.2-.7 10.2 1" stroke="#09130D" strokeWidth="1.9" strokeLinecap="round" />
          <path d="M7.8 12.6c2.9-.8 5.9-.5 8.4.8" stroke="#09130D" strokeWidth="1.7" strokeLinecap="round" />
          <path d="M8.6 15.5c2.2-.6 4.5-.4 6.5.6" stroke="#09130D" strokeWidth="1.5" strokeLinecap="round" />
        </svg>
      );
    case "chrome":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label="Chrome">
          <circle cx="12" cy="12" r="10" fill="#FFFFFF" />
          <path d="M12 2a10 10 0 0 1 8.66 5H12z" fill="#EA4335" />
          <path d="M3.34 7A10 10 0 0 1 12 2v5H7.67z" fill="#EA4335" />
          <path d="M20.66 7A10 10 0 0 1 12 22l4.33-7.5z" fill="#FBBC05" />
          <path d="M12 22A10 10 0 0 1 3.34 7l4.33 7.5z" fill="#34A853" />
          <circle cx="12" cy="12" r="4.8" fill="#FFFFFF" />
          <circle cx="12" cy="12" r="3.6" fill="#4285F4" />
        </svg>
      );
    case "edge":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label="Edge">
          <circle cx="12" cy="12" r="10" fill="#0C4A6E" />
          <path d="M4.5 14.5C5.5 8.5 11.5 5 17 7.5c2 1 3 3 2.5 5H10.5c-1.5 0-2.5 1-2.5 2.5 0 2.5 3 4 6.5 3.5" stroke="#38BDF8" strokeWidth="2.8" strokeLinecap="round" />
          <circle cx="11.5" cy="13.5" r="2.5" fill="#22C55E" />
        </svg>
      );
    case "vscode":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label="VS Code">
          <rect x="2" y="2" width="20" height="20" rx="5" fill="#0369A1" />
          <path d="M5.5 9.5L16.5 4.5l3 1.5v12l-3 1.5L5.5 14.5" stroke="#FFFFFF" strokeWidth="2" strokeLinejoin="round" />
          <path d="M16.5 5v14" stroke="#38BDF8" strokeWidth="2.2" />
        </svg>
      );
    case "steam":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label="Steam">
          <circle cx="12" cy="12" r="10.5" fill="#1B2838" stroke="#66C0F4" strokeWidth="1.4" />
          <circle cx="15" cy="9" r="3" stroke="#FFFFFF" strokeWidth="1.8" />
          <circle cx="9" cy="15.5" r="2.2" fill="#FFFFFF" />
          <path d="M4 12.5l5 3 6-6.5" stroke="#FFFFFF" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      );
    case "twitch":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label="Twitch">
          <rect x="2" y="2" width="20" height="20" rx="5" fill="#9146FF" />
          <rect x="5.5" y="5.5" width="13" height="11" rx="2" fill="#FFFFFF" />
          <rect x="9.5" y="8.5" width="1.8" height="4.5" rx="0.6" fill="#9146FF" />
          <rect x="13.2" y="8.5" width="1.8" height="4.5" rx="0.6" fill="#9146FF" />
        </svg>
      );
    case "youtube":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label="YouTube">
          <rect x="2" y="4.5" width="20" height="15" rx="4.5" fill="#DC2626" />
          <path d="M10 8.5l6 3.5-6 3.5z" fill="#FFFFFF" />
        </svg>
      );
    case "nvidia":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label="NVIDIA">
          <rect x="2" y="3.5" width="20" height="17" rx="4" fill="#3F6212" stroke="#84CC16" strokeWidth="1.4" />
          <path d="M6 12c2.5-3.5 7.5-4 11-1-2 3.5-6.5 4.5-11 1z" stroke="#FFFFFF" strokeWidth="1.8" />
          <circle cx="12.5" cy="11.5" r="1.8" fill="#BEF264" />
        </svg>
      );
    case "terminal":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label="Terminal">
          <rect x="2.5" y="4" width="19" height="16" rx="3.5" fill="#0F172A" stroke="#38BDF8" strokeWidth="1.5" />
          <path d="M6.5 9l3.5 3-3.5 3M12 15h5" stroke="#22D3EE" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      );
    case "explorer":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label="Explorador">
          <path d="M3 6.5A2 2 0 0 1 5 4.5h4.2l1.8 2H19a2 2 0 0 1 2 2V17.5a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" fill="#FBBF24" />
        </svg>
      );
    case "record":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label="Gravar">
          <circle cx="12" cy="12" r="8.5" stroke="#EF4444" strokeWidth="2" />
          <circle cx="12" cy="12" r="4.5" fill="#EF4444" />
        </svg>
      );
    case "pause":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label="Pausar">
          <rect x="6.5" y="5" width="4" height="14" rx="1.2" fill="#EAB308" />
          <rect x="13.5" y="5" width="4" height="14" rx="1.2" fill="#EAB308" />
        </svg>
      );
    case "mic":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label="Microfone">
          <rect x="9" y="3.5" width="6" height="10" rx="3" fill="#22C55E" />
          <path d="M6 10.5a6 6 0 0 0 12 0M12 16.5v4M8.5 20.5h7" stroke="#22C55E" strokeWidth="1.8" strokeLinecap="round" />
        </svg>
      );
    case "play":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label="Play/Pause">
          <path d="M5 6.5l8 5.5-8 5.5z" fill="#22D3EE" />
          <rect x="15" y="7" width="2.2" height="10" rx="0.8" fill="#22D3EE" />
          <rect x="18.5" y="7" width="2.2" height="10" rx="0.8" fill="#22D3EE" />
        </svg>
      );
    case "next":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label="Próxima">
          <path d="M5 6.5l9 5.5-9 5.5z" fill="#22D3EE" />
          <rect x="16" y="6.5" width="2.8" height="11" rx="1" fill="#22D3EE" />
        </svg>
      );
    case "prev":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label="Anterior">
          <rect x="5.2" y="6.5" width="2.8" height="11" rx="1" fill="#22D3EE" />
          <path d="M19 6.5l-9 5.5 9 5.5z" fill="#22D3EE" />
        </svg>
      );
    case "audio":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label="Áudio">
          <path d="M4 9.5h4l5-4v13l-5-4H4z" fill="#38BDF8" />
          <path d="M16 8.5a5 5 0 0 1 0 7M18.5 6a8.5 8.5 0 0 1 0 12" stroke="#38BDF8" strokeWidth="1.8" strokeLinecap="round" />
        </svg>
      );
    case "volume_up":
    case "volume_down":
    case "mute_all":
    case "audio_switch":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label={name}>
          <path d="M4 9.5h4l5-4v13l-5-4H4z" fill={name === "mute_all" ? "#EF4444" : "#22D3EE"} />
          {name === "mute_all" ? (
            <path d="M16 9l5 6M21 9l-5 6" stroke="#EF4444" strokeWidth="2" strokeLinecap="round" />
          ) : (
            <path d="M16.5 9a4.5 4.5 0 0 1 0 6" stroke="#22D3EE" strokeWidth="2" strokeLinecap="round" />
          )}
        </svg>
      );
    case "cursor":
    case "windsurf":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label="Cursor">
          <polygon points="12,2 21,7 21,17 12,22 3,17 3,7" stroke="#22D3EE" strokeWidth="1.8" fill="#0B1220" />
          <polygon points="9,8 17,12 9,16" fill="#22D3EE" />
        </svg>
      );
    case "docker":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label="Docker">
          <rect x="4" y="8" width="4" height="3.5" rx="0.6" fill="#38BDF8" />
          <rect x="9" y="8" width="4" height="3.5" rx="0.6" fill="#38BDF8" />
          <rect x="14" y="8" width="4" height="3.5" rx="0.6" fill="#38BDF8" />
          <rect x="9" y="4" width="4" height="3.5" rx="0.6" fill="#38BDF8" />
          <path d="M3 13h17c0 4-3 7-8.5 7S3 17 3 13z" fill="#0EA5E9" />
        </svg>
      );
    case "git":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label="Git">
          <circle cx="7" cy="6" r="2.5" stroke="#F97316" strokeWidth="1.8" />
          <circle cx="7" cy="18" r="2.5" stroke="#F97316" strokeWidth="1.8" />
          <circle cx="17" cy="10" r="2.5" stroke="#22D3EE" strokeWidth="1.8" />
          <path d="M7 8.5v7M7 14c4 0 6-1.5 8-3" stroke="#E2E8F0" strokeWidth="1.8" strokeLinecap="round" />
        </svg>
      );
    case "powershell":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label="PowerShell">
          <rect x="2.5" y="4" width="19" height="16" rx="3" fill="#1D4ED8" stroke="#60A5FA" strokeWidth="1.5" />
          <path d="M7 9l4 3-4 3M12.5 15h4.5" stroke="#FFFFFF" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      );
    case "cpu":
    case "gpu":
    case "ram":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label={name}>
          <rect x="6" y="6" width="12" height="12" rx="2" stroke={name === "gpu" ? "#22C55E" : name === "ram" ? "#EAB308" : "#22D3EE"} strokeWidth="1.8" />
          <rect x="9.5" y="9.5" width="5" height="5" rx="1" fill={name === "gpu" ? "#22C55E" : name === "ram" ? "#EAB308" : "#22D3EE"} />
          <path d="M9 3v3M15 3v3M9 18v3M15 18v3" stroke="#94A3B8" strokeWidth="1.6" strokeLinecap="round" />
        </svg>
      );
    case "lock_pc":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label="Bloquear">
          <path d="M7 11V8a5 5 0 0 1 10 0v3" stroke="#F59E0B" strokeWidth="2" strokeLinecap="round" />
          <rect x="5" y="11" width="14" height="10" rx="2.5" fill="#F59E0B" />
        </svg>
      );
    case "stopwatch":
    case "timer":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label="Cronômetro">
          <circle cx="12" cy="13" r="8" stroke="#F59E0B" strokeWidth="2" />
          <path d="M12 9v4l3 2M9.5 3h5" stroke="#F59E0B" strokeWidth="2" strokeLinecap="round" />
        </svg>
      );
    case "lightbulb":
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label="Luz">
          <circle cx="12" cy="10" r="5.5" stroke="#FBBF24" strokeWidth="2" />
          <rect x="9" y="17" width="6" height="3.5" rx="1" fill="#FBBF24" />
        </svg>
      );
    default:
      return (
        <svg width={s} height={s} viewBox="0 0 24 24" fill="none" aria-label={name ?? "Ícone"}>
          <rect x="4" y="5" width="16" height="14" rx="3" stroke="#94A3B8" strokeWidth="1.8" />
          <circle cx="12" cy="12" r="2.5" fill="#38BDF8" />
        </svg>
      );
  }
}
