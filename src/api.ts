// Wrapper tipizzati dei comandi Tauri (src-tauri/src/commands.rs). Il webview non fa rete
// e non vede segreti: tutto passa da qui.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type HotkeyMode = "ptt" | "toggle";

export interface Settings {
  version: number;
  hotkey: string;
  hotkey_mode: HotkeyMode;
  language: string;
  local_model: string;
  keep_warm_minutes: number;
  restore_clipboard: boolean;
  restore_delay_ms: number;
  max_seconds: number;
  active_mode: string;
  default_llm: string;
  providers: ProviderConfig[];
  stt_provider: string;
  llm_timeout_s: number;
  cli_timeout_s: number;
  retention_days: number;
  audio_device: string | null;
  onboarded: boolean;
  check_updates: boolean;
}

export type ProviderKind =
  "openai_compat" | "anthropic" | "gemini" | "cli" | "none";

export interface ProviderConfig {
  id: string;
  label: string;
  kind: ProviderKind;
  base_url: string;
  model: string;
  needs_key: boolean;
  send_sampling: boolean;
  command: string[];
  system_flag: string | null;
  model_flag: string | null;
  clear_env: string[];
  stt_model: string | null;
}

export interface ProviderView extends ProviderConfig {
  has_key: boolean;
}

export interface ModelView {
  id: string;
  label: string;
  size: number;
  installed: boolean;
  recommended: boolean;
}

export interface Mode {
  id: number;
  slug: string;
  name: string;
  system_prompt: string;
  user_template: string;
  provider_id: string | null;
  model: string | null;
  temperature: number;
}

export interface HistoryItem {
  id: number;
  created_at: number;
  mode: string | null;
  raw_text: string;
  final_text: string;
  status: "ok" | "llm_fallback" | "cancelled" | "paste_failed" | "error";
  error: string | null;
  llm_provider: string | null;
  audio_ms: number;
  total_ms: number | null;
}

export type Access = "granted" | "denied" | "undetermined" | "unknown";

export interface PermissionStatus {
  microphone: Access;
  accessibility: Access;
  needs_accessibility: boolean;
}

export interface AppInfo {
  version: string;
  pending_update: string | null;
  data_dir: string;
  log_dir: string;
  personal_build: boolean;
  platform: string;
}

export type PipelineState =
  "idle" | "recording" | "transcribing" | "postprocessing" | "pasting";

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) =>
    invoke<Settings>("save_settings", { settings }),
  listInputDevices: () => invoke<string[]>("list_input_devices"),
  micTest: (on: boolean) => invoke<void>("mic_test", { on }),
  listModels: () => invoke<ModelView[]>("list_models"),
  downloadModel: (id: string) => invoke<void>("download_model", { id }),
  deleteModel: (id: string) => invoke<void>("delete_model", { id }),
  listProviders: () => invoke<ProviderView[]>("list_providers"),
  saveProvider: (provider: ProviderConfig) =>
    invoke<void>("save_provider", { provider }),
  setApiKey: (id: string, key: string) =>
    invoke<void>("set_api_key", { id, key }),
  deleteApiKey: (id: string) => invoke<void>("delete_api_key", { id }),
  checkProvider: (id: string) => invoke<number>("check_provider", { id }),
  listProviderModels: (id: string) =>
    invoke<string[]>("list_provider_models", { id }),
  listModes: () => invoke<Mode[]>("list_modes"),
  updateMode: (mode: Omit<Mode, "id">) => invoke<void>("update_mode", { mode }),
  createMode: (name: string) => invoke<string>("create_mode", { name }),
  deleteMode: (slug: string) => invoke<void>("delete_mode", { slug }),
  resetMode: (slug: string) => invoke<void>("reset_mode", { slug }),
  getVocabulary: () => invoke<string[]>("get_vocabulary"),
  addTerm: (term: string) => invoke<void>("add_term", { term }),
  removeTerm: (term: string) => invoke<void>("remove_term", { term }),
  getHistory: (query: string, limit: number, offset: number) =>
    invoke<HistoryItem[]>("get_history", { query, limit, offset }),
  deleteDictation: (id: number) => invoke<void>("delete_dictation", { id }),
  clearHistory: () => invoke<number>("clear_history"),
  copyText: (text: string) => invoke<void>("copy_text", { text }),
  permissionsStatus: () => invoke<PermissionStatus>("permissions_status"),
  requestMicrophone: () => invoke<Access>("request_microphone"),
  requestAccessibility: () => invoke<void>("request_accessibility"),
  openPermissionSettings: (which: "microphone" | "accessibility") =>
    invoke<void>("open_permission_settings", { which }),
  getAutostart: () => invoke<boolean>("get_autostart"),
  setAutostart: (enabled: boolean) =>
    invoke<void>("set_autostart", { enabled }),
  finishOnboarding: () => invoke<void>("finish_onboarding"),
  appInfo: () => invoke<AppInfo>("app_info"),
  checkUpdate: () => invoke<string | null>("check_update"),
  installUpdate: () => invoke<void>("install_update"),
};

export const events = {
  level: (cb: (rms: number) => void): Promise<UnlistenFn> =>
    listen<number>("audio://level", (e) => cb(e.payload)),
  state: (cb: (s: PipelineState) => void): Promise<UnlistenFn> =>
    listen<{ state: PipelineState }>("pipeline://state", (e) =>
      cb(e.payload.state),
    ),
  modelProgress: (
    cb: (p: { id: string; done: number; total: number }) => void,
  ) =>
    listen<{ id: string; done: number; total: number }>(
      "models://progress",
      (e) => cb(e.payload),
    ),
  modelDone: (cb: (p: { id: string; error: string | null }) => void) =>
    listen<{ id: string; error: string | null }>("models://done", (e) =>
      cb(e.payload),
    ),
  settingsChanged: (cb: (s: Settings) => void) =>
    listen<Settings>("settings://changed", (e) => cb(e.payload)),
  navigate: (cb: (section: string) => void) =>
    listen<string>("navigate", (e) => cb(e.payload)),
};

/** Messaggio leggibile da un errore di `invoke` (il backend restituisce stringhe). */
export function errorText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
}
