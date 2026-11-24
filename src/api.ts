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
  app_modes: AppMode[];
  ui_language: string;
  command_hotkey: string;
  translate_to: string;
  vocab_dismissed: string[];
  meeting_auto_detect: boolean;
  meeting_calendar: boolean;
  app_styles_auto: boolean;
}

export interface Segment {
  start_ms: number;
  end_ms: number;
  text: string;
}

export interface FileDone {
  segments: Segment[];
  language: string | null;
  duration_ms: number;
  /** "cancelled" se annullata dall'utente. */
  error: string | null;
}

export interface MeetingState {
  recording: boolean;
  started_at: number | null;
  /** Perché l'audio delle app non si registra: si registra solo il microfono. */
  system_error: string | null;
  /** Il microfono manda solo silenzio assoluto: manca il permesso Microfono. */
  mic_silent: boolean;
}

export interface MeetingLine {
  speaker: "me" | "others";
  start_ms: number;
  end_ms: number;
  text: string;
}

export interface Meeting {
  id: number;
  started_at: number;
  duration_ms: number;
  title: string;
  dir: string;
  status: "transcribing" | "done" | "error";
  error: string | null;
  lines: MeetingLine[];
  summary: string | null;
  attendees: string[];
  actions: MeetingAction[];
}

/** Una cosa da fare uscita da una riunione. */
export interface MeetingAction {
  who: string;
  what: string;
  /** La scadenza come è stata detta ("entro giovedì"). */
  due: string;
  /** La stessa scadenza come data, AAAA-MM-GG. */
  due_date: string | null;
  done: boolean;
}

export interface Source {
  kind: "meeting" | "dictation";
  id: number;
  title: string;
  at: number;
  offset_ms: number | null;
  text: string;
}

export interface Answer {
  text: string;
  sources: Source[];
}

export interface AskTarget {
  label: string;
  /** Il provider gira su questo computer: i brani non escono. */
  local: boolean;
  available: boolean;
}

export interface ImportResult {
  terms: number;
  snippets: number;
  modes: number;
  app_rules: number;
}

export interface Snippet {
  id: number;
  trigger: string;
  text: string;
}

export interface Stats {
  dictations: number;
  words: number;
  /** Tempo passato a parlare, in ms. */
  audio_ms: number;
  /** Parole al giorno, dal più vecchio a oggi. */
  days: number[];
  /** [app, parole], le cinque più usate. */
  apps: [string, number][];
}

export interface AppMode {
  app: string;
  mode: string;
}

export type ProviderKind =
  "openai_compat" | "anthropic" | "gemini" | "cli" | "local" | "none";

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
  slow: boolean;
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
  mode_slug: string | null;
  language: string | null;
  raw_text: string;
  final_text: string;
  status: "ok" | "llm_fallback" | "cancelled" | "paste_failed" | "error";
  error: string | null;
  stt_provider: string;
  stt_model: string | null;
  llm_provider: string | null;
  llm_model: string | null;
  audio_ms: number;
  stt_ms: number | null;
  llm_ms: number | null;
  paste_ms: number | null;
  total_ms: number | null;
  target_app: string | null;
  audio_device: string | null;
  /** Il comando vocale, per i comandi sulla selezione (raw_text è allora la selezione). */
  command: string | null;
}

export interface HistoryFilter {
  query: string;
  since: number | null;
  mode: string | null;
  app: string | null;
}

export interface HistoryFacets {
  total: number;
  today: number;
  week: number;
  /** [slug, nome, quanti] */
  modes: [string, string, number][];
  /** [app, quanti] */
  apps: [string, number][];
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
  listLlmModels: () => invoke<ModelView[]>("list_llm_models"),
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
  createModeWithTask: (name: string, task: string) =>
    invoke<string>("create_mode_with_task", { name, task }),
  deleteMode: (slug: string) => invoke<void>("delete_mode", { slug }),
  resetMode: (slug: string) => invoke<void>("reset_mode", { slug }),
  getVocabulary: () => invoke<string[]>("get_vocabulary"),
  vocabularySuggestions: () =>
    invoke<[string, number][]>("vocabulary_suggestions"),
  dismissSuggestion: (term: string) =>
    invoke<void>("dismiss_suggestion", { term }),
  listSnippets: () => invoke<Snippet[]>("list_snippets"),
  saveSnippet: (trigger: string, text: string) =>
    invoke<void>("save_snippet", { trigger, text }),
  deleteSnippet: (id: number) => invoke<void>("delete_snippet", { id }),
  getStats: (since: number, days: number) =>
    invoke<Stats>("get_stats", {
      since,
      utcOffsetMin: -new Date().getTimezoneOffset(),
      days,
    }),
  addTerm: (term: string) => invoke<void>("add_term", { term }),
  removeTerm: (term: string) => invoke<void>("remove_term", { term }),
  getHistory: (filter: HistoryFilter, limit: number, offset: number) =>
    invoke<HistoryItem[]>("get_history", { filter, limit, offset }),
  historyFacets: (today: number, week: number) =>
    invoke<HistoryFacets>("history_facets", { today, week }),
  deleteDictation: (id: number) => invoke<void>("delete_dictation", { id }),
  clearHistory: () => invoke<number>("clear_history"),
  copyText: (text: string) => invoke<void>("copy_text", { text }),
  pickMediaFile: () => invoke<string | null>("pick_media_file"),
  transcribeFile: (path: string) => invoke<void>("transcribe_file", { path }),
  cancelFile: () => invoke<void>("cancel_file"),
  summarize: (text: string) => invoke<string>("summarize", { text }),
  saveTextFile: (name: string, text: string) =>
    invoke<string | null>("save_text_file", { name, text }),
  exportSettings: () => invoke<string | null>("export_settings"),
  meetingStart: () => invoke<MeetingState>("meeting_start"),
  meetingStop: () => invoke<number>("meeting_stop"),
  meetingState: () => invoke<MeetingState>("meeting_state"),
  listMeetings: () => invoke<Meeting[]>("list_meetings"),
  getMeeting: (id: number) => invoke<Meeting | null>("get_meeting", { id }),
  summarizeMeeting: (id: number) =>
    invoke<{ summary: string; actions: MeetingAction[] }>("summarize_meeting", {
      id,
    }),
  saveMeetingActions: (id: number, actions: MeetingAction[]) =>
    invoke<void>("save_meeting_actions", { id, actions }),
  addMeetingReminders: (id: number) =>
    invoke<number>("add_meeting_reminders", { id }),
  renameMeeting: (id: number, title: string) =>
    invoke<void>("rename_meeting", { id, title }),
  deleteMeeting: (id: number) => invoke<void>("delete_meeting", { id }),
  ask: (question: string) => invoke<Answer>("ask", { question }),
  askTarget: () => invoke<AskTarget>("ask_target"),
  calendarAccess: (request: boolean) =>
    invoke<"granted" | "denied" | "undetermined">("calendar_access", {
      request,
    }),
  screenPermission: (request: boolean) =>
    invoke<boolean>("screen_permission", { request }),
  importSettings: () => invoke<ImportResult | null>("import_settings"),
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
  state: (
    cb: (s: PipelineState, app: string | null) => void,
  ): Promise<UnlistenFn> =>
    listen<{ state: PipelineState; app: string | null }>(
      "pipeline://state",
      (e) => cb(e.payload.state, e.payload.app),
    ),
  historyChanged: (cb: () => void): Promise<UnlistenFn> =>
    listen<number>("history://changed", () => cb()),
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
  fileProgress: (
    cb: (p: { stage: "decode" | "transcribe"; pct: number }) => void,
  ) =>
    listen<{ stage: "decode" | "transcribe"; pct: number }>(
      "file://progress",
      (e) => cb(e.payload),
    ),
  meetingState: (cb: (s: MeetingState) => void) =>
    listen<MeetingState>("meeting://state", (e) => cb(e.payload)),
  meetingProgress: (cb: (p: { id: number; pct: number }) => void) =>
    listen<{ id: number; pct: number }>("meeting://progress", (e) =>
      cb(e.payload),
    ),
  meetingDone: (cb: (id: number) => void) =>
    listen<number>("meeting://done", (e) => cb(e.payload)),
  fileDone: (cb: (d: FileDone) => void) =>
    listen<FileDone>("file://done", (e) => cb(e.payload)),
  navigate: (cb: (section: string) => void) =>
    listen<string>("navigate", (e) => cb(e.payload)),
};

/** Messaggio leggibile da un errore di `invoke` (il backend restituisce stringhe). */
export function errorText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
}
