import { useEffect, useState } from "react";
import { api, errorText, type AppInfo } from "../api";
import { useSettings } from "../App";
import { Button, Group, Notice, PageTitle, Row, Toggle } from "../ui";

export default function About() {
  const { settings, update } = useSettings();
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [notice, setNotice] = useState<{
    text: string;
    tone: "ok" | "error" | "info";
  } | null>(null);
  useEffect(() => {
    api.appInfo().then(setInfo);
  }, []);
  if (!info) return null;

  return (
    <>
      <PageTitle title="Informazioni" />
      <Group>
        <Row label="Versione">
          <span className="text-muted">
            {info.version}
            {info.personal_build ? " (build personale)" : ""}
          </span>
        </Row>
        <Row
          label="Dati"
          hint={<span className="selectable">{info.data_dir}</span>}
        />
        <Row
          label="Log"
          hint={<span className="selectable">{info.log_dir}</span>}
        />
      </Group>
      <Group title="Aggiornamenti">
        <Row
          label={
            info.pending_update
              ? `È disponibile la versione ${info.pending_update}`
              : "Cerca aggiornamenti automaticamente"
          }
          hint={
            info.pending_update
              ? "VoxLocal si riavvia dopo l'installazione."
              : "All'avvio e ogni 6 ore. Gli aggiornamenti sono firmati."
          }
        >
          {info.pending_update ? (
            <Button
              variant="primary"
              onClick={async () => {
                setNotice({ text: "Scarico e installo…", tone: "info" });
                try {
                  await api.installUpdate();
                } catch (e) {
                  setNotice({ text: errorText(e), tone: "error" });
                }
              }}
            >
              Installa e riavvia
            </Button>
          ) : (
            <>
              <Button
                variant="ghost"
                onClick={async () => {
                  setNotice({ text: "Controllo…", tone: "info" });
                  try {
                    const v = await api.checkUpdate();
                    setNotice(
                      v
                        ? { text: `Trovata la versione ${v}.`, tone: "ok" }
                        : { text: "Hai già l'ultima versione.", tone: "ok" },
                    );
                    setInfo(await api.appInfo());
                  } catch (e) {
                    setNotice({ text: errorText(e), tone: "error" });
                  }
                }}
              >
                Cerca ora
              </Button>
              <Toggle
                label="Cerca aggiornamenti automaticamente"
                checked={settings.check_updates}
                onChange={(check_updates) => update({ check_updates })}
              />
            </>
          )}
        </Row>
      </Group>
      {notice && (
        <div className="-mt-4 mb-6 px-1">
          <Notice tone={notice.tone} text={notice.text} />
        </div>
      )}
      <Group title="Privacy">
        <Row
          label="Nessuna telemetria"
          hint="VoxLocal non invia statistiche né dati d'uso. Audio e testo lasciano il computer solo se scegli un servizio cloud per la trascrizione o per l'elaborazione."
        />
      </Group>
      <p className="mt-6 text-[12px] text-muted">
        Per rivedere la configurazione iniziale{" "}
        <a href="#onboarding" className="text-cobalt">
          riapri la guida
        </a>
        .
      </p>
    </>
  );
}
