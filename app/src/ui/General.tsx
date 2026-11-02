import { type Kept, useWaking } from "../core";
import { t } from "../locales";
import { Band, Knob, Line } from "./Bits";

export default function General({
  kept,
  change,
}: {
  kept: Kept;
  change: (what: Partial<Kept>) => Promise<void>;
}) {
  const { waking, trouble, ask } = useWaking();

  return (
    <>
      <h1>{t("railGeneral")}</h1>

      <Band says={t("bandTongue")} />

      <Line says={t("tongue")} why={t("tongueWhy")}>
        <select
          aria-label={t("tongue")}
          value={kept.locale ?? ""}
          onChange={(event) => change({ locale: event.target.value || null })}
        >
          <option value="">{t("tongueTheirs")}</option>
          <option value="zh">简体中文</option>
          <option value="en">English</option>
        </select>
      </Line>

      <Band says={t("bandDoes")} />

      {waking?.offered && (
        <Line
          says={t("wake")}
          why={t("wakeWhy")}
          more={
            waking.theirs ? (
              <div className="said">{t("wakeTheirs")}</div>
            ) : trouble ? (
              <div className="alarm">{trouble}</div>
            ) : null
          }
        >
          <Knob
            on={waking.wakes}
            says={t("wake")}
            asleep={waking.theirs && !waking.wakes}
            onPress={() => ask(!waking.wakes)}
          />
        </Line>
      )}

      <Line says={t("hides")} why={t("hidesWhy")}>
        <Knob
          on={kept["hides-when-left"]}
          says={t("hides")}
          onPress={() => change({ "hides-when-left": !kept["hides-when-left"] })}
        />
      </Line>
    </>
  );
}
