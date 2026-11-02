import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import { fill, items, t } from "../locales";
import { Band, Line, wentWrong } from "./Bits";

type Saved = { path: string; items: number; missing: number; bytes: number };
type Brought = { added: number; already: number; refused: number; fromElsewhere: boolean };

const EXTENSION = "klipubak";
const ALSO_READS = "cpbackup";

function named() {
  const day = new Date().toISOString().slice(0, 10);
  return `Klipu-${day}.${EXTENSION}`;
}

export default function Backup() {
  const [busy, setBusy] = useState<"out" | "in" | null>(null);
  const [said, setSaid] = useState<string | null>(null);
  const [came, setCame] = useState<string | null>(null);

  const out = async () => {
    setSaid(null);
    setBusy("out");
    const where = await save({
      defaultPath: named(),
      filters: [{ name: "Klipu", extensions: [EXTENSION] }],
    }).catch(() => null);
    if (!where) {
      setBusy(null);
      return;
    }
    try {
      const kept = await invoke<Saved>("save_backup", { path: where, at: Date.now() });
      const lost = kept.missing > 0 ? ` · ${fill("outMissing", items(kept.missing))}` : "";
      setSaid(`${fill("outDone", items(kept.items))}${lost}`);
    } catch (why) {
      setSaid(wentWrong("failedOut", why));
    } finally {
      setBusy(null);
    }
  };

  const bring = async () => {
    setCame(null);
    setBusy("in");
    const chosen = await open({
      multiple: false,
      filters: [{ name: "Klipu", extensions: [EXTENSION, ALSO_READS] }],
    }).catch(() => null);
    if (typeof chosen !== "string") {
      setBusy(null);
      return;
    }
    try {
      const brought = await invoke<Brought>("load_backup", { path: chosen, at: Date.now() });
      const notes = [
        brought.added > 0 && brought.already > 0 ? fill("inAlready", items(brought.already)) : null,
        brought.refused > 0 ? fill("inRefused", items(brought.refused)) : null,
        brought.fromElsewhere ? t("inElsewhere") : null,
      ].filter(Boolean);
      const tail = notes.length > 0 ? ` · ${notes.join(" · ")}` : "";
      setCame(
        brought.added === 0 && notes.length === 0
          ? t("inNothing")
          : `${brought.added === 0 ? t("inNothing") : fill("inDone", items(brought.added))}${tail}`,
      );
    } catch (why) {
      setCame(wentWrong("failedIn", why));
    } finally {
      setBusy(null);
    }
  };

  return (
    <>
      <h1>{t("railBackup")}</h1>

      <Band says={t("bandCopies")} />

      <Line
        says={t("out")}
        why={said ?? t("outWhy")}
        more={<div className="said">{t("outPlain")}</div>}
      >
        <button type="button" className="strong" disabled={busy !== null} onClick={out}>
          {busy === "out" ? t("busy") : t("outDo")}
        </button>
      </Line>

      <Line says={t("in")} why={came ?? t("inWhy")}>
        <button type="button" className="mild" disabled={busy !== null} onClick={bring}>
          {busy === "in" ? t("busy") : t("inDo")}
        </button>
      </Line>
    </>
  );
}
