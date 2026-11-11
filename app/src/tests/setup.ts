import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach, beforeEach, vi } from "vitest";
import { adopt } from "../locales";

const asked = vi.hoisted(() => ({
  greeting: { kind: "tour" } as unknown,
  bound: true,
  locale: "zh",
  usedBytes: 0,
  glance: { items: 0, today: 0, pinned: 0, bytes: 0 },
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  save: vi.fn(() => Promise.resolve("/donde/quiera/Klipu.klipubak")),
  open: vi.fn(() => Promise.resolve("/donde/quiera/Klipu.klipubak")),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(() => Promise.resolve()),
  revealItemInDir: vi.fn(() => Promise.resolve()),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn((what: string, args?: { config?: unknown; wanted?: boolean }) => {
    if (what === "settings") {
      return Promise.resolve({
        locale: asked.locale,
        theme: "system",
        shortcut: "Ctrl+Alt+K",
        "plain-shortcut": "Ctrl+Alt+Shift+K",
        "hides-when-left": true,
        "keeps-days": 30,
        "images-quota-mb": 0,
      });
    }
    if (what === "keep") {
      const config = args?.config as Record<string, unknown>;
      for (const key of ["keeps-days", "images-quota-mb"]) {
        if (typeof config?.[key] !== "number") {
          return Promise.reject(new Error(`invalid type for ${key}: expected a number`));
        }
      }
      return Promise.resolve(config);
    }
    if (what === "looks") {
      return Promise.resolve({
        text: { id: "", label: "SF Pro", css: "-apple-system" },
        texts: [{ id: "avenir-next", label: "Avenir Next", css: '"Avenir Next"' }],
        code: { id: "", label: "Menlo", css: "Menlo" },
        codes: [{ id: "sf-mono", label: "SF Mono", css: "ui-monospace" }],
        accents: [
          {
            id: "indigo",
            light: "#4F46E5",
            dark: "#A5B4FC",
            lightSelected: "#E9E9FB",
            darkSelected: "#282C46",
          },
          {
            id: "teal",
            light: "#0F766E",
            dark: "#5EEAD4",
            lightSelected: "#DFF3F0",
            darkSelected: "#1E3639",
          },
        ],
      });
    }
    if (what === "relabel") return Promise.resolve(null);
    if (what === "empty") return Promise.resolve(3);
    if (what === "trouble") return Promise.resolve(null);
    if (what === "keys") {
      const role = (args as { role?: string } | undefined)?.role ?? "panel";
      const wanted = { panel: "Ctrl+Alt+K", plain: "Ctrl+Alt+Shift+K" }[role];
      return Promise.resolve({ wanted, bound: role === "panel" ? asked.bound : true });
    }
    if (what === "waking") {
      return Promise.resolve({ offered: true, wakes: false, theirs: false, managed: false });
    }
    if (what === "wake") {
      return Promise.resolve({
        offered: true,
        wakes: Boolean(args?.wanted),
        theirs: false,
        managed: false,
      });
    }
    if (what === "storage_used") return Promise.resolve(asked.usedBytes);
    if (what === "where_it_lives") return Promise.resolve("C:UsersquienAppDataLocalCopyPaste");
    if (what === "save_backup") {
      return Promise.resolve({ path: "/donde/quiera/Klipu.klipubak", items: 3, bytes: 2048 });
    }
    if (what === "spare") {
      return Promise.resolve(["Ctrl+Alt+J", "Ctrl+Alt+Shift+V"]);
    }
    if (what === "peek_backup") {
      return Promise.resolve({
        token: "backup-1",
        name: "Klipu.klipubak",
        added: 2,
        already: 1,
        refused: 0,
        partial: 0,
        repaired: 0,
        expires: 0,
        bytesAfter: 2048,
        quota: null,
        keepFor: null,
        fromElsewhere: false,
      });
    }
    if (what === "cancel_backup") return Promise.resolve();
    if (what === "repair_search") return Promise.resolve(0);
    if (what === "load_backup") {
      return Promise.resolve({
        kind: "done",
        result: { added: 2, already: 1, refused: 0, partial: 0, repaired: 0, fromElsewhere: false },
      });
    }
    if (what === "open_web") {
      return Promise.resolve();
    }
    if (what === "greeting") return Promise.resolve(asked.greeting);
    if (what === "glance") return Promise.resolve({ ...asked.glance });
    if (what === "show_panel") return Promise.resolve(null);
    if (what === "open_settings" || what === "tour") return Promise.resolve(null);
    return Promise.reject(new Error(`sin simular: ${what}`));
  }),
}));

vi.mock("@tauri-apps/api/app", () => ({
  getVersion: vi.fn(() => Promise.resolve("3.0.0")),
}));

export const scene = asked;

export const theWindow = {
  minimize: vi.fn(() => Promise.resolve()),
  close: vi.fn(() => Promise.resolve()),
};

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => theWindow,
}));

afterEach(() => {
  cleanup();
  asked.greeting = { kind: "tour" };
  asked.bound = true;
  asked.locale = "zh";
  asked.usedBytes = 0;
  asked.glance = { items: 0, today: 0, pinned: 0, bytes: 0 };
});

export function forPanel(args: unknown) {
  const role = (args as { role?: string } | undefined)?.role;
  return role === undefined || role === "panel";
}

beforeEach(() => adopt("zh"));
