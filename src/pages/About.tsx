import { useEffect, useState } from "react";
import { api, errorText, type AppInfo } from "../api";
import { useSettings } from "../App";
import { useT } from "../i18n";
import { Button, Group, Notice, PageTitle, Row, Toggle } from "../ui";

export default function About() {
  const t = useT();
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

  const check = async () => {
    setNotice({ text: t("common.checking"), tone: "info" });
    try {
      const v = await api.checkUpdate();
      setNotice(
        v
          ? { text: t("about.found", { v }), tone: "ok" }
          : { text: t("about.upToDate"), tone: "ok" },
      );
      setInfo(await api.appInfo());
    } catch (e) {
      setNotice({ text: errorText(e), tone: "error" });
    }
  };

  const install = async () => {
    setNotice({ text: t("about.installing"), tone: "info" });
    try {
      await api.installUpdate();
    } catch (e) {
      setNotice({ text: errorText(e), tone: "error" });
    }
  };

  const [before, after] = t("about.reopenGuide").split("{link}");

  return (
    <>
      <PageTitle title={t("about.title")} />
      <Group>
        <Row label={t("about.version")}>
          <span className="text-muted">
            {info.version}
            {info.personal_build ? t("about.personal") : ""}
          </span>
        </Row>
        <Row
          label={t("about.data")}
          hint={<span className="selectable">{info.data_dir}</span>}
        />
        <Row
          label={t("about.log")}
          hint={<span className="selectable">{info.log_dir}</span>}
        />
      </Group>

      <Group title={t("about.updates")}>
        <Row
          label={
            info.pending_update
              ? t("about.updateAvailable", { v: info.pending_update })
              : t("about.autoUpdates")
          }
          hint={
            info.pending_update
              ? t("about.restartHint")
              : t("about.updatesHint")
          }
        >
          {info.pending_update ? (
            <Button variant="primary" onClick={install}>
              {t("about.install")}
            </Button>
          ) : (
            <>
              <Button variant="ghost" onClick={check}>
                {t("about.checkNow")}
              </Button>
              <Toggle
                label={t("about.autoUpdates")}
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

      <Group title={t("about.privacy")}>
        <Row label={t("about.noTelemetry")} hint={t("about.noTelemetryHint")} />
      </Group>
      <p className="mt-6 text-[12px] text-muted">
        {before}
        <a href="#onboarding" className="text-cobalt">
          {t("about.reopenGuideLink")}
        </a>
        {after}
      </p>
    </>
  );
}
