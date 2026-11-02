import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "../App";
import type { Kept } from "../core";
import { adopt } from "../locales";
import { accentSaid, previewStyle } from "../ui/Appearance";

async function opened() {
  const who = userEvent.setup();
  render(<App />);
  await who.click(await screen.findByRole("button", { name: "外观" }));
  return who;
}

async function saved() {
  const { invoke } = await import("@tauri-apps/api/core");
  return vi.mocked(invoke);
}

const KEPT: Kept = {
  locale: "zh",
  theme: "system",
  shortcut: "Ctrl+Alt+K",
  "hides-when-left": true,
  "keeps-days": 30,
  "images-quota-mb": 0,
};

const CHOICES = {
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
};

describe("the appearance", () => {
  beforeEach(() => {
    adopt("zh");
    vi.clearAllMocks();
  });

  it("starts on what the panel always looked like, each one marked as the default", async () => {
    await opened();
    const size = screen.getByRole("radiogroup", { name: "缩放" });
    expect(within(size).getByRole("radio", { name: "标准" })).toBeChecked();
    expect(screen.getByRole("radio", { name: "靛蓝 · 默认" })).toBeChecked();
    const height = screen.getByRole("radiogroup", { name: "条目高度" });
    expect(within(height).getByRole("radio", { name: "标准" })).toBeChecked();
  });

  it("each choice is sent to be saved as the panel reads it", async () => {
    const who = await opened();
    const invoke = await saved();
    await who.click(screen.getByRole("radio", { name: "更大" }));
    expect(invoke).toHaveBeenCalledWith("keep", {
      config: expect.objectContaining({ "text-size": "larger" }),
    });
    await who.click(screen.getByRole("radio", { name: "宽松" }));
    expect(invoke).toHaveBeenCalledWith("keep", {
      config: expect.objectContaining({ density: "comfortable" }),
    });
    await who.click(screen.getByRole("radio", { name: "青色" }));
    expect(invoke).toHaveBeenCalledWith("keep", {
      config: expect.objectContaining({ accent: "teal" }),
    });
  });

  it("resetting brings back size, height and colour but leaves the theme alone", async () => {
    const who = await opened();
    const invoke = await saved();
    await who.selectOptions(screen.getByLabelText("主题"), "dark");
    await who.click(screen.getByRole("radio", { name: "大" }));
    await who.click(screen.getByRole("button", { name: "恢复默认外观" }));
    expect(invoke).toHaveBeenCalledWith("keep", {
      config: expect.objectContaining({
        theme: "dark",
        "text-size": "normal",
        density: "normal",
        accent: "indigo",
      }),
    });
  });

  it("the size and the height move with the arrow keys, as one stop each", async () => {
    const who = await opened();
    const size = screen.getByRole("radiogroup", { name: "缩放" });
    within(size).getByRole("radio", { name: "标准" }).focus();
    await who.keyboard("{ArrowRight}");
    expect(screen.getByRole("radio", { name: "大" })).toBeChecked();
  });

  it("the preview wears the chosen size, lines and colour", () => {
    const style = previewStyle(
      { ...KEPT, "text-size": "large", density: "compact", accent: "teal" },
      CHOICES,
    ) as Record<string, unknown>;
    expect(style["--zoom"]).toBe(1.08);
    expect(style["--lines"]).toBe(1);
    expect(style["--light"]).toBe("#0F766E");
    expect(style["--dark"]).toBe("#5EEAD4");
    expect(style["--light-selected"]).toBe("#DFF3F0");
    expect(style["--dark-selected"]).toBe("#1E3639");
  });

  it("an untouched file previews as the defaults", () => {
    const style = previewStyle(KEPT, CHOICES) as Record<string, unknown>;
    expect(style["--zoom"]).toBe(1);
    expect(style["--lines"]).toBe(2);
    expect(style["--light"]).toBe("#4F46E5");
  });

  it("names a colour it does not know by its id", () => {
    expect(accentSaid("teal", false)).toBe("青色");
    expect(accentSaid("violeta", false)).toBe("violeta");
  });
});

describe("each choice group", () => {
  it("is a single Tab stop on the chosen option", async () => {
    adopt("zh");
    const who = userEvent.setup();
    render(<App />);
    await who.click(await screen.findByRole("button", { name: "外观" }));
    const size = screen.getByRole("radiogroup", { name: "缩放" });
    const stops = within(size)
      .getAllByRole("radio")
      .map((one) => one.getAttribute("tabindex"));
    expect(stops).toEqual(["-1", "0", "-1", "-1"]);
    const colours = screen.getByRole("radiogroup", { name: "强调色" });
    expect(
      within(colours)
        .getAllByRole("radio")
        .filter((one) => one.getAttribute("tabindex") === "0"),
    ).toHaveLength(1);
  });
});
