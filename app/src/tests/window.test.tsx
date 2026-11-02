import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "../App";
import { adopt } from "../locales";
import { forPanel } from "./setup";

describe("the window", () => {
  beforeEach(() => {
    adopt("zh");
    document.documentElement.removeAttribute("data-theme");
  });

  it("opens on General, which is where the things touched once live", async () => {
    render(<App />);
    expect(await screen.findByRole("heading", { level: 1 })).toHaveTextContent("通用");
    expect(await screen.findByLabelText("界面语言")).toBeDefined();
  });

  it("the shortcut and what the panel answers live in their own section", async () => {
    const who = userEvent.setup();
    render(<App />);
    await who.click(screen.getByRole("button", { name: "快捷键" }));
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("快捷键");
    expect(await screen.findByText("Ctrl + Alt + K")).toBeDefined();
    expect(screen.getByText("双击")).toBeDefined();
  });

  it("lists every shortcut that works anywhere, the usual paste only as a reminder", async () => {
    const who = userEvent.setup();
    render(<App />);
    await who.click(screen.getByRole("button", { name: "快捷键" }));
    expect(await screen.findByText("原样粘贴")).toBeDefined();
    expect(screen.getByText("Ctrl + V")).toBeDefined();
    expect(screen.getAllByText("粘贴为纯文本").length).toBeGreaterThan(0);
    expect(await screen.findByText("Ctrl + Alt + Shift + K")).toBeDefined();
    expect(screen.queryByText("Ctrl + Alt + P")).toBeNull();
    expect(screen.getAllByRole("button", { name: "修改" })).toHaveLength(2);
    expect(screen.getAllByRole("button", { name: "移除" })).toHaveLength(1);
  });

  it("a paste shortcut can be left without a combination", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    const who = userEvent.setup();
    render(<App />);
    await who.click(screen.getByRole("button", { name: "快捷键" }));
    const [remove] = await screen.findAllByRole("button", { name: "移除" });
    await who.click(remove);
    const kept = vi
      .mocked(invoke)
      .mock.calls.filter(([what]) => what === "keep")
      .pop();
    const said = kept?.[1] as { config: Record<string, string> } | undefined;
    expect(said?.config["plain-shortcut"]).toBe("");
    expect(said?.config.shortcut).toBe("Ctrl+Alt+K");
  });

  it("refuses the combination every app pastes with", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    const who = userEvent.setup();
    render(<App />);
    await who.click(screen.getByRole("button", { name: "快捷键" }));
    const changes = await screen.findAllByRole("button", { name: "修改" });
    const before = vi.mocked(invoke).mock.calls.filter(([what]) => what === "keep").length;
    await who.click(changes[1]);
    const stop = await screen.findByRole("button", { name: "保持不变" });
    stop.focus();
    await who.keyboard("{Control>}[KeyV]{/Control}");
    expect(await screen.findByText(/通用的复制/)).toBeDefined();
    const after = vi.mocked(invoke).mock.calls.filter(([what]) => what === "keep").length;
    expect(after).toBe(before);
  });

  it("refuses a combination another Klipu shortcut already answers to", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    const who = userEvent.setup();
    render(<App />);
    await who.click(screen.getByRole("button", { name: "快捷键" }));
    const changes = await screen.findAllByRole("button", { name: "修改" });
    const before = vi.mocked(invoke).mock.calls.filter(([what]) => what === "keep").length;
    await who.click(changes[0]);
    const stop = await screen.findByRole("button", { name: "保持不变" });
    stop.focus();
    await who.keyboard("{Control>}{Alt>}{Shift>}[KeyK]{/Shift}{/Alt}{/Control}");
    expect(await screen.findByText(/已被 Klipu 的另一个快捷键使用/)).toBeDefined();
    const after = vi.mocked(invoke).mock.calls.filter(([what]) => what === "keep").length;
    expect(after).toBe(before);
  });

  it("offers five sections and About, and nothing else", () => {
    render(<App />);
    const rail = screen.getByRole("navigation", { name: "分区" });
    const says = buttonsIn(rail).map((one) => one.textContent);
    expect(says).toEqual(["通用", "外观", "快捷键", "历史记录", "备份", "关于"]);
  });

  it("switches section when one is chosen", async () => {
    const who = userEvent.setup();
    render(<App />);
    await who.click(screen.getByRole("button", { name: "历史记录" }));
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("历史记录");
    expect(await screen.findByLabelText("保留时长")).toHaveValue("30");
  });

  it("the chosen theme is written on the root, which is what paints it", async () => {
    const who = userEvent.setup();
    render(<App />);
    await who.click(await screen.findByRole("button", { name: "外观" }));
    await who.selectOptions(await screen.findByLabelText("主题"), "light");
    expect(document.documentElement.getAttribute("data-theme")).toBe("light");
    await who.selectOptions(screen.getByLabelText("主题"), "system");
    expect(document.documentElement.hasAttribute("data-theme")).toBe(false);
  });

  it("what changes is sent to be saved, it does not stay in the window", async () => {
    const who = userEvent.setup();
    const { invoke } = await import("@tauri-apps/api/core");
    render(<App />);
    await who.click(await screen.findByRole("button", { name: "外观" }));
    await who.selectOptions(await screen.findByLabelText("主题"), "dark");
    expect(invoke).toHaveBeenCalledWith("keep", {
      config: expect.objectContaining({ theme: "dark" }),
    });
  });

  it("its own title bar really minimizes and closes, it does not just draw them", async () => {
    const who = userEvent.setup();
    const { theWindow } = await import("./setup");
    render(<App />);
    await who.click(screen.getByRole("button", { name: "最小化" }));
    expect(theWindow.minimize).toHaveBeenCalled();
    await who.click(screen.getByRole("button", { name: "关闭" }));
    expect(theWindow.close).toHaveBeenCalled();
  });

  it("choosing «Forever» is saved as zero, which is what the file understands", async () => {
    const who = userEvent.setup();
    const { invoke } = await import("@tauri-apps/api/core");
    render(<App />);
    await who.click(screen.getByRole("button", { name: "历史记录" }));
    await who.selectOptions(await screen.findByLabelText("保留时长"), "0");
    expect(invoke).toHaveBeenCalledWith("keep", {
      config: expect.objectContaining({ "keeps-days": 0 }),
    });
    expect(screen.queryByText(/invalid type/)).toBeNull();
  });

  it("starting with the session is decided by the system, not by the file", async () => {
    const who = userEvent.setup();
    const { invoke } = await import("@tauri-apps/api/core");
    render(<App />);
    const knob = await screen.findByLabelText("开机自启");
    expect(knob).toHaveAttribute("aria-checked", "false");
    await who.click(knob);
    expect(invoke).toHaveBeenCalledWith("wake", { wanted: true });
    expect(await screen.findByLabelText("开机自启")).toHaveAttribute("aria-checked", "true");
  });

  it("warns when the panel is not working, instead of hiding it", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    const real = vi.mocked(invoke).getMockImplementation();
    vi.mocked(invoke).mockImplementation(((what: string, args?: never) =>
      what === "trouble" ? Promise.resolve("history-unopened") : real?.(what, args)) as never);
    render(<App />);
    expect(await screen.findByText(/无法打开历史数据库/)).toBeDefined();
    expect(screen.getByText(/不会被保存/)).toBeDefined();
    vi.mocked(invoke).mockImplementation(real as never);
  });

  it("does not invent panel problems when all is well", async () => {
    render(<App />);
    await screen.findByRole("button", { name: "通用" });
    expect(screen.queryByText(/不会被保存/)).toBeNull();
  });

  it("says the shortcut does not answer when another program has taken it", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    const real = vi.mocked(invoke).getMockImplementation();
    vi.mocked(invoke).mockImplementation(((what: string, args?: never) =>
      what === "keys" && forPanel(args)
        ? Promise.resolve({ wanted: "Ctrl+Alt+K", bound: false })
        : real?.(what, args)) as never);
    const who = userEvent.setup();
    render(<App />);
    await who.click(screen.getByRole("button", { name: "快捷键" }));
    expect(await screen.findByText(/已被其他程序占用/)).toBeDefined();
    vi.mocked(invoke).mockImplementation(real as never);
  });

  it("offers the free combinations and adopts the one pressed", async () => {
    const who = userEvent.setup();
    const { invoke } = await import("@tauri-apps/api/core");
    const real = vi.mocked(invoke).getMockImplementation();
    vi.mocked(invoke).mockImplementation(((what: string, args?: never) =>
      what === "keys" && forPanel(args)
        ? Promise.resolve({ wanted: "Ctrl+Alt+K", bound: false })
        : real?.(what, args)) as never);
    render(<App />);
    await who.click(screen.getByRole("button", { name: "快捷键" }));
    expect(await screen.findByText(/当前可用/)).toBeDefined();
    const offered = await screen.findByRole("button", { name: "Ctrl + Alt + Shift + V" });
    await who.click(offered);
    const asked = vi
      .mocked(invoke)
      .mock.calls.filter(([what]) => what === "keep")
      .pop();
    expect(asked).toBeDefined();
    const sent = asked?.[1] as { config: { shortcut: string } } | undefined;
    expect(sent?.config.shortcut).toBe("Ctrl+Alt+Shift+V");
    vi.mocked(invoke).mockImplementation(real as never);
  });

  it("says nothing about the shortcut when the system did give it up", async () => {
    const who = userEvent.setup();
    render(<App />);
    await who.click(screen.getByRole("button", { name: "快捷键" }));
    expect(await screen.findByText("Ctrl + Alt + K")).toBeDefined();
    expect(screen.queryByText(/Otro programa ya usa/)).toBeNull();
  });

  it("an image cap that is not in the list is shown as it is", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    const real = vi.mocked(invoke).getMockImplementation();
    vi.mocked(invoke).mockImplementation(((what: string, args?: never) =>
      what === "settings"
        ? Promise.resolve({
            locale: "zh",
            theme: "system",
            shortcut: "Ctrl+Alt+K",
            "hides-when-left": true,
            "keeps-days": 30,
            "images-quota-mb": 700,
          })
        : real?.(what, args)) as never);
    render(<App />);
    await userEvent.click(await screen.findByRole("button", { name: "历史记录" }));
    const picked = (await screen.findByLabelText("空间上限")) as HTMLSelectElement;
    expect(picked.value).toBe("700");
    expect(screen.getByRole("option", { name: "700 MB" })).toBeDefined();
    vi.mocked(invoke).mockImplementation(real as never);
  });

  it("exporting writes the chosen file and says how much it kept", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    const { save } = await import("@tauri-apps/plugin-dialog");
    render(<App />);
    await userEvent.click(await screen.findByRole("button", { name: "备份" }));
    await userEvent.click(await screen.findByRole("button", { name: "导出…" }));
    expect(save).toHaveBeenCalled();
    expect(invoke).toHaveBeenCalledWith(
      "save_backup",
      expect.objectContaining({ path: "/donde/quiera/Klipu.klipubak" }),
    );
    expect(await screen.findByText("已保存：3 项")).toBeDefined();
  });

  it("importing brings what is missing and does not duplicate what was there", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    render(<App />);
    await userEvent.click(await screen.findByRole("button", { name: "备份" }));
    await userEvent.click(await screen.findByRole("button", { name: "选择文件…" }));
    expect(invoke).toHaveBeenCalledWith(
      "load_backup",
      expect.objectContaining({ path: "/donde/quiera/Klipu.klipubak" }),
    );
    expect(await screen.findByText("已导入 2 项 · 1 项已存在，未重复添加")).toBeDefined();
  });

  it("changing the shortcut saves the combination pressed", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "快捷键" }));
    const [change] = await screen.findAllByRole("button", { name: "修改" });
    await userEvent.click(change);
    const stop = await screen.findByRole("button", { name: "保持不变" });
    stop.focus();
    await userEvent.keyboard("{Control>}{Alt>}[F9]{/Alt}{/Control}");
    const kept = vi
      .mocked(invoke)
      .mock.calls.filter(([what]) => what === "keep")
      .pop();
    const said = kept?.[1] as { config: { shortcut: string } } | undefined;
    expect(said?.config.shortcut).toBe("Ctrl+Alt+F9");
  });

  it("emptying the history asks for confirmation before doing it", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    render(<App />);
    await userEvent.click(await screen.findByRole("button", { name: "历史记录" }));
    const wipe = await screen.findByRole("button", { name: "清空" });
    await userEvent.click(wipe);
    expect(vi.mocked(invoke).mock.calls.some(([what]) => what === "empty")).toBe(false);
    await userEvent.click(await screen.findByRole("button", { name: "确定吗？" }));
    expect(vi.mocked(invoke).mock.calls.some(([what]) => what === "empty")).toBe(true);
  });

  it("the version comes from the program itself, not from text written by hand", async () => {
    const who = userEvent.setup();
    render(<App />);
    await who.click(screen.getByRole("button", { name: "关于" }));
    expect(await screen.findByText("3.0.0")).toBeDefined();
  });

  it("a link that cannot be opened is said, not swallowed", async () => {
    const who = userEvent.setup();
    const { invoke } = await import("@tauri-apps/api/core");
    const real = vi.mocked(invoke).getMockImplementation();
    vi.mocked(invoke).mockImplementation((what: string, args?: unknown) => {
      if (what === "open_web") return Promise.reject(new Error("forbidden"));
      return (real as (a: string, b?: unknown) => Promise<unknown>)(what, args);
    });
    try {
      render(<App />);
      await who.click(screen.getByRole("button", { name: "关于" }));
      await who.click(await screen.findByRole("button", { name: "源代码" }));
      expect(await screen.findByText(/无法打开/)).toBeDefined();
    } finally {
      vi.mocked(invoke).mockImplementation(real as never);
    }
  });

  it("choosing English changes the whole window, not just the language row", async () => {
    const who = userEvent.setup();
    const { invoke } = await import("@tauri-apps/api/core");
    render(<App />);
    await who.selectOptions(await screen.findByLabelText("界面语言"), "en");
    expect(await screen.findByRole("button", { name: "History" })).toBeDefined();
    await who.click(screen.getByRole("button", { name: "Appearance" }));
    expect(screen.getByLabelText("Theme")).toBeDefined();
    await who.click(screen.getByRole("button", { name: "About" }));
    expect(screen.getByText("All local")).toBeDefined();
    expect(invoke).toHaveBeenCalledWith("relabel", { locale: "en" });
  });

  it("About says what it is and that nothing leaves this machine", async () => {
    const who = userEvent.setup();
    render(<App />);
    await who.click(screen.getByRole("button", { name: "关于" }));
    expect(screen.getByText(/无遥测/)).toBeDefined();
    expect(screen.getByText("完全本地")).toBeDefined();
    expect(screen.getByText("不联网")).toBeDefined();
  });
});

function buttonsIn(rail: HTMLElement) {
  return Array.from(rail.querySelectorAll("button"));
}
