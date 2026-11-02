import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { adopt, type Binding, panelKeys } from "../locales";

function ui(name: string) {
  return readFileSync(join(__dirname, "..", "..", "..", "crates", "cp-panel", "ui", name), "utf8");
}

const PANEL = ui("panel.slint");

const KEYS_READ = PANEL.slice(PANEL.indexOf("key-pressed(event)"));

const BRANCHES = KEYS_READ.split("return accept;").slice(0, -1);

// what the branch asks for before it runs, which is where a modifier is required rather than read
function conditionOf(branch: string) {
  const at = branch.lastIndexOf("if ");
  const said = at === -1 ? branch : branch.slice(at);
  const opens = said.indexOf("{");
  return opens === -1 ? said : said.slice(0, opens);
}

// a key promised alone has to be read alone: a branch that demands a modifier alongside it
// answers a different chord, which is how Enter kept its promise from the «paste as» branch
function readsItAlone(branch: string, key: string) {
  return conditionOf(branch)
    .split("||")
    .filter((one) => one.includes(key))
    .some((one) => !one.includes("modifiers."));
}

function answers(branch: string, needs: string[]) {
  if (!needs.every((one) => branch.includes(one))) {
    return false;
  }
  return needs.length > 1 || readsItAlone(branch, needs[0]);
}

const LOOKED_FOR: Record<string, string> = {
  enter: "event.text == Key.Return",
  "⏎": "event.text == Key.Return",
  esc: "event.text == Key.Escape",
  retroceso: "event.text == Key.Backspace",
  backspace: "event.text == Key.Backspace",
  "⌫": "event.text == Key.Backspace",
  supr: "event.text == Key.Delete",
  delete: "event.text == Key.Delete",
  tab: "event.text == Key.Tab",
  "⇥": "event.text == Key.Tab",
  f1: "event.text == Key.F1",
  f2: "event.text == Key.F2",
  "flecha derecha": "event.text == Key.RightArrow",
  "right arrow": "event.text == Key.RightArrow",
  "→": "event.text == Key.RightArrow",
};

const CLICKS = new Set(["单击", "click", "双击", "double click"]);

const MODIFIER: Record<string, string> = {
  ctrl: "modifiers.control",
  alt: "modifiers.alt",
  shift: "modifiers.shift",
  "⌘": "modifiers.control",
  "⌥": "modifiers.alt",
  "⇧": "modifiers.shift",
  "⌃": "modifiers.meta",
};

const GLYPHS = new Set(["⌘", "⌥", "⇧", "⌃"]);

const DESCRIBED = new Set([
  "flechas",
  "arrows",
  "uno",
  "varios",
  "one",
  "many",
  "clic",
  "click",
  "doble clic",
  "double click",
  "space",
  "方向键",
  "单击",
  "双击",
]);

// a Mac combination carries no separator: ⌘⌫ is the Command key and the one it holds
function pieces(combination: string): string[] {
  const said = combination.trim();
  if (said.includes("+")) {
    return said
      .split("+")
      .map((one) => one.trim())
      .filter((one) => one.length > 0);
  }
  const letters = [...said];
  let at = 0;
  while (at < letters.length && GLYPHS.has(letters[at])) {
    at += 1;
  }
  const held = letters.slice(0, at);
  const key = letters.slice(at).join("");
  return key.length > 0 ? [...held, key] : held;
}

function wanted(last: string) {
  const one = last.toLowerCase();
  const looked = LOOKED_FOR[one];
  if (looked) {
    return looked;
  }
  if (one.length === 1) {
    return `event.text == "${one}"`;
  }
  if (DESCRIBED.has(one) || one.startsWith("#")) {
    return null;
  }
  throw new Error(`«${last}» is neither a key the panel looks for nor a described gesture`);
}

function asked(combination: string): string[] | null {
  const parts = pieces(combination);
  const last = parts.at(-1);
  if (!last) {
    return null;
  }
  const looked = wanted(last);
  if (!looked) {
    return null;
  }
  return [
    looked,
    ...parts.slice(0, -1).flatMap((held) => {
      const one = MODIFIER[held.toLowerCase()];
      expect(one, `«${held}» is not a modifier`).toBeTruthy();
      return [one];
    }),
  ];
}

function rows(): Binding[] {
  return ["zh", "en"].flatMap((tongue) => {
    adopt(tongue);
    return panelKeys();
  });
}

describe("the shortcut table", () => {
  it("promises only keys the panel really watches", () => {
    for (const row of rows()) {
      for (const combination of row.keys.split("·")) {
        const needs = asked(combination);
        if (!needs) {
          continue;
        }
        const found = BRANCHES.some((branch) => answers(branch, needs));
        expect(found, `«${row.keys}» promises ${needs.join(" + ")} and no branch reads it`).toBe(
          true,
        );
      }
    }
  });

  it("says the same in both languages, row by row", () => {
    adopt("zh");
    const es = panelKeys();
    adopt("en");
    const en = panelKeys();
    expect(es.length).toBe(en.length);
    expect(es.length).toBeGreaterThan(0);
    for (let at = 0; at < es.length; at += 1) {
      expect(es[at].keys.length).toBeGreaterThan(0);
      expect(en[at].keys.length).toBeGreaterThan(0);
      expect(es[at].does).not.toBe(en[at].does);
    }
  });

  it("a click with a modifier really reads what is clicked", () => {
    const cards = ui("cards.slint");
    // the layers row tells this one in words instead of naming a key, so nothing above reaches it
    expect(cards, "no clic suma tipos en vez de cambiarlos").toContain(
      "self.adding = event.modifiers.control",
    );
    for (const row of rows()) {
      for (const combination of row.keys.split("·")) {
        const parts = pieces(combination).map((one) => one.toLowerCase());
        const last = parts.at(-1);
        if (!last || !CLICKS.has(last) || parts.length < 2) {
          continue;
        }
        for (const held of parts.slice(0, -1)) {
          const looked = MODIFIER[held];
          expect(looked, `«${held}» is not a modifier`).toBeTruthy();
          expect(
            cards,
            `«${row.keys}» promises ${looked} and nothing that is clicked reads it`,
          ).toContain(looked);
        }
      }
    }
  });
});
