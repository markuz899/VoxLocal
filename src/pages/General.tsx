import { useEffect, useState } from "react";
import { api, errorText } from "../api";
import { useSettings } from "../App";
import { LANGUAGES } from "../format";
import {
  Button,
  Group,
  HotkeyInput,
  LevelMeter,
  Notice,
  PageTitle,
  Row,
  Select,
  Toggle,
} from "../ui";

const RETENTION: [string, string][] = [
  ["7", "7 giorni"],
  ["30", "30 giorni"],
  ["90", "90 giorni"],
  ["365", "1 anno"],
  ["0", "Per sempre"],
];

export default function General() {
  const { settings, update } = useSettings();
  const [devices, setDevices] = useState<string[]>([]);
  const [autostart, setAutostart] = useState(false);
  const [testing, setTesting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.listInputDevices().then(setDevices);
    api.getAutostart().then(setAutostart);
    return () => {
      api.micTest(false);
    };
  }, []);

  const toggleTest = async () => {
    setError(null);
    try {
      await api.micTest(!testing);
      setTesting(!testing);
    } catch (e) {
      setError(errorText(e));
    }
  };

  const save = async (patch: Parameters<typeof update>[0]) =>
    setError(await update(patch));

  return (
    <>
      <PageTitle title="Generale" />
      <Notice tone="error" text={error} />

      <Group title="Dettatura">
        <Row label="Scorciatoia" hint="Funziona in qualsiasi app. Esc annulla.">
          <HotkeyInput
            value={settings.hotkey}
            onChange={(hotkey) => save({ hotkey })}
          />
        </Row>
        <Row label="Come si attiva">
          <Select
            label="Come si attiva"
            value={settings.hotkey_mode}
            onChange={(v) => save({ hotkey_mode: v as "ptt" | "toggle" })}
            options={[
              ["ptt", "Tieni premuto per parlare"],
              ["toggle", "Premi per avviare e fermare"],
            ]}
          />
        </Row>
        <Row
          label="Lingua"
          hint="Fissarla rende la trascrizione più precisa e veloce."
        >
          <Select
            label="Lingua"
            value={settings.language}
            onChange={(language) => save({ language })}
            options={LANGUAGES}
          />
        </Row>
      </Group>

      <Group title="Microfono">
        <Row label="Dispositivo">
          <Select
            label="Microfono"
            value={settings.audio_device ?? ""}
            onChange={(v) => save({ audio_device: v || null })}
            options={[
              ["", "Microfono di sistema"],
              ...devices.map((d): [string, string] => [d, d]),
            ]}
          />
        </Row>
        <Row label={testing ? <LevelMeter /> : "Prova il microfono"}>
          <Button onClick={toggleTest}>{testing ? "Ferma" : "Prova"}</Button>
        </Row>
      </Group>

      <Group title="Sistema">
        <Row label="Apri all'accesso">
          <Toggle
            label="Apri all'accesso"
            checked={autostart}
            onChange={async (v) => {
              try {
                await api.setAutostart(v);
              } catch (e) {
                setError(errorText(e));
              }
              setAutostart(await api.getAutostart());
            }}
          />
        </Row>
        <Row
          label="Ripristina gli appunti dopo l'incolla"
          hint="Rimette negli appunti quello che c'era prima del dettato."
        >
          <Toggle
            label="Ripristina gli appunti"
            checked={settings.restore_clipboard}
            onChange={(restore_clipboard) => save({ restore_clipboard })}
          />
        </Row>
        <Row label="Conserva la cronologia per">
          <Select
            label="Conserva la cronologia per"
            value={String(settings.retention_days)}
            onChange={(v) => save({ retention_days: Number(v) })}
            options={RETENTION}
          />
        </Row>
      </Group>
    </>
  );
}
