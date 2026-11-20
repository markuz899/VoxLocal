import { useEffect, useState } from "react";
import { api, errorText } from "../api";
import { useSettings } from "../App";
import { languageOptions } from "../format";
import { UI_LANGUAGES, useT } from "../i18n";
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

export default function General() {
  const t = useT();
  const { settings, update } = useSettings();
  const lang = settings.ui_language;
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

  const retention: [string, string][] = [
    ["7", t("general.days", { n: 7 })],
    ["30", t("general.days", { n: 30 })],
    ["90", t("general.days", { n: 90 })],
    ["365", t("general.year")],
    ["0", t("general.forever")],
  ];

  return (
    <>
      <PageTitle title={t("general.title")} />
      <Notice tone="error" text={error} />

      <Group>
        <Row label={t("general.uiLanguage")}>
          <Select
            label={t("general.uiLanguage")}
            value={lang}
            onChange={(ui_language) => save({ ui_language })}
            options={UI_LANGUAGES}
          />
        </Row>
      </Group>

      <Group title={t("general.dictation")}>
        <Row label={t("general.shortcut")} hint={t("general.shortcutHint")}>
          <HotkeyInput
            value={settings.hotkey}
            onChange={(hotkey) => save({ hotkey })}
          />
        </Row>
        <Row label={t("general.activation")}>
          <Select
            label={t("general.activation")}
            value={settings.hotkey_mode}
            onChange={(v) => save({ hotkey_mode: v as "ptt" | "toggle" })}
            options={[
              ["ptt", t("general.ptt")],
              ["toggle", t("general.toggle")],
            ]}
          />
        </Row>
        <Row
          label={t("general.commandShortcut")}
          hint={t("general.commandShortcutHint")}
        >
          <HotkeyInput
            value={settings.command_hotkey}
            onChange={(command_hotkey) => save({ command_hotkey })}
          />
        </Row>
        <Row label={t("general.language")} hint={t("general.languageHint")}>
          <Select
            label={t("general.language")}
            value={settings.language}
            onChange={(language) => save({ language })}
            options={languageOptions(lang, true)}
          />
        </Row>
        <Row
          label={t("general.translateTo")}
          hint={t("general.translateToHint")}
        >
          <Select
            label={t("general.translateTo")}
            value={settings.translate_to}
            onChange={(translate_to) => save({ translate_to })}
            options={languageOptions(lang)}
          />
        </Row>
      </Group>

      <Group title={t("general.microphone")}>
        <Row label={t("general.device")}>
          <Select
            label={t("general.microphone")}
            value={settings.audio_device ?? ""}
            onChange={(v) => save({ audio_device: v || null })}
            options={[
              ["", t("common.systemMic")],
              ...devices.map((d): [string, string] => [d, d]),
            ]}
          />
        </Row>
        <Row label={testing ? <LevelMeter /> : t("general.testMic")}>
          <Button onClick={toggleTest}>
            {testing ? t("common.stop") : t("common.test")}
          </Button>
        </Row>
      </Group>

      <Group title={t("general.system")}>
        <Row label={t("general.autostart")}>
          <Toggle
            label={t("general.autostart")}
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
          label={t("general.restoreClipboard")}
          hint={t("general.restoreClipboardHint")}
        >
          <Toggle
            label={t("general.restoreClipboard")}
            checked={settings.restore_clipboard}
            onChange={(restore_clipboard) => save({ restore_clipboard })}
          />
        </Row>
        <Row label={t("general.retention")}>
          <Select
            label={t("general.retention")}
            value={String(settings.retention_days)}
            onChange={(v) => save({ retention_days: Number(v) })}
            options={retention}
          />
        </Row>
      </Group>
    </>
  );
}
