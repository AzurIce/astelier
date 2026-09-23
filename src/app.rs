//! 应用根组件与全局状态。

use crate::api::*;
use crate::model::*;
use crate::ui::theme::{self, Theme};
use crate::ui::widgets::{Toast, Toasts};
use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub struct AppState {
    pub config: Signal<Option<Config>>,
    /// 当前 provider 的模型档案
    pub profiles: Signal<Vec<ModelProfile>>,
    pub graphs: Signal<Vec<Graph>>,
    /// 节点图分组（文件夹）
    pub groups: Signal<Vec<GraphGroup>>,
    /// 选中的节点图；空 = 全局批次视图
    pub selected_graph: Signal<String>,
    pub runs: Signal<Vec<Run>>,
    pub theme: Signal<Theme>,
    pub show_settings: Signal<bool>,
    pub show_advanced: Signal<bool>,
    /// (run_id, image index)
    pub lightbox: Signal<Option<(String, usize)>>,
    /// 打开快照弹窗的批次 id
    pub snapshot_run: Signal<Option<String>>,
    pub toasts: Signal<Vec<Toast>>,
}

impl AppState {
    // ---- 只读访问器 ----
    pub fn config(&self) -> Option<Config> {
        self.config.cloned()
    }
    pub fn profiles(&self) -> Vec<ModelProfile> {
        self.profiles.cloned()
    }
    pub fn graphs(&self) -> Vec<Graph> {
        self.graphs.cloned()
    }
    pub fn groups(&self) -> Vec<GraphGroup> {
        self.groups.cloned()
    }
    pub fn selected_graph_id(&self) -> String {
        self.selected_graph.cloned()
    }
    pub fn runs(&self) -> Vec<Run> {
        self.runs.cloned()
    }
    pub fn theme(&self) -> Theme {
        self.theme.cloned()
    }
    pub fn show_settings(&self) -> bool {
        self.show_settings.cloned()
    }
    pub fn lightbox(&self) -> Option<(String, usize)> {
        self.lightbox.cloned()
    }
    pub fn snapshot_run_id(&self) -> Option<String> {
        self.snapshot_run.cloned()
    }
    pub fn toasts(&self) -> Vec<Toast> {
        self.toasts.cloned()
    }

    pub fn selected_graph(&self) -> Option<Graph> {
        let id = self.selected_graph_id();
        self.graphs().into_iter().find(|g| g.id == id)
    }

    pub fn profile_of(&self, model_id: &str) -> Option<ModelProfile> {
        self.profiles().into_iter().find(|p| p.id == model_id)
    }

    pub fn toast(&self, text: impl Into<String>, kind: &'static str) {
        let id = crate::util::now_ms() ^ ((self.toasts().len() as u64) << 8);
        let mut toasts_sig = self.toasts;
        toasts_sig.write().push(Toast {
            id,
            text: text.into(),
            kind,
        });
        let mut toasts = self.toasts;
        spawn(async move {
            crate::util::delay(4200).await;
            toasts.write().retain(|t| t.id != id);
        });
    }

    /// 本地替换一张图（服务端返回后同步）
    pub fn patch_replace_graph(mut self, graph: Graph) {
        self.graphs.with_mut(|v| {
            if let Some(x) = v.iter_mut().find(|x| x.id == graph.id) {
                *x = graph;
            }
        });
    }

    /// 修改一张图并同步服务端（整图落盘；节点内容/连线/位置/标题共用此路径）
    pub fn patch_graph(self, graph: Graph, f: impl FnOnce(&mut Graph)) {
        let mut graph = graph;
        f(&mut graph);
        self.patch_replace_graph(graph.clone());
        spawn(async move {
            if let Ok(saved) = update_graph(graph).await {
                let state = self;
                state.patch_replace_graph(saved);
            }
        });
    }

    /// 选中节点图
    pub fn select_graph(mut self, id: &str) {
        self.selected_graph.set(id.to_string());
    }
}

#[component]
pub fn App() -> Element {
    let state = AppState {
        config: use_signal(|| None),
        profiles: use_signal(Vec::new),
        graphs: use_signal(Vec::new),
        groups: use_signal(Vec::new),
        selected_graph: use_signal(String::new),
        runs: use_signal(Vec::new),
        theme: use_signal(theme::stored),
        show_settings: use_signal(|| false),
        show_advanced: use_signal(|| false),
        lightbox: use_signal(|| None),
        snapshot_run: use_signal(|| None),
        toasts: use_signal(Vec::new),
    };
    let mut config = state.config;
    let mut profiles = state.profiles;
    let mut graphs = state.graphs;
    let mut groups = state.groups;
    let mut runs = state.runs;

    use_hook(move || {
        theme::apply(state.theme());
    });

    // ---- 首次加载：配置 / 节点图 / 分组 / 批次 ----
    use_effect(move || {
        spawn(async move {
            if let Ok(cfg) = get_config().await {
                config.set(Some(cfg));
            }
            if let Ok(gs) = list_graphs().await {
                graphs.set(gs);
            }
            if let Ok(gs) = list_groups().await {
                groups.set(gs);
            }
            if let Ok(r) = list_runs(200).await {
                runs.set(r);
            }
        });
    });

    // ---- 模型档案跟随 active provider ----
    use_effect(move || {
        let cfg = config();
        spawn(async move {
            if let Some(cfg) = cfg {
                if let Ok(list) = resolve_profiles(cfg.active_provider).await {
                    profiles.set(list);
                }
            }
        });
    });

    // ---- 批次轮询：存在 Running 时每 1.2s 拉一次 ----
    use_effect(move || {
        let any_running = runs.cloned().iter().any(|r| r.status == RunStatus::Running);
        if any_running {
            spawn(async move {
                crate::util::delay(1200).await;
                if let Ok(list) = list_runs(200).await {
                    runs.set(list);
                }
            });
        }
    });

    rsx! {
        Stylesheet { href: asset!("/assets/main.css") }

        div {
            class: "app",
            ondragover: move |e| e.prevent_default(),

            crate::ui::topbar::Topbar { state }

            div {
                class: "main",
                crate::ui::sidebar::Sidebar { state }

                if let Some(graph) = state.selected_graph() {
                    crate::ui::graph::GraphPage { key: "{graph.id}", state, graph }
                } else if config().is_some() {
                    div {
                        class: "stage",
                        crate::ui::feed::Feed { state }
                        div { class: "stage-hint",
                            "← 从左侧选择或新建一个节点图开始 · 生图节点的输出连线到显示节点"
                        }
                    }
                } else {
                    div { class: "stage boot",
                        span { class: "boot-spin", crate::ui::icons::IconLoader { size: 20 } }
                        "正在准备工作台…"
                    }
                }
            }

            if state.show_settings() {
                crate::ui::settings::SettingsModal { state }
            }
            if let Some((run_id, idx)) = state.lightbox() {
                crate::ui::lightbox::Lightbox { state, run_id, index: idx }
            }
            if let Some(run_id) = state.snapshot_run_id() {
                crate::ui::snapshot::SnapshotModal { state, run_id }
            }

            Toasts { toasts: state.toasts }
        }
    }
}
