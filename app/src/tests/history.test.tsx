import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Kept } from "../core";
import History from "../ui/History";

const kept: Kept = {
  locale: "zh",
  theme: "system",
  shortcut: "Ctrl+Alt+K",
  "hides-when-left": true,
  "keeps-days": 30,
  "images-quota-mb": 0,
};

const real = vi.mocked(invoke).getMockImplementation();

function answerWith(answer: () => Promise<unknown>) {
  vi.mocked(invoke).mockImplementation((what, args) =>
    what === "empty" ? answer() : real ? real(what, args) : Promise.reject(what),
  );
}

async function confirm() {
  const user = userEvent.setup();
  await user.click(screen.getByRole("button", { name: "清空" }));
  await user.click(screen.getByRole("button", { name: "确定吗？" }));
}

afterEach(() => {
  vi.mocked(invoke).mockImplementation(real ?? (() => Promise.resolve()));
});

describe("clearing history waits for the actual result", () => {
  it("stays busy until completion and prevents repeated submission", async () => {
    let done: (gone: number) => void = () => {};
    const request = vi.fn(
      () =>
        new Promise<number>((resolve) => {
          done = resolve;
        }),
    );
    answerWith(request);
    render(<History kept={kept} change={vi.fn()} />);
    await confirm();
    expect(screen.getByRole("button", { name: "正在清空…" })).toBeDisabled();
    expect(screen.queryByText(/已删除/)).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "正在清空…" }));
    expect(request).toHaveBeenCalledTimes(1);
    await act(async () => done(12));
    expect(await screen.findByText("已删除 12 项，置顶内容已保留")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "清空" })).toBeEnabled();
  });

  it("reports a failure without claiming success and permits a successful retry", async () => {
    const request = vi.fn().mockRejectedValueOnce("empty-failed").mockResolvedValueOnce(2);
    answerWith(request);
    render(<History kept={kept} change={vi.fn()} />);
    await confirm();
    expect(await screen.findByText(/无法清空历史记录/)).toBeInTheDocument();
    expect(screen.queryByText(/已删除/)).not.toBeInTheDocument();
    await confirm();
    expect(await screen.findByText("已删除 2 项，置顶内容已保留")).toBeInTheDocument();
    expect(screen.queryByText(/无法清空历史记录/)).not.toBeInTheDocument();
  });

  it.each([
    ["empty-stopped", "面板已停止"],
    ["empty-timeout", "清空尚未确认完成"],
    ["empty-busy", "历史记录正在处理其他操作"],
  ])("explains %s without promising deletion", async (reason, message) => {
    answerWith(() => Promise.reject(reason));
    render(<History kept={kept} change={vi.fn()} />);
    await confirm();
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent(message));
    expect(screen.queryByText(/已删除/)).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "清空" })).toBeEnabled();
  });

  it("replaces an unconfirmed timeout with a late completion", async () => {
    answerWith(() => Promise.reject("empty-timeout"));
    render(<History kept={kept} change={vi.fn()} />);
    await confirm();
    expect(screen.getByRole("status")).toHaveTextContent("清空尚未确认完成");
    const callback = vi
      .mocked(listen)
      .mock.calls.filter(([name]) => name === "emptied")
      .at(-1)?.[1];
    expect(callback).toBeDefined();
    act(() => callback?.({ event: "emptied", id: 1, payload: 4 }));
    expect(screen.getByRole("status")).toHaveTextContent("已删除 4 项，置顶内容已保留");
  });

  it("replaces an unconfirmed timeout with a late failure", async () => {
    answerWith(() => Promise.reject("empty-timeout"));
    render(<History kept={kept} change={vi.fn()} />);
    await confirm();
    expect(screen.getByRole("status")).toHaveTextContent("清空尚未确认完成");
    const callback = vi
      .mocked(listen)
      .mock.calls.filter(([name]) => name === "empty-failed")
      .at(-1)?.[1];
    expect(callback).toBeDefined();
    act(() => callback?.({ event: "empty-failed", id: 1, payload: null }));
    expect(screen.getByRole("status")).toHaveTextContent("无法清空历史记录");
    expect(screen.getByRole("status")).not.toHaveTextContent("尚未确认");
  });
});
