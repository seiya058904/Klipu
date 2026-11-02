#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trouble {
    Unstarted,
    HistoryReplaced,
    HistoryUnopened,
    Undrawn,
    Unwatched,
    Unemptied,
    ClipboardDenied,
    Stopped,
    Unsteady,
    Unanswering,
}

pub const ALL: [Trouble; 10] = [
    Trouble::Unstarted,
    Trouble::HistoryReplaced,
    Trouble::HistoryUnopened,
    Trouble::Undrawn,
    Trouble::Unwatched,
    Trouble::Unemptied,
    Trouble::ClipboardDenied,
    Trouble::Stopped,
    Trouble::Unsteady,
    Trouble::Unanswering,
];

impl Trouble {
    pub fn still_keeping(self) -> bool {
        matches!(self, Self::HistoryReplaced | Self::Unemptied)
    }

    pub fn key(self) -> &'static str {
        match self {
            Trouble::Unstarted => "unstarted",
            Trouble::HistoryReplaced => "history-replaced",
            Trouble::HistoryUnopened => "history-unopened",
            Trouble::Undrawn => "undrawn",
            Trouble::Unwatched => "unwatched",
            Trouble::Unemptied => "unemptied",
            Trouble::ClipboardDenied => "clipboard-denied",
            Trouble::Stopped => "stopped",
            Trouble::Unsteady => "unsteady",
            Trouble::Unanswering => "unanswering",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        ALL.into_iter().find(|one| one.key() == key.trim())
    }

    pub fn worded(self, chinese: bool) -> &'static str {
        let [zh, en] = match self {
            Trouble::Unstarted => ["面板无法启动", "the panel could not start"],
            Trouble::HistoryReplaced => [
                "历史数据库已损坏，已新建一个",
                "the history was damaged and a new one was started",
            ],
            Trouble::HistoryUnopened => ["无法打开历史数据库", "the history could not be opened"],
            Trouble::Undrawn => ["无法绘制面板", "the panel could not be drawn"],
            Trouble::Unwatched => ["没有在监听剪贴板", "nothing is watching the clipboard"],
            Trouble::Unemptied => ["无法清空历史记录", "the history could not be emptied"],
            Trouble::ClipboardDenied => [
                "系统不允许 Klipu 读取剪贴板",
                "the system does not let Klipu read the clipboard",
            ],
            Trouble::Stopped => [
                "面板已停止监听剪贴板",
                "the panel stopped watching the clipboard",
            ],
            Trouble::Unsteady => [
                "面板无法保持运行，请重启 Klipu",
                "the panel will not stay up; restart Klipu",
            ],
            Trouble::Unanswering => ["面板没有响应", "the panel is not answering"],
        };
        if chinese { zh } else { en }
    }
}

#[cfg(test)]
#[path = "trouble_test.rs"]
mod tests;
