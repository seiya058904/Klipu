import { invoke } from "@tauri-apps/api/core";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import Backup from "../ui/Backup";

const preview = {
  token: "reviewed-snapshot",
  name: "old.cpbackup",
  added: 2,
  already: 1,
  partial: 0,
  refused: 0,
  repaired: 0,
  expires: 0,
  bytesAfter: 2048,
  quota: null,
  keepFor: null,
  fromElsewhere: false,
};

const original = vi.mocked(invoke).getMockImplementation();
afterEach(() => {
  vi.mocked(invoke).mockImplementation(original as never);
  vi.mocked(invoke).mockClear();
});

function answers(values: Record<string, unknown>) {
  vi.mocked(invoke).mockImplementation(((what: string, args?: never) => {
    if (what in values) {
      const value = values[what];
      return value instanceof Error ? Promise.reject(value) : Promise.resolve(value);
    }
    return original?.(what, args);
  }) as never);
}

describe("reviewing a backup before importing", () => {
  it("only imports the reviewed snapshot after explicit confirmation", async () => {
    answers({ peek_backup: preview });
    const who = userEvent.setup();
    render(<Backup />);
    await who.click(screen.getByRole("button", { name: "选择文件…" }));
    expect(await screen.findByRole("region", { name: "确认导入" })).toBeInTheDocument();
    expect(vi.mocked(invoke).mock.calls.some(([name]) => name === "load_backup")).toBe(false);
    await who.click(screen.getByRole("button", { name: "按当前设置导入" }));
    expect(invoke).toHaveBeenCalledWith("load_backup", { token: "reviewed-snapshot" });
    expect(await screen.findByText("已导入 2 项 · 1 项已存在，未重复添加")).toBeInTheDocument();
  });

  it("cancels the prepared snapshot without importing", async () => {
    answers({ peek_backup: preview });
    const who = userEvent.setup();
    render(<Backup />);
    await who.click(screen.getByRole("button", { name: "选择文件…" }));
    await who.click(await screen.findByRole("button", { name: "取消" }));
    expect(invoke).toHaveBeenCalledWith("cancel_backup", { token: preview.token });
    expect(vi.mocked(invoke).mock.calls.some(([name]) => name === "load_backup")).toBe(false);
    expect(screen.queryByRole("region", { name: "确认导入" })).not.toBeInTheDocument();
  });

  it("explains retention, quota and missing contents before opening History", async () => {
    answers({ peek_backup: { ...preview, expires: 2, quota: 1024, partial: 1, refused: 1 } });
    const who = userEvent.setup();
    render(<Backup />);
    await who.click(screen.getByRole("button", { name: "选择文件…" }));
    expect(await screen.findByText(/2 项已超过当前保留时长/)).toBeInTheDocument();
    expect(screen.getByText(/自动清理可能删除现有或导入的未置顶记录/)).toBeInTheDocument();
    expect(screen.getByText(/1 项缺少部分内容/)).toBeInTheDocument();
    expect(screen.getByText(/1 项没有可恢复的内容/)).toBeInTheDocument();
    await who.click(screen.getByRole("button", { name: "先调整保留设置" }));
    expect(invoke).toHaveBeenCalledWith("cancel_backup", { token: preview.token });
    expect(invoke).toHaveBeenCalledWith("open_settings", { rail: "history" });
  });

  it("requires another confirmation when the checked cleanup risk increases", async () => {
    answers({
      peek_backup: preview,
      load_backup: { kind: "review", review: { ...preview, expires: 1 } },
    });
    const who = userEvent.setup();
    render(<Backup />);
    await who.click(screen.getByRole("button", { name: "选择文件…" }));
    await who.click(await screen.findByRole("button", { name: "按当前设置导入" }));
    expect(await screen.findByText(/保留设置或清理影响已变化/)).toBeInTheDocument();
    expect(screen.getByText(/1 项已超过当前保留时长/)).toBeInTheDocument();
    expect(vi.mocked(invoke).mock.calls.filter(([name]) => name === "load_backup")).toHaveLength(1);
    expect(screen.getByRole("button", { name: "按当前设置导入" })).toBeEnabled();
  });

  it("can retry a failed check and reports partial restoration separately", async () => {
    answers({ peek_backup: new Error("unreadable backup") });
    const who = userEvent.setup();
    render(<Backup />);
    await who.click(screen.getByRole("button", { name: "选择文件…" }));
    expect(await screen.findByText(/请重新选择备份后重试/)).toBeInTheDocument();
    answers({
      peek_backup: preview,
      load_backup: { kind: "done", result: { ...preview, partial: 1, refused: 1, repaired: 1 } },
    });
    await who.click(screen.getByRole("button", { name: "选择文件…" }));
    await who.click(await screen.findByRole("button", { name: "按当前设置导入" }));
    expect(
      await screen.findByText(/已导入 2 项.*缺少部分内容.*已修复 1 项的搜索索引/),
    ).toBeInTheDocument();
  });

  it("disposes a late preview when the page was left during inspection", async () => {
    let deliver: ((value: typeof preview) => void) | undefined;
    const pending = new Promise<typeof preview>((resolve) => {
      deliver = resolve;
    });
    answers({ peek_backup: pending });
    const who = userEvent.setup();
    const { unmount } = render(<Backup />);
    await who.click(screen.getByRole("button", { name: "选择文件…" }));
    unmount();
    deliver?.(preview);
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("cancel_backup", { token: preview.token }),
    );
  });

  it("does not describe failed imports as records that already existed", async () => {
    answers({
      peek_backup: preview,
      load_backup: {
        kind: "done",
        result: { ...preview, added: 0, already: 0, refused: 2 },
      },
    });
    const who = userEvent.setup();
    render(<Backup />);
    await who.click(screen.getByRole("button", { name: "选择文件…" }));
    await who.click(await screen.findByRole("button", { name: "按当前设置导入" }));
    expect(await screen.findByText("未新增记录 · 2 项无法导入")).toBeInTheDocument();
  });

  it("repairs existing search without requiring another backup", async () => {
    answers({ repair_search: 3 });
    const who = userEvent.setup();
    render(<Backup />);
    await who.click(screen.getByRole("button", { name: "检查并修复" }));
    expect(invoke).toHaveBeenCalledWith("repair_search");
    expect(await screen.findByText("已修复 3 项的搜索索引")).toBeInTheDocument();
  });
});
