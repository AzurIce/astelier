//! 顶栏：品牌 / Provider 切换 / 主题切换 / 设置入口。

use crate::app::AppState;
use crate::ui::widgets::{Dropdown, DropdownItem};
use dioxus::prelude::*;

#[component]
pub fn Topbar(state: AppState) -> Element {
    let mut config = state.config;

    let provider_items: Vec<DropdownItem> = config()
        .map(|cfg| {
            cfg.providers
                .iter()
                .map(|p| DropdownItem {
                    value: p.id.clone(),
                    label: p.name.clone(),
                    caption: Some(p.base_url.clone()),
                    badges: vec![],
                })
                .collect()
        })
        .unwrap_or_default();
    let active_name = config()
        .and_then(|cfg| cfg.active().map(|p| p.name.clone()))
        .unwrap_or_else(|| "未配置".into());

    rsx! {
        header { class: "topbar",
            div { class: "brand",
                span { class: "brand-mark", crate::ui::icons::IconSparkles { size: 15 } }
                span { class: "brand-name", "Atelier" }
                span { class: "brand-sub", "生成式艺术工作台" }
            }

            div { class: "topbar-right",
                Dropdown {
                    label: active_name,
                    items: provider_items,
                    value: config().map(|c| c.active_provider.clone()).unwrap_or_default(),
                    disabled: false,
                    drop_up: false,
                    onpick: move |id: String| {
                        // 切换工作 provider：此后新节点默认用它
                        if let Some(cfg) = config() {
                            let mut next = cfg.clone();
                            next.active_provider = id;
                            config.set(Some(next));
                        }
                    },
                }
                crate::ui::theme::ThemeToggle { theme: state.theme }
                button {
                    class: "icon-btn",
                    title: "设置",
                    "aria-label": "设置",
                    onclick: move |_| state.show_settings.set(true),
                    crate::ui::icons::IconSliders { size: 16 }
                }
            }
        }
    }
}
