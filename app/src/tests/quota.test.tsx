import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { type Kept, nearLimit } from "../core";
import { adopt, sized } from "../locales";
import History from "../ui/History";
import { scene as asked } from "./setup";

const A_GIGABYTE = 1024 ** 3;

async function heardOn(name: string): Promise<(event: unknown) => void> {
  const { listen } = await import("@tauri-apps/api/event");
  const calls = () => vi.mocked(listen).mock.calls.filter(([what]) => what === name);
  await waitFor(() => expect(calls().length).toBeGreaterThan(0));
  return calls().at(-1)?.[1] as (event: unknown) => void;
}

const kept = (quotaMb: number): Kept => ({
  locale: "zh",
  theme: "system",
  shortcut: "Ctrl+Alt+K",
  "hides-when-left": true,
  "keeps-days": 30,
  "images-quota-mb": quotaMb,
});

describe("when the space is near its limit", () => {
  it("is near from seventy percent of the limit on", () => {
    const limitBytes = 5120 * 1024 * 1024;
    expect(nearLimit(limitBytes * 0.7, 5120)).toBe(true);
    expect(nearLimit(limitBytes * 0.7 - 1, 5120)).toBe(false);
    expect(nearLimit(limitBytes, 5120)).toBe(true);
    expect(nearLimit(limitBytes * 2, 5120)).toBe(true);
    expect(nearLimit(0, 5120)).toBe(false);
  });

  it("is never near when there is no limit", () => {
    expect(nearLimit(500 * A_GIGABYTE, 0)).toBe(false);
  });
});

describe("how a size is said", () => {
  afterEach(() => adopt("zh"));

  it("uses gigabytes from one on, with the decimal mark of the language", () => {
    expect(sized(1.4 * A_GIGABYTE)).toBe("1.4 GB");
    expect(sized(2 * A_GIGABYTE)).toBe("2 GB");
    adopt("en");
    expect(sized(1.4 * A_GIGABYTE)).toBe("1.4 GB");
  });

  it("uses megabytes below a gigabyte", () => {
    expect(sized(300 * 1024 * 1024)).toBe("300 MB");
    expect(sized(0)).toBe("0 MB");
  });
});

describe("the space of the history in Settings", () => {
  afterEach(() => adopt("zh"));

  it("tells how much is used against the limit and warns at seventy percent", async () => {
    adopt("zh");
    asked.usedBytes = 4 * A_GIGABYTE;
    render(<History kept={kept(5120)} change={vi.fn()} />);
    expect(await screen.findByText("历史记录已占用 4 GB，上限 5 GB")).toBeInTheDocument();
    const near = screen.getByText(/最早的未置顶/);
    expect(near).toHaveClass("said-plain");
    expect(near).not.toHaveClass("said");
  });

  it("tells the usage without warning while it is below seventy percent", async () => {
    adopt("zh");
    asked.usedBytes = 3.4 * A_GIGABYTE;
    render(<History kept={kept(5120)} change={vi.fn()} />);
    expect(await screen.findByText("历史记录已占用 3.4 GB，上限 5 GB")).toBeInTheDocument();
    expect(screen.queryByText(/最早的未置顶/)).toBeNull();
  });

  it("says nothing about usage when there is no limit", async () => {
    adopt("zh");
    asked.usedBytes = 5 * A_GIGABYTE;
    render(<History kept={kept(0)} change={vi.fn()} />);
    expect(await screen.findByLabelText("空间上限")).toBeInTheDocument();
    await screen.findByText(/C:Users/);
    expect(screen.queryByText(/历史记录已占用/)).toBeNull();
    expect(screen.queryByText(/最早的未置顶/)).toBeNull();
  });

  it("offers the five gigabyte limit and a larger one", async () => {
    render(<History kept={kept(5120)} change={vi.fn()} />);
    expect(await screen.findByRole("option", { name: "5 GB" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "10 GB" })).toBeInTheDocument();
  });

  it("speaks English too", async () => {
    adopt("en");
    asked.usedBytes = 4 * A_GIGABYTE;
    render(<History kept={kept(5120)} change={vi.fn()} />);
    expect(await screen.findByText("The history takes up 4 GB of 5 GB")).toBeInTheDocument();
    expect(screen.getByText(/oldest unpinned items are removed/)).toBeInTheDocument();
  });

  it("weighs the history again once it is emptied", async () => {
    adopt("zh");
    asked.usedBytes = 4 * A_GIGABYTE;
    render(<History kept={kept(5120)} change={vi.fn()} />);
    expect(await screen.findByText("历史记录已占用 4 GB，上限 5 GB")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "清空" }));
    await userEvent.click(screen.getByRole("button", { name: "确定吗？" }));
    expect(await screen.findByText("已删除 3 项，置顶内容已保留")).toBeInTheDocument();
    expect(screen.getByText("历史记录已占用 4 GB，上限 5 GB")).toBeInTheDocument();
    asked.usedBytes = 0;
    const emptied = await heardOn("emptied");
    act(() => emptied({ payload: null }));
    expect(await screen.findByText("历史记录已占用 0 MB，上限 5 GB")).toBeInTheDocument();
    expect(screen.queryByText(/最早的未置顶/)).toBeNull();
  });

  it("weighs the history again when another window keeps settings or 2.x crosses", async () => {
    adopt("zh");
    asked.usedBytes = 4 * A_GIGABYTE;
    render(<History kept={kept(5120)} change={vi.fn()} />);
    expect(await screen.findByText("历史记录已占用 4 GB，上限 5 GB")).toBeInTheDocument();
    for (const [name, gb] of [
      ["kept", 1],
      ["crossed", 3.6],
    ] as const) {
      const heard = await heardOn(name);
      asked.usedBytes = gb * A_GIGABYTE;
      act(() => heard({ payload: null }));
      expect(await screen.findByText(`历史记录已占用 ${gb} GB，上限 5 GB`)).toBeInTheDocument();
    }
  });
});
