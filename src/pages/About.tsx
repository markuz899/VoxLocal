import { useEffect, useState } from "react";
import { api, type AppInfo } from "../api";
import { Group, PageTitle, Row } from "../ui";

export default function About() {
  const [info, setInfo] = useState<AppInfo | null>(null);
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
