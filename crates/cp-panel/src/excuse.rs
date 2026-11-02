use cp_core::paste::Failure;

pub fn why_not(failure: Failure, english: bool) -> &'static str {
    match failure {
        Failure::NotForeground | Failure::ForegroundTimeout => crate::say::pick_in(
            english,
            "窗口没能回到前台，内容仍在剪贴板",
            "the window never came back to the front; still copied",
        ),
        Failure::NoKeyboardFocus => crate::say::pick_in(
            english,
            "那里没有可输入的位置，内容仍在剪贴板",
            "there was nowhere to type there; still copied",
        ),
        Failure::TargetGone => crate::say::pick_in(
            english,
            "目标窗口已关闭，内容仍在剪贴板",
            "that window is gone; still copied",
        ),
        Failure::SendDenied => crate::say::pick_in(english, denied_zh(), denied_en()),
        Failure::InputProtected => crate::say::pick_in(
            english,
            "系统正在保护输入内容，内容仍在剪贴板",
            "the system is protecting what is typed; still copied",
        ),
        Failure::TargetElevated => crate::say::pick_in(
            english,
            "目标窗口以管理员身份运行，内容仍在剪贴板",
            "that window runs as administrator; still copied",
        ),
    }
}

pub fn cannot_open() -> &'static str {
    crate::say::pick(
        "无法打开：它可能已不在原处",
        "could not open it: it may not be there any more",
    )
}

pub fn cannot_open_link() -> &'static str {
    crate::say::pick("无法打开链接", "that link could not be opened")
}

pub fn cannot_name() -> &'static str {
    crate::say::pick("无法保存名称", "that name could not be kept")
}

const fn denied_zh() -> &'static str {
    "系统阻止了模拟按键，内容仍在剪贴板"
}

const fn denied_en() -> &'static str {
    "the system refused the keystroke; still copied"
}

#[cfg(test)]
#[path = "excuse_test.rs"]
mod tests;
