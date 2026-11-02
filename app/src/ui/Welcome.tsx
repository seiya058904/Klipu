import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { type ReactNode, useEffect, useState } from "react";
import { asKeys, caps, type Kept, useKept, useKeys } from "../core";
import { fill, t } from "../locales";
import mark from "../mark.svg";
import Chrome from "./Chrome";

export type Greeting = { kind: "tour" } | { kind: "keys" } | { kind: "nothing" };

const STEPS = ["hello", "keys", "where"] as const;

type Step = (typeof STEPS)[number];

function leave() {
  void getCurrentWindow().close();
}

export default function Welcome() {
  const { kept, trouble, change } = useKept();
  const [greeting, setGreeting] = useState<Greeting | null>(null);

  useEffect(() => {
    invoke<Greeting>("greeting")
      .then(setGreeting)
      .catch(() => setGreeting({ kind: "nothing" }));
    const heard = listen<Greeting>("greeting", (event) => setGreeting(event.payload));
    return () => {
      void heard.then((drop) => drop());
    };
  }, []);

  const lost = greeting?.kind === "nothing" || (trouble !== null && kept === null);
  useEffect(() => {
    if (lost) {
      leave();
    }
  }, [lost]);

  return (
    <>
      <Chrome knobs={false} />
      {kept && greeting?.kind === "tour" && <Tour kept={kept} change={change} />}
      {kept && greeting?.kind === "keys" && (
        <TryIt kept={kept} change={change} dots={null} next={leave} alone />
      )}
    </>
  );
}

function Tour({ kept, change }: { kept: Kept; change: (what: Partial<Kept>) => Promise<void> }) {
  const [at, setAt] = useState(0);

  const step: Step = STEPS[at];
  const next = () => (at + 1 < STEPS.length ? setAt(at + 1) : leave());
  const dots = <Dots at={at} />;

  switch (step) {
    case "hello":
      return (
        <Screen
          dots={dots}
          left={<Quiet says={t("welcomeSkip")} onPress={leave} />}
          right={<Strong says={t("welcomeStart")} onPress={next} />}
        >
          <div className="welcome-hello">
            <img className="welcome-mark" src={mark} alt="" />
            <h1>{t("welcomeHello")}</h1>
            <p>{t("welcomeHelloWhy")}</p>
            <div className="welcome-pills">
              <span>{t("welcomeLocal")}</span>
              <span>{t("welcomeNoAccount")}</span>
              <span>{t("welcomeNoTracking")}</span>
            </div>
          </div>
        </Screen>
      );
    case "keys":
      return <TryIt kept={kept} change={change} dots={dots} next={next} />;
    case "where":
      return (
        <Screen dots={dots} left={null} right={<Strong says={t("welcomeDone")} onPress={next} />}>
          <Heading
            over={t("welcomeWhereOver")}
            title={t("welcomeWhereTitle")}
            said={t("welcomeWhereWhy")}
          />
          <div className="welcome-where">
            <dl>
              <div>
                <dt>{t("welcomeClick")}</dt>
                <dd>{t("welcomeClickDoes")}</dd>
              </div>
              <div>
                <dt>{t("welcomeRightClick")}</dt>
                <dd>{t("welcomeRightClickDoes")}</dd>
              </div>
            </dl>
            <Tray />
          </div>
          <p className="welcome-aside">{t("welcomeUse")}</p>
        </Screen>
      );
  }
}

function TryIt({
  kept,
  change,
  dots,
  next,
  alone = false,
}: {
  kept: Kept;
  change: (what: Partial<Kept>) => Promise<void>;
  dots: ReactNode;
  next: () => void;
  alone?: boolean;
}) {
  const { keys, spare, recheck } = useKeys(kept.shortcut);
  const over = alone ? t("welcomeKeysAloneOver") : t("welcomeKeysOver");
  const skip = alone ? t("chromeClose") : t("welcomeSkipStep");
  const [tried, setTried] = useState(false);

  useEffect(() => {
    const heard = listen("panel-shown", () => setTried(true));
    return () => {
      void heard.then((drop) => drop());
    };
  }, []);

  const taken = keys !== null && !keys.bound;
  const shown = asKeys(kept.shortcut);

  if (taken) {
    return (
      <Screen dots={dots} left={<Quiet says={skip} onPress={next} />} right={null}>
        <Heading
          over={over}
          title={t("welcomeKeysTaken")}
          said={fill(spare.length > 0 ? "welcomeKeysTakenWhy" : "welcomeKeysTakenAlone", shown)}
        />
        {spare.length > 0 && (
          <div className="welcome-spare">
            {spare.map((one) => (
              <button
                key={one}
                type="button"
                onClick={() => void change({ shortcut: one }).finally(() => recheck(one))}
              >
                {asKeys(one)}
              </button>
            ))}
          </div>
        )}
        <p className="welcome-aside">{t("welcomeKeysOwn")}</p>
      </Screen>
    );
  }

  return (
    <Screen
      dots={dots}
      left={
        tried ? (
          <Quiet says={t("welcomeTryAgain")} onPress={() => setTried(false)} />
        ) : (
          <Quiet says={skip} onPress={next} />
        )
      }
      right={
        tried ? <Strong says={alone ? t("welcomeDone") : t("welcomeNext")} onPress={next} /> : null
      }
    >
      <Heading over={over} title={t("welcomeKeysTitle")} said={t("welcomeKeysWhy")} />
      <div className="welcome-caps">
        {caps(kept.shortcut).map((one, index) => (
          <span key={one} className="welcome-cap-pair">
            {index > 0 && <span className="welcome-plus">+</span>}
            <kbd>{one}</kbd>
          </span>
        ))}
      </div>
      {tried ? (
        <Done line={t("welcomeKeysDone")} small={t("welcomeKeysDoneWhy")} />
      ) : (
        <Waiting line={t("welcomeKeysWaiting")} small={t("welcomeKeysWaitingWhy")} />
      )}
    </Screen>
  );
}

function Screen({
  dots,
  left,
  right,
  children,
}: {
  dots: ReactNode;
  left: ReactNode;
  right: ReactNode;
  children: ReactNode;
}) {
  return (
    <main className="welcome">
      <div className="welcome-body">{children}</div>
      <footer className="welcome-foot">
        <div className="welcome-left">{left}</div>
        {dots}
        <div className="welcome-right">{right}</div>
      </footer>
    </main>
  );
}

function Dots({ at }: { at: number }) {
  return (
    <div
      className="welcome-dots"
      role="img"
      aria-label={fill("welcomeStep", `${at + 1} / ${STEPS.length}`)}
    >
      {STEPS.map((one, index) => (
        <span key={one} className={index === at ? "on" : undefined} />
      ))}
    </div>
  );
}

function Heading({ over, title, said }: { over: string; title: string; said: string }) {
  return (
    <div className="welcome-heading">
      <span className="welcome-over">{over}</span>
      <h1>{title}</h1>
      <p>{said}</p>
    </div>
  );
}

function Waiting({ line, small }: { line: string; small: string }) {
  return (
    <div className="welcome-state" aria-live="polite">
      <span className="welcome-pulse" />
      <div>
        <b>{line}</b>
        <span>{small}</span>
      </div>
    </div>
  );
}

function Done({ line, small }: { line: string; small?: string }) {
  return (
    <div className="welcome-state done" aria-live="polite">
      <svg viewBox="0 0 18 18" aria-hidden="true">
        <path d="M3.5 9.5l3.5 3.5 7.5-8" />
      </svg>
      <div>
        <b>{line}</b>
        {small && <span>{small}</span>}
      </div>
    </div>
  );
}

function Quiet({ says, onPress }: { says: string; onPress: () => void }) {
  return (
    <button type="button" className="welcome-quiet" onClick={onPress}>
      {says}
    </button>
  );
}

function Strong({ says, onPress }: { says: string; onPress: () => void }) {
  return (
    <button type="button" className="strong welcome-strong" onClick={onPress}>
      {says}
    </button>
  );
}

function Tray() {
  return (
    <div className="welcome-tray" aria-hidden="true">
      <div className="welcome-hidden">
        <span />
        <span className="ours">
          <i>K</i>
        </span>
        <span />
        <span />
      </div>
      <div className="welcome-bar">
        <span className="arrow">
          <svg viewBox="0 0 12 12" aria-hidden="true">
            <path d="M2.5 7.5L6 4l3.5 3.5" />
          </svg>
        </span>
        <span className="glyph" />
        <span className="glyph" />
        <span className="clock">
          16:40
          <br />
          02/10
        </span>
      </div>
    </div>
  );
}
