import { act, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { adopt } from "../locales";
import Ready, { LASTS_MS, midnight } from "../ui/Ready";
import { scene, theWindow } from "./setup";

async function asked() {
  const { invoke } = await import("@tauri-apps/api/core");
  return vi.mocked(invoke);
}

describe("the ready card", () => {
  afterEach(() => {
    vi.useRealTimers();
    theWindow.close.mockClear();
  });

  it("tells what the history holds", async () => {
    adopt("zh");
    scene.glance = { items: 12480, today: 37, pinned: 18, bytes: 84 * 1024 ** 2 };
    render(<Ready />);
    expect(await screen.findByText("Klipu 已就绪")).toBeInTheDocument();
    expect(screen.getByText("12,480")).toBeInTheDocument();
    expect(screen.getByText("37")).toBeInTheDocument();
    expect(screen.getByText("18")).toBeInTheDocument();
    expect(screen.getByText("84 MB")).toBeInTheDocument();
    expect(screen.getByText("Ctrl")).toBeInTheDocument();
  });

  it("invites to copy something when the history is empty", async () => {
    adopt("zh");
    render(<Ready />);
    expect(await screen.findByText(/还没有内容/)).toBeInTheDocument();
    expect(screen.queryByText("已保存")).toBeNull();
  });

  it("says nothing about the history when it could not be read", async () => {
    adopt("zh");
    const invoke = await asked();
    const real = invoke.getMockImplementation() as (what: string, args?: never) => Promise<unknown>;
    invoke.mockImplementation(((what: string, args?: never) =>
      what === "glance" ? Promise.reject(new Error("locked")) : real(what, args)) as never);
    render(<Ready />);
    expect(await screen.findByText("Klipu 已就绪")).toBeInTheDocument();
    expect(screen.queryByText(/还没有内容/)).toBeNull();
    expect(screen.queryByText("已保存")).toBeNull();
    invoke.mockImplementation(real as never);
  });

  it("counts today from local midnight", async () => {
    adopt("zh");
    render(<Ready />);
    await screen.findByText("Klipu 已就绪");
    expect(await asked()).toHaveBeenCalledWith("glance", { since: midnight() });
  });

  it("opens the history or the settings and goes away", async () => {
    adopt("zh");
    const who = userEvent.setup();
    render(<Ready />);
    await who.click(await screen.findByRole("button", { name: "查看历史" }));
    expect(await asked()).toHaveBeenCalledWith("show_panel");
    await who.click(screen.getByRole("button", { name: "设置" }));
    expect(await asked()).toHaveBeenCalledWith("open_settings", { rail: "general" });
    expect(theWindow.close).toHaveBeenCalled();
  });

  it("leaves on its own, but waits while the mouse is over it", async () => {
    adopt("zh");
    vi.useFakeTimers({ shouldAdvanceTime: true });
    render(<Ready />);
    await screen.findByText("Klipu 已就绪");
    const card = document.documentElement;
    fireEvent.mouseEnter(card);
    act(() => vi.advanceTimersByTime(LASTS_MS * 2));
    expect(theWindow.close).not.toHaveBeenCalled();
    fireEvent.mouseLeave(card);
    act(() => vi.advanceTimersByTime(LASTS_MS));
    expect(theWindow.close).toHaveBeenCalled();
  });
});

describe("midnight", () => {
  it("is the start of the local day", () => {
    const at = new Date(2026, 9, 16, 15, 42, 7);
    expect(midnight(at)).toBe(new Date(2026, 9, 16).getTime());
  });
});
