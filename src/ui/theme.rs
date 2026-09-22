//! 主题：light / dark，localStorage 记忆，未设置时跟随系统（CSS media）。

use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    System,
    Light,
    Dark,
}

#[allow(dead_code)] // 仅 wasm 客户端使用
const LS_KEY: &str = "atelier-theme";

pub fn stored() -> Theme {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(win) = web_sys::window() {
            if let Ok(Some(ls)) = win.local_storage() {
                if let Ok(Some(v)) = ls.get_item(LS_KEY) {
                    return match v.as_str() {
                        "light" => Theme::Light,
                        "dark" => Theme::Dark,
                        _ => Theme::System,
                    };
                }
            }
        }
    }
    Theme::System
}

pub fn apply(theme: Theme) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(win) = web_sys::window() {
            if let Some(doc) = win.document() {
                if let Some(el) = doc.document_element() {
                    let val = match theme {
                        Theme::Light => Some("light"),
                        Theme::Dark => Some("dark"),
                        Theme::System => None,
                    };
                    match val {
                        Some(v) => {
                            let _ = el.set_attribute("data-theme", v);
                        }
                        None => {
                            let _ = el.remove_attribute("data-theme");
                        }
                    }
                }
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = theme;
    }
}

pub fn persist(theme: Theme) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(win) = web_sys::window() {
            if let Ok(Some(ls)) = win.local_storage() {
                let v = match theme {
                    Theme::Light => "light",
                    Theme::Dark => "dark",
                    Theme::System => "system",
                };
                let _ = ls.set_item(LS_KEY, v);
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = theme;
    }
}

/// 顶栏的主题切换按钮：light ↔ dark。
/// 图标显隐由 CSS 依「生效主题」决定，保证 SSR 与客户端首帧一致（水合安全）。
#[component]
pub fn ThemeToggle(theme: Signal<crate::ui::theme::Theme>) -> Element {
    let toggle = move |_| {
        let next = if effective_dark() {
            Theme::Light
        } else {
            Theme::Dark
        };
        theme.set(next);
        apply(next);
        persist(next);
    };
    rsx! {
        button {
            class: "icon-btn theme-toggle",
            title: "切换亮色 / 暗色模式",
            "aria-label": "切换主题",
            onclick: toggle,
            span { class: "theme-icon-sun", crate::ui::icons::IconSun { size: 16 } }
            span { class: "theme-icon-moon", crate::ui::icons::IconMoon { size: 16 } }
        }
    }
}

/// 当前「生效」是否为暗色（显式设置优先，否则跟随系统；SSR 上恒 false）
fn effective_dark() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(win) = web_sys::window() {
            if let Ok(Some(mq)) = win.match_media("(prefers-color-scheme: dark)") {
                return mq.matches();
            }
        }
    }
    false
}
