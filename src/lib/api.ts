import { invoke } from "@tauri-apps/api/core";

export interface ModelInfo { id: string; label: string }
export interface ProviderInfo {
  key: string; label: string; is_cli: boolean; key_url: string | null;
  install_cmd: string | null; note: string | null; models: ModelInfo[];
}
export interface Settings {
  default_provider: string;
  models: Record<string, string>;
  default_tone: string;
  style: string;
  show_pill: boolean;
  launch_at_login: boolean;
  pill_pos: [number, number] | null;
  shortcut_quick: string;
  shortcut_options: string;
  cli_paths: Record<string, string>;
  first_run_done: boolean;
}
export interface OptionsInit {
  has_draft: boolean; chars: number; tones: string[]; default_tone: string;
  providers: ProviderInfo[]; provider: string; models: Record<string, string>;
}

export const TONES = ["Standard", "Professional", "Friendly", "Formal", "Casual",
  "Concise", "Persuasive", "Apologetic", "Assertive", "Grateful"];

export const api = {
  providerList: () => invoke<ProviderInfo[]>("provider_list"),
  providerStatus: (key: string) => invoke<string>("provider_status", { key }),
  testProvider: (key: string, model: string | null, draft: string | null = null) =>
    invoke<string>("test_provider", { key, model, draft }),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<{ warnings: string[] }>("save_settings", { settings }),
  finishFirstRun: () => invoke<void>("finish_first_run"),
  hasApiKey: (key: string) => invoke<boolean>("has_api_key", { key }),
  setApiKey: (key: string, value: string) => invoke<void>("set_api_key", { key, value }),
  deleteApiKey: (key: string) => invoke<void>("delete_api_key", { key }),
  signIn: (key: string) => invoke<void>("sign_in", { key }),
  optionsInit: () => invoke<OptionsInit>("options_init"),
  optionsGo: (a: { tone: string; length: string; context: string; provider: string; model: string | null }) =>
    invoke<void>("options_go", a),
  optionsCancel: () => invoke<void>("options_cancel"),
  pillClick: (region: "main" | "menu") => invoke<void>("pill_click", { region }),
  pillRightClick: () => invoke<void>("pill_right_click"),
  pillMoved: (x: number, y: number) => invoke<void>("pill_moved", { x, y }),
  accessibilityStatus: () => invoke<boolean>("accessibility_status"),
  openAccessibilitySettings: () => invoke<void>("open_accessibility_settings"),
  platform: () => invoke<"windows" | "macos">("platform_name"),
  defaultStyle: () => invoke<string>("default_style"),
};

/** Human label + badge class for a provider status string from Rust. */
export function statusView(s: string | undefined): { text: string; cls: string } {
  if (!s) return { text: "Checking…", cls: "idle" };
  if (s === "ready") return { text: "✅ Ready", cls: "ready" };
  if (s === "signed_out") return { text: "⚠️ Signed out", cls: "warn" };
  if (s === "not_installed") return { text: "❌ Not installed", cls: "bad" };
  if (s === "no_key") return { text: "🔑 No key", cls: "warn" };
  if (s === "unchecked") return { text: "Not checked", cls: "idle" };
  return { text: "⚠️ " + s.replace(/^error:/, "").slice(0, 60), cls: "bad" };
}
