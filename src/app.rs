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
    pub recipes: Signal<Vec<Recipe>>,
    /// 选中配方的输入列表
    pub recipe_inputs: Signal<Vec<RecipeInput>>,
    pub groups: Signal<Vec<InputGroup>>,
    /// 选中的配方；空 = 未选（全局批次视图）
    pub selected_recipe: Signal<String>,
    /// 选中的输入；空 = 快捷框为空白草稿
    pub selected_input: Signal<String>,
    pub runs: Signal<Vec<Run>>,
    pub theme: Signal<Theme>,
    pub show_settings: Signal<bool>,
    pub show_advanced: Signal<bool>,
    /// (run_id, image index)
    pub lightbox: Signal<Option<(String, usize)>>,
    /// 打开快照弹窗的批次 id
    pub snapshot_run: Signal<Option<String>>,
    /// 待载入模板编辑区草稿的快照（来自「恢复模板为此快照」，保存后才成为新版本）
    pub restore_template: Signal<Option<Recipe>>,
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
    pub fn recipes(&self) -> Vec<Recipe> {
        self.recipes.cloned()
    }
    pub fn recipe_inputs(&self) -> Vec<RecipeInput> {
        self.recipe_inputs.cloned()
    }
    pub fn groups(&self) -> Vec<InputGroup> {
        self.groups.cloned()
    }
    pub fn selected_recipe_id(&self) -> String {
        self.selected_recipe.cloned()
    }
    pub fn selected_input_id(&self) -> String {
        self.selected_input.cloned()
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

    pub fn selected_recipe(&self) -> Option<Recipe> {
        let id = self.selected_recipe_id();
        self.recipes().into_iter().find(|r| r.id == id)
    }

    pub fn selected_input(&self) -> Option<RecipeInput> {
        let id = self.selected_input_id();
        self.recipe_inputs().into_iter().find(|i| i.id == id)
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

    /// 修改配方字段并同步服务端（标题等非内容性字段；不触发版本 +1）
    pub fn patch_recipe(mut self, id: &str, f: impl FnOnce(&mut Recipe)) {
        self.recipes.with_mut(|list| {
            if let Some(x) = list.iter_mut().find(|x| x.id == id) {
                f(x);
            }
        });
        let snapshot = self.recipes().into_iter().find(|r| r.id == id);
        if let Some(recipe) = snapshot {
            spawn(async move {
                let _ = update_recipe(recipe).await;
            });
        }
    }

    /// 用服务端返回的配方整体替换本地副本
    pub fn patch_replace_recipe(mut self, recipe: Recipe) {
        self.recipes.with_mut(|v| {
            if let Some(x) = v.iter_mut().find(|x| x.id == recipe.id) {
                *x = recipe;
            }
        });
    }

    /// 修改选中配方的输入并同步服务端
    pub fn mutate_input(self, id: &str, f: impl FnOnce(&mut RecipeInput)) {
        let mut inputs_sig = self.recipe_inputs;
        inputs_sig.with_mut(|list| {
            if let Some(x) = list.iter_mut().find(|x| x.id == id) {
                f(x);
            }
        });
        let snapshot = self.recipe_inputs().into_iter().find(|i| i.id == id);
        if let Some(input) = snapshot {
            spawn(async move {
                let _ = update_input(input).await;
            });
        }
    }

    /// 选中配方（并清空输入选择）
    pub fn select_recipe(mut self, id: &str) {
        self.selected_recipe.set(id.to_string());
        self.selected_input.set(String::new());
    }

    /// 选中输入（连带选中其配方）
    pub fn select_input(mut self, recipe_id: &str, input_id: &str) {
        self.selected_recipe.set(recipe_id.to_string());
        self.selected_input.set(input_id.to_string());
    }
}

#[component]
pub fn App() -> Element {
    let state = AppState {
        config: use_signal(|| None),
        profiles: use_signal(Vec::new),
        recipes: use_signal(Vec::new),
        recipe_inputs: use_signal(Vec::new),
        groups: use_signal(Vec::new),
        selected_recipe: use_signal(String::new),
        selected_input: use_signal(String::new),
        runs: use_signal(Vec::new),
        theme: use_signal(theme::stored),
        show_settings: use_signal(|| false),
        show_advanced: use_signal(|| false),
        lightbox: use_signal(|| None),
        snapshot_run: use_signal(|| None),
        restore_template: use_signal(|| None),
        toasts: use_signal(Vec::new),
    };
    let mut config = state.config;
    let mut profiles = state.profiles;
    let mut recipes = state.recipes;
    let mut groups = state.groups;
    let selected_recipe = state.selected_recipe;
    let mut runs = state.runs;

    use_hook(move || {
        theme::apply(state.theme());
    });

    // ---- 首次加载：配置 / 配方 / 分组 / 批次 ----
    use_effect(move || {
        spawn(async move {
            if let Ok(cfg) = get_config().await {
                config.set(Some(cfg));
            }
            if let Ok(rs) = list_recipes().await {
                recipes.set(rs);
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

    // ---- 选中配方变化 → 加载其输入列表 ----
    let state_for_inputs = state;
    use_effect(move || {
        let rid = selected_recipe.cloned();
        let mut inputs_sig = state_for_inputs.recipe_inputs;
        spawn(async move {
            if rid.is_empty() {
                return;
            }
            if let Ok(list) = list_inputs(rid).await {
                inputs_sig.set(list);
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

                if let Some(recipe) = state.selected_recipe() {
                    crate::ui::recipe::RecipePage { key: "{recipe.id}", state, recipe }
                } else if config().is_some() {
                    div {
                        class: "stage",
                        crate::ui::feed::Feed { state }
                        div { class: "stage-hint",
                            "← 从左侧选择或新建一个配方开始 · 配方下可创建多个输入"
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
