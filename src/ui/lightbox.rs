//! 灯箱：批次图片放大浏览，←/→ 切换，ESC 关闭。

use crate::app::AppState;
use crate::model::Run;
use dioxus::prelude::*;

#[component]
pub fn Lightbox(state: AppState, run_id: String, index: usize) -> Element {
    let run: Option<Run> = state
        .runs()
        .into_iter()
        .find(|r| r.id == run_id);
    let Some(run) = run else {
        return rsx! {};
    };
    let images = run.images.clone();
    let Some(asset) = images.get(index).cloned() else {
        return rsx! {};
    };

    let mut lightbox = state.lightbox;
    let close = move |_| lightbox.set(None);
    let rid_kb = run_id.clone();
    let rid_prev = run_id.clone();
    let rid_next = run_id.clone();
    let dl_name = format!("atelier-{run_id}-{index}.{}", asset.ext);

    rsx! {
        div {
            class: "lightbox",
            tabindex: "0",
            autofocus: true,
            onclick: close.clone(),
            onkeydown: move |e| {
                use keyboard_types::Key;
                match e.key() {
                    Key::Escape => lightbox.set(None),
                    Key::ArrowLeft if index > 0 => {
                        lightbox.set(Some((rid_kb.clone(), index - 1)));
                    }
                    Key::ArrowRight if index + 1 < images.len() => {
                        lightbox.set(Some((rid_kb.clone(), index + 1)));
                    }
                    _ => {}
                }
            },
            div { class: "lightbox-inner",
                onclick: move |e| e.stop_propagation(),
                header { class: "lightbox-head",
                    span { class: "mono", "{run.model_id}" }
                    span { class: "lightbox-count", "{index + 1} / {images.len()}" }
                    div { class: "lightbox-actions",
                        if index > 0 {
                            button {
                                class: "icon-btn",
                                title: "上一张",
                                onclick: move |_| lightbox.set(Some((rid_prev.clone(), index - 1))),
                                crate::ui::icons::IconChevronLeft { size: 16 }
                            }
                        }
                        if index + 1 < images.len() {
                            button {
                                class: "icon-btn",
                                title: "下一张",
                                onclick: move |_| lightbox.set(Some((rid_next.clone(), index + 1))),
                                crate::ui::icons::IconChevronRight { size: 16 }
                            }
                        }
                        a {
                            class: "ghost-btn small",
                            href: "{asset.url()}",
                            download: "{dl_name}",
                            crate::ui::icons::IconDownload { size: 13 }
                            "下载"
                        }
                        button {
                            class: "icon-btn",
                            title: "关闭",
                            onclick: close,
                            crate::ui::icons::IconX { size: 15 }
                        }
                    }
                }
                div { class: "lightbox-stage",
                    img { src: "{asset.url()}" }
                }
                if !run.prompt.is_empty() {
                    footer { class: "lightbox-prompt", "{run.prompt}" }
                }
            }
        }
    }
}
