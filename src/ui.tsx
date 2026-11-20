// Componenti di base: pochi, senza libreria. Raggruppati come nelle impostazioni di sistema:
// gruppi con una sola cornice, righe "etichetta a sinistra, controllo a destra".
import { useEffect, useRef, useState, type ReactNode } from "react";
import { events } from "./api";
import { formatHotkey, hotkeyFromEvent } from "./format";
import { useT } from "./i18n";

export function PageTitle({
  title,
  description,
}: {
  title: string;
  description?: string;
}) {
  return (
    <header className="mb-6">
      <h1 className="text-[22px] font-semibold tracking-tight">{title}</h1>
      {description && (
        <p className="mt-1 max-w-[60ch] text-muted">{description}</p>
      )}
    </header>
  );
}

export function Group({
  title,
  children,
}: {
  title?: string;
  children: ReactNode;
}) {
  return (
    <section className="mb-6">
      {title && (
        <h2 className="mb-2 px-1 text-[13px] font-semibold text-muted">
          {title}
        </h2>
      )}
      <div className="divide-y divide-line rounded-[10px] border border-line bg-surface">
        {children}
      </div>
    </section>
  );
}

export function Row({
  label,
  hint,
  children,
}: {
  label: ReactNode;
  hint?: ReactNode;
  children?: ReactNode;
}) {
  return (
    <div className="flex min-h-12 items-center justify-between gap-6 px-4 py-2.5">
      <div className="min-w-0">
        <div>{label}</div>
        {hint && <div className="mt-0.5 text-[12px] text-muted">{hint}</div>}
      </div>
      {children && (
        <div className="flex shrink-0 items-center gap-2">{children}</div>
      )}
    </div>
  );
}

type ButtonVariant = "primary" | "secondary" | "danger" | "ghost";

const BUTTON: Record<ButtonVariant, string> = {
  primary: "bg-cobalt text-white hover:brightness-110 disabled:opacity-50",
  secondary: "border border-line bg-surface hover:bg-paper disabled:opacity-50",
  danger:
    "border border-line bg-surface text-rec hover:bg-paper disabled:opacity-50",
  ghost: "text-muted hover:text-ink disabled:opacity-50",
};

export function Button({
  variant = "secondary",
  className = "",
  ...props
}: React.ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: ButtonVariant;
}) {
  return (
    <button
      type="button"
      className={`h-7 rounded-md px-3 text-[13px] font-medium whitespace-nowrap ${BUTTON[variant]} ${className}`}
      {...props}
    />
  );
}

export function Toggle({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      onClick={() => onChange(!checked)}
      className={`relative h-[22px] w-[38px] rounded-full transition-colors ${
        checked ? "bg-cobalt" : "bg-line"
      }`}
    >
      <span
        className={`absolute top-[2px] left-[2px] h-[18px] w-[18px] rounded-full bg-white shadow transition-transform ${
          checked ? "translate-x-4" : ""
        }`}
      />
    </button>
  );
}

/** Campo di testo e select condividono lo stesso aspetto. */
const FIELD =
  "h-7 rounded-md border border-line bg-surface px-2 text-[13px] text-ink hover:border-muted/60 focus:border-cobalt focus:outline-none";

export function Select({
  value,
  onChange,
  options,
  label,
  className = "",
}: {
  value: string;
  onChange: (v: string) => void;
  options: [string, string][];
  label: string;
  className?: string;
}) {
  // Stesso aspetto dei campi di testo: niente stile nativo, freccia disegnata a parte.
  return (
    <span className={`relative inline-flex max-w-64 ${className}`}>
      <select
        aria-label={label}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className={`${FIELD} w-full appearance-none truncate pr-7`}
      >
        {options.map(([v, l]) => (
          <option key={v} value={v}>
            {l}
          </option>
        ))}
      </select>
      <svg
        aria-hidden
        viewBox="0 0 10 6"
        className="pointer-events-none absolute top-1/2 right-2.5 h-[6px] w-[10px] -translate-y-1/2 text-muted"
      >
        <path
          d="M1 1l4 4 4-4"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
    </span>
  );
}

export function TextInput(props: React.InputHTMLAttributes<HTMLInputElement>) {
  const { className = "", ...rest } = props;
  return (
    <input
      spellCheck={false}
      className={`${FIELD} placeholder:text-muted ${className}`}
      {...rest}
    />
  );
}

/** Messaggio sotto un'azione: errore in rosso, conferma in verde. */
export function Notice({
  text,
  tone,
}: {
  text: string | null;
  tone: "error" | "ok" | "info";
}) {
  if (!text) return null;
  const color =
    tone === "error" ? "text-rec" : tone === "ok" ? "text-ok" : "text-muted";
  return (
    <p
      role={tone === "error" ? "alert" : "status"}
      className={`text-[12px] ${color}`}
    >
      {text}
    </p>
  );
}

/**
 * Misuratore di livello: barre sottili che si accendono da sinistra.
 * È l'elemento distintivo dell'app (lo stesso dell'overlay).
 */
export function LevelMeter({
  bars = 28,
  active = true,
}: {
  bars?: number;
  active?: boolean;
}) {
  const [level, setLevel] = useState(0);
  useEffect(() => {
    const off = events.level((rms) =>
      setLevel(Math.min(1, Math.sqrt(rms) * 2.2)),
    );
    return () => {
      off.then((f) => f());
    };
  }, []);
  const lit = active ? Math.round(level * bars) : 0;
  return (
    <div className="flex h-6 items-center gap-[3px]" aria-hidden>
      {Array.from({ length: bars }, (_, i) => (
        <span
          key={i}
          className={`w-[3px] rounded-full transition-[height,background-color] duration-75 ${
            i < lit ? "bg-cobalt" : "bg-line"
          }`}
          style={{ height: i < lit ? `${10 + ((i * 7) % 13)}px` : "6px" }}
        />
      ))}
    </div>
  );
}

/** Campo che registra una combinazione di tasti nella sintassi del plugin global-shortcut. */
export function HotkeyInput({
  value,
  onChange,
}: {
  value: string;
  onChange: (v: string) => void;
}) {
  const t = useT();
  const [recording, setRecording] = useState(false);
  const ref = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!recording) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      if (e.key === "Escape") {
        setRecording(false);
        return;
      }
      const hk = hotkeyFromEvent(e);
      if (hk) {
        setRecording(false);
        onChange(hk);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [recording, onChange]);

  return (
    <button
      ref={ref}
      type="button"
      onClick={() => setRecording(true)}
      onBlur={() => setRecording(false)}
      className={`h-7 min-w-32 rounded-md border px-3 text-[13px] font-medium ${
        recording
          ? "border-cobalt bg-cobalt-soft text-cobalt"
          : "border-line bg-surface"
      }`}
    >
      {recording ? t("common.pressKeys") : formatHotkey(value)}
    </button>
  );
}
