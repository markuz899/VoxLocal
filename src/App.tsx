import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useState,
} from "react";
import { api, errorText, events, type Settings } from "./api";
import Onboarding from "./pages/Onboarding";
import General from "./pages/General";
import Transcription from "./pages/Transcription";
import Modes from "./pages/Modes";
import Providers from "./pages/Providers";
import Vocabulary from "./pages/Vocabulary";
import History from "./pages/History";
import About from "./pages/About";
import Faq from "./pages/Faq";
import Stats from "./pages/Stats";
import Snippets from "./pages/Snippets";
import Files from "./pages/Files";
import Meetings from "./pages/Meetings";
import Ask from "./pages/Ask";
import { translate, useT } from "./i18n";

interface SettingsCtx {
  settings: Settings;
  /** Applica una modifica e la salva subito; restituisce l'errore del backend, se c'è. */
  update: (patch: Partial<Settings>) => Promise<string | null>;
}

const Ctx = createContext<SettingsCtx | null>(null);

export function useSettings(): SettingsCtx {
  const ctx = useContext(Ctx);
  if (!ctx) throw new Error("useSettings fuori da <App>");
  return ctx;
}

const SECTIONS = [
  { id: "general", label: "nav.general", page: General },
  { id: "transcription", label: "nav.transcription", page: Transcription },
  { id: "modes", label: "nav.modes", page: Modes },
  { id: "providers", label: "nav.providers", page: Providers },
  { id: "vocabulary", label: "nav.vocabulary", page: Vocabulary },
  { id: "snippets", label: "nav.snippets", page: Snippets },
  { id: "meetings", label: "nav.meetings", page: Meetings },
  { id: "ask", label: "nav.ask", page: Ask },
  { id: "files", label: "nav.files", page: Files },
  { id: "history", label: "nav.history", page: History },
  { id: "stats", label: "nav.stats", page: Stats },
  { id: "faq", label: "nav.faq", page: Faq },
  { id: "about", label: "nav.about", page: About },
] as const;

function currentSection(): string {
  return window.location.hash.replace(/^#/, "") || "general";
}

export default function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [section, setSection] = useState(currentSection);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    api.getSettings().then(setSettings, (e) => setLoadError(errorText(e)));
    const offs = [
      events.settingsChanged(setSettings),
      events.navigate((s) => (window.location.hash = s)),
    ];
    const onHash = () => setSection(currentSection());
    window.addEventListener("hashchange", onHash);
    return () => {
      window.removeEventListener("hashchange", onHash);
      offs.forEach((p) => p.then((off) => off()));
    };
  }, []);

  const update = useCallback(
    async (patch: Partial<Settings>) => {
      if (!settings) return "settings not loaded yet";
      try {
        setSettings(await api.saveSettings({ ...settings, ...patch }));
        return null;
      } catch (e) {
        return errorText(e);
      }
    },
    [settings],
  );

  if (loadError) {
    return (
      <p className="p-8 text-rec">
        {translate("en", "app.loadError", { error: loadError })}
      </p>
    );
  }
  if (!settings) return null;

  const showOnboarding = !settings.onboarded || section === "onboarding";

  return (
    <Ctx.Provider value={{ settings, update }}>
      {showOnboarding ? (
        <Onboarding
          onDone={() => {
            window.location.hash = "general";
          }}
        />
      ) : (
        <Shell section={section} />
      )}
    </Ctx.Provider>
  );
}

function Shell({ section }: { section: string }) {
  const t = useT();
  const active = SECTIONS.find((s) => s.id === section) ?? SECTIONS[0];
  const Page = active.page;
  return (
    <div className="flex h-full">
      <nav
        aria-label={t("nav.sections")}
        className="flex w-52 shrink-0 flex-col gap-0.5 border-r border-line px-3 pt-6"
      >
        <div className="mb-5 px-2 text-[15px] font-semibold">VoxLocal</div>
        {SECTIONS.map((s) => (
          <a
            key={s.id}
            href={`#${s.id}`}
            aria-current={s.id === active.id ? "page" : undefined}
            className={`rounded-md px-2 py-1.5 ${
              s.id === active.id
                ? "bg-cobalt-soft font-medium text-cobalt"
                : "hover:bg-surface"
            }`}
          >
            {t(s.label)}
          </a>
        ))}
      </nav>
      <main className="min-w-0 flex-1 overflow-y-auto px-10 pt-8 pb-12">
        <div className={active.id === "history" ? "" : "max-w-[640px]"}>
          <Page />
        </div>
      </main>
    </div>
  );
}
