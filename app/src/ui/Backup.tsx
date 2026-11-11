import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useEffect, useRef, useState } from "react";
import { fill, items, sized, t } from "../locales";
import { Band, Line, wentWrong } from "./Bits";

type Saved = { path: string; items: number; missing: number; bytes: number };
type Brought = {
  added: number;
  already: number;
  refused: number;
  partial: number;
  repaired: number;
  fromElsewhere: boolean;
};
type Review = Brought & {
  token: string;
  name: string;
  expires: number;
  bytesAfter: number;
  quota: number | null;
  keepFor: number | null;
};
type Loaded = { kind: "review"; review: Review } | { kind: "done"; result: Brought };

const EXTENSION = "klipubak";
const ALSO_READS = "cpbackup";

function named() {
  const day = new Date().toISOString().slice(0, 10);
  return `Klipu-${day}.${EXTENSION}`;
}

export default function Backup() {
  const [busy, setBusy] = useState<"out" | "in" | "repair" | null>(null);
  const [said, setSaid] = useState<string | null>(null);
  const [came, setCame] = useState<string | null>(null);
  const [review, setReview] = useState<Review | null>(null);
  const [repaired, setRepaired] = useState<string | null>(null);
  const held = useRef<string | null>(null);
  const alive = useRef(true);

  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      if (held.current) void invoke("cancel_backup", { token: held.current }).catch(() => {});
    };
  }, []);

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

  const inspect = async () => {
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
      const next = await invoke<Review>("peek_backup", { path: chosen });
      if (!alive.current) {
        void invoke("cancel_backup", { token: next.token }).catch(() => {});
        return;
      }
      held.current = next.token;
      setReview(next);
    } catch (why) {
      console.error("peek_backup", why);
      if (alive.current) setCame(t("inReviewFailed"));
    } finally {
      if (alive.current) setBusy(null);
    }
  };

  const cancel = async (adjust = false) => {
    const token = held.current;
    held.current = null;
    setReview(null);
    setCame(null);
    if (token) await invoke("cancel_backup", { token }).catch(() => {});
    if (adjust) {
      await invoke("open_settings", { rail: "history" }).catch(() => setCame(t("inAdjustFailed")));
    }
  };

  const bring = async () => {
    if (!review) return;
    setBusy("in");
    setCame(null);
    try {
      const landed = await invoke<Loaded>("load_backup", { token: review.token });
      if (!alive.current) return;
      if (landed.kind === "review") {
        setReview(landed.review);
        setCame(t("inChanged"));
        return;
      }
      held.current = null;
      setReview(null);
      const brought = landed.result;
      const notes = [
        brought.already > 0 ? fill("inAlready", items(brought.already)) : null,
        brought.partial > 0 ? fill("inPartial", items(brought.partial)) : null,
        brought.refused > 0 ? fill("inRefused", items(brought.refused)) : null,
        brought.repaired > 0 ? fill("inRepaired", items(brought.repaired)) : null,
        brought.fromElsewhere ? t("inElsewhere") : null,
      ].filter(Boolean);
      setCame(
        [
          brought.added === 0 ? t("inNothing") : fill("inDone", items(brought.added)),
          ...notes,
        ].join(" · "),
      );
    } catch (why) {
      console.error("load_backup", why);
      held.current = null;
      if (alive.current) {
        setReview(null);
        setCame(t("inReviewFailed"));
      }
    } finally {
      if (alive.current) setBusy(null);
    }
  };

  const repair = async () => {
    setBusy("repair");
    setRepaired(null);
    try {
      const count = await invoke<number>("repair_search");
      if (alive.current)
        setRepaired(count > 0 ? fill("inRepaired", items(count)) : t("inRepairUnchanged"));
    } catch (why) {
      console.error("repair_search", why);
      if (alive.current) setRepaired(t("inRepairFailed"));
    } finally {
      if (alive.current) setBusy(null);
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
        <button
          type="button"
          className="strong"
          disabled={busy !== null || review !== null}
          onClick={out}
        >
          {busy === "out" ? t("busy") : t("outDo")}
        </button>
      </Line>

      <Line says={t("in")} why={<span role="status">{came ?? t("inWhy")}</span>}>
        {!review && (
          <button type="button" className="mild" disabled={busy !== null} onClick={inspect}>
            {busy === "in" ? t("inChecking") : t("inDo")}
          </button>
        )}
      </Line>
      {review && (
        <ImportReview review={review} busy={busy !== null} bring={bring} cancel={cancel} />
      )}

      <Band says={t("inRepairBand")} />
      <Line says={t("inRepair")} why={<span role="status">{repaired ?? t("inRepairWhy")}</span>}>
        <button
          type="button"
          className="mild"
          disabled={busy !== null || review !== null}
          onClick={repair}
        >
          {busy === "repair" ? t("busy") : t("inRepairDo")}
        </button>
      </Line>
    </>
  );
}

function ImportReview({
  review,
  busy,
  bring,
  cancel,
}: {
  review: Review;
  busy: boolean;
  bring: () => void;
  cancel: (adjust?: boolean) => void;
}) {
  const overQuota = review.quota !== null && review.bytesAfter > review.quota;
  return (
    <section className="backup-review" aria-label={t("inReviewTitle")}>
      <Line
        stacked
        says={t("inReviewTitle")}
        why={<span className="path">{review.name}</span>}
        more={
          <>
            <p className="said-plain">
              {fill("inReviewCounts", items(review.added), items(review.already))}
            </p>
            {review.repaired > 0 && (
              <p className="said-plain">{fill("inWillRepair", items(review.repaired))}</p>
            )}
            {review.partial > 0 && (
              <p className="said">{fill("inPartial", items(review.partial))}</p>
            )}
            {review.refused > 0 && (
              <p className="said">{fill("inWillRefuse", items(review.refused))}</p>
            )}
            {review.fromElsewhere && <p className="said">{t("inElsewhere")}</p>}
            {review.expires > 0 && (
              <p className="said">{fill("inExpires", items(review.expires))}</p>
            )}
            {overQuota && (
              <p className="said">
                {fill("inOverQuota", sized(review.bytesAfter), sized(review.quota ?? 0))}
              </p>
            )}
            <p className="said-plain">{t("inPolicyReminder")}</p>
          </>
        }
      >
        <button type="button" className="mild" disabled={busy} onClick={() => cancel()}>
          {t("inCancel")}
        </button>
        {(review.expires > 0 || overQuota) && (
          <button type="button" className="mild" disabled={busy} onClick={() => cancel(true)}>
            {t("inAdjust")}
          </button>
        )}
        <button
          type="button"
          className="strong"
          disabled={busy || (review.added === 0 && review.repaired === 0)}
          onClick={bring}
        >
          {busy ? t("busy") : t("inConfirm")}
        </button>
      </Line>
    </section>
  );
}
