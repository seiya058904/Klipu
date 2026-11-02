import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { t } from "../locales";
import Welcome from "../ui/Welcome";
import { scene as asked, theWindow } from "./setup";

type Heard = (event: { payload: unknown }) => void;

async function heardOn(name: string): Promise<Heard> {
  const { listen } = await import("@tauri-apps/api/event");
  await waitFor(() => {
    expect(vi.mocked(listen).mock.calls.some(([what]) => what === name)).toBe(true);
  });
  const call = vi.mocked(listen).mock.calls.find(([what]) => what === name) as [string, Heard];
  return call[1];
}

async function press(name: string) {
  fireEvent.click(await screen.findByRole("button", { name }));
}

describe("the welcome of a fresh install", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    asked.greeting = { kind: "tour" };
  });

  it("teaches the shortcut until it is tried, shows the tray and closes at the end", async () => {
    render(<Welcome />);
    await press(t("welcomeStart"));
    await screen.findByRole("heading", { name: t("welcomeKeysTitle") });
    expect(screen.queryByRole("button", { name: t("welcomeNext") })).toBeNull();

    const shown = await heardOn("panel-shown");
    act(() => shown({ payload: null }));
    await press(t("welcomeNext"));

    await screen.findByRole("heading", { name: t("welcomeWhereTitle") });
    await press(t("welcomeDone"));
    await waitFor(() => expect(theWindow.close).toHaveBeenCalled());
  });

  it("offers the free shortcuts when its own is taken", async () => {
    asked.bound = false;
    render(<Welcome />);
    await press(t("welcomeStart"));
    await screen.findByRole("heading", { name: t("welcomeKeysTaken") });
    await screen.findByRole("button", { name: "Ctrl + Alt + J" });
  });

  it("skipping from the start closes the window without going through anything", async () => {
    render(<Welcome />);
    await press(t("welcomeSkip"));
    await waitFor(() => expect(theWindow.close).toHaveBeenCalled());
  });

  it("with nothing to show it closes itself instead of staying blank", async () => {
    asked.greeting = { kind: "nothing" };
    render(<Welcome />);
    await waitFor(() => expect(theWindow.close).toHaveBeenCalled());
  });
});

describe("opening the app while it is already running", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    asked.greeting = { kind: "keys" };
  });

  it("teaches only the shortcut, without the tour", async () => {
    render(<Welcome />);
    await screen.findByRole("heading", { name: t("welcomeKeysTitle") });
    expect(screen.queryByRole("button", { name: t("welcomeStart") })).toBeNull();
    await press(t("chromeClose"));
    await waitFor(() => expect(theWindow.close).toHaveBeenCalled());
  });
});
