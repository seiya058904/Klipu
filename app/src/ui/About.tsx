import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useEffect, useRef, useState } from "react";
import { licences, notices } from "../core";
import { fill, t } from "../locales";
import { composed } from "../markdown";
import mark from "../mark.svg";
import { CloudOff, Code, Gift, Info, Key } from "./Icons";

const REPOSITORY = "https://github.com/seiya058904/Klipu";
const UPSTREAM = "https://github.com/rgdevment/CopyPaste";
const PRIVACY = `${REPOSITORY}/blob/main/PRIVACY.md`;

export const LINKS = [REPOSITORY, UPSTREAM, PRIVACY];

function Notices({
  text,
  label,
  onLink,
}: {
  text: string;
  label: string;
  onLink: (url: string) => void;
}) {
  const box = useRef<HTMLElement>(null);
  const follow = useRef(onLink);
  follow.current = onLink;

  useEffect(() => {
    const holder = box.current;
    if (!holder) return;
    holder.innerHTML = composed(text);
    const clicked = (event: Event) => {
      const link = (event.target as HTMLElement).closest("a");
      if (!link) return;
      event.preventDefault();
      follow.current(link.href);
    };
    holder.addEventListener("click", clicked);
    return () => holder.removeEventListener("click", clicked);
  }, [text]);

  return <section ref={box} className="notices" aria-label={label} />;
}

export default function About() {
  const [trouble, setTrouble] = useState<string | null>(null);
  const [version, setVersion] = useState<string | null>(null);
  const [said, setSaid] = useState<string | null>(null);
  const [texts, setTexts] = useState<string | null>(null);
  const [report, setReport] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    getVersion()
      .then(setVersion)
      .catch(() => setVersion(null));
  }, []);

  const go = (where: string) => {
    setTrouble(null);
    invoke("open_web", { url: where }).catch(() => setTrouble(fill("linkRefused", where)));
  };

  const saveReport = () => {
    setTrouble(null);
    setSaving(true);
    invoke<string | null>("save_report")
      .then((path) => {
        if (path) setReport(path);
      })
      .catch(() => setTrouble(t("reportRefused")))
      .finally(() => setSaving(false));
  };

  const openLog = () => {
    setTrouble(null);
    invoke("open_log").catch(() => setTrouble(t("logRefused")));
  };

  return (
    <>
      {trouble && <p className="alarm">{trouble}</p>}

      <div className="brow">
        <img className="mark" src={mark} alt="" />
        <span>
          <h1>Klipu</h1>
          <span className="line2">
            <span>{version ?? "—"}</span>
            <i />
            <span>GPL-3.0</span>
          </span>
        </span>
      </div>

      <div className="what-is">
        <p className="eyebrow">
          <Info />
          {t("aboutIs")}
        </p>
        <p>{t("aboutWhat")}</p>
        <p>{t("aboutPrivacy")}</p>
        <p className="quiet">{t("aboutOrigin")}</p>
        <button
          type="button"
          className="welcome-link"
          onClick={() => {
            invoke("tour").catch(() => {});
          }}
        >
          {t("welcomeShowAgain")}
        </button>
        <div className="badges">
          <span className="badge">
            <Key />
            {t("badgeLocal")}
          </span>
          <span className="badge">
            <Code />
            {t("badgeOpen")}
          </span>
          <span className="badge">
            <Gift />
            {t("badgeFree")}
          </span>
          <span className="badge">
            <CloudOff />
            {t("badgeQuiet")}
          </span>
        </div>
      </div>

      <div className="trouble">
        <div className="rule" style={{ marginTop: 0 }}>
          {t("troubleTitle")}
        </div>
        <p className="quiet">
          {t("troubleWhat")} <em>{t("troubleNeverSent")}</em>
          {t("troubleYours")}
        </p>
        <div className="feet">
          <button type="button" className="mild" disabled={saving} onClick={saveReport}>
            {saving ? t("busy") : t("troubleReport")}
          </button>
          <button type="button" className="mild" onClick={openLog}>
            {t("troubleLog")}
          </button>
        </div>
        {report && (
          <p className="quiet">
            {t("reportSaved")}{" "}
            <button
              type="button"
              className="welcome-link"
              onClick={() => {
                revealItemInDir(report).catch(() => setTrouble(t("reportShowRefused")));
              }}
            >
              {t("reportShow")}
            </button>
          </p>
        )}
      </div>

      <div className="links">
        <button type="button" onClick={() => go(REPOSITORY)}>
          {t("aboutRepo")}
        </button>
        <button type="button" onClick={() => go(UPSTREAM)}>
          {t("aboutUpstream")}
        </button>
        <button type="button" onClick={() => go(PRIVACY)}>
          {t("aboutPrivacyLink")}
        </button>
        <button
          type="button"
          aria-expanded={said !== null}
          onClick={() => {
            if (said !== null) return setSaid(null);
            setTrouble(null);
            notices()
              .then(setSaid)
              .catch(() => setTrouble(t("noticesRefused")));
          }}
        >
          {t("aboutNotices")}
        </button>
        <button
          type="button"
          aria-expanded={texts !== null}
          onClick={() => {
            if (texts !== null) return setTexts(null);
            setTrouble(null);
            licences()
              .then(setTexts)
              .catch(() => setTrouble(t("licencesRefused")));
          }}
        >
          {t("aboutLicences")}
        </button>
      </div>
      {said !== null && <Notices text={said} label={t("aboutNotices")} onLink={go} />}
      {texts !== null && <Notices text={texts} label={t("aboutLicences")} onLink={go} />}
    </>
  );
}
