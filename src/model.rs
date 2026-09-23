//! 统一数据模型 —— 客户端 / 服务端共享。
//!
//! 核心思想：模型能力以 `ModelProfile`（含 params schema）元数据描述，
//! UI 表单与输入持久化都使用统一的 `ParamKey → ParamValue` 表示；
//! 发送请求时由 adapter 按 `api` 种类转换成各协议的具体字段。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 统一参数值。Unset = 该参数不随请求发送。
/// 注意必须用相邻标签（tag+content）：serde 的内部标签不支持含 String/数字的新类型变体。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "t", content = "v", rename_all = "snake_case")]
pub enum ParamValue {
    Unset,
    Text(String),
    Number(f64),
    /// 尺寸：预设档（如 "1024x1024"、"auto"）或自定义 "WxH"
    Size(String),
}

// 客户端仅持久化/展示该值；转换方法只在服务端 adapter 路径使用
#[allow(dead_code)]
impl ParamValue {
    pub fn is_unset(&self) -> bool {
        matches!(self, ParamValue::Unset)
    }

    /// 转成请求体里的 JSON 值（Unset → None，由调用方跳过）
    pub fn to_request_json(&self) -> Option<serde_json::Value> {
        match self {
            ParamValue::Unset => None,
            ParamValue::Text(s) => Some(serde_json::Value::String(s.clone())),
            ParamValue::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 9e15 {
                    Some(serde_json::Value::Number((*n as i64).into()))
                } else {
                    serde_json::Number::from_f64(*n).map(serde_json::Value::Number)
                }
            }
            ParamValue::Size(s) => Some(serde_json::Value::String(s.clone())),
        }
    }
}

pub type ParamMap = BTreeMap<String, ParamValue>;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Gen,
    Edit,
}

impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Mode::Gen => "文生图",
            Mode::Edit => "编辑",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ParamKind {
    Select,
    Number,
    Text,
    Size,
}

/// 参数描述符：统一键 + API 字段名 + 控件类型。
/// 接入新模型 = 配一份 params 元数据，不写代码。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ParamDef {
    /// 统一键（如 "quality"），ParamMap 与 UI 用它寻址
    pub key: String,
    /// 发送到请求体的字段名；空串 = 同 key
    #[serde(default)]
    pub api_key: String,
    pub label: String,
    pub kind: ParamKind,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    /// 归入「更多参数」折叠区
    #[serde(default)]
    pub advanced: bool,
    /// 语义分组："" = 生成主参数，"output" = 输出，"safety" = 审核与标识
    #[serde(default)]
    pub group: String,
    /// 限定出现的模式；空 = gen + edit 都出现
    #[serde(default)]
    pub modes: Vec<Mode>,
}

impl ParamDef {
    pub fn api_key(&self) -> &str {
        if self.api_key.is_empty() {
            &self.key
        } else {
            &self.api_key
        }
    }

    pub fn visible_in(&self, mode: Mode) -> bool {
        self.modes.is_empty() || self.modes.contains(&mode)
    }
}

/// 自定义尺寸约束规则（gpt-image-2+：边长被 16 整除、比例 1:3–3:1、上限 3840×2160）
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SizeRule {
    pub step: u32,
    pub min_side: u32,
    pub max_w: u32,
    pub max_h: u32,
    pub ratio_min: f64,
    pub ratio_max: f64,
    pub note: String,
}

impl SizeRule {
    pub fn gpt_image() -> Self {
        Self {
            step: 16,
            min_side: 16,
            max_w: 3840,
            max_h: 2160,
            ratio_min: 1.0 / 3.0,
            ratio_max: 3.0,
            note: "边长需被 16 整除 · 宽高比 1:3–3:1 · 上限 3840×2160（超 2560×1440 为实验性）".into(),
        }
    }

    pub fn validate(&self, w: i64, h: i64) -> Result<(), String> {
        if w < self.min_side as i64 || h < self.min_side as i64 {
            return Err(format!("宽高都不能小于 {}", self.min_side));
        }
        if w % self.step as i64 != 0 || h % self.step as i64 != 0 {
            return Err(format!("宽和高都必须能被 {} 整除", self.step));
        }
        let ratio = w as f64 / h as f64;
        if !(self.ratio_min..=self.ratio_max).contains(&ratio) {
            return Err("宽高比必须在 1:3 到 3:1 之间".into());
        }
        if w > self.max_w as i64 || h > self.max_h as i64 {
            return Err(format!("分辨率上限 {}x{}", self.max_w, self.max_h));
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiKind {
    /// OpenAI Images 兼容协议（/images/generations、/images/edits）
    OpenAiImages,
    /// 未识别协议：JSON 直发 generations，字段名照搬 api_key，以网关为准
    Generic,
}

/// 尺寸控件的比例预设：w/h 表达宽高比（也作为该比例 1.0MP 时的参考尺寸）
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RatioPreset {
    pub label: String,
    pub w: u32,
    pub h: u32,
}

/// 模型能力档案：UI 参数区、校验、请求转换全部由它驱动。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ModelProfile {
    pub id: String,
    #[serde(default)]
    pub label: String,
    pub api: ApiKind,
    pub stream: bool,
    pub max_prompt: u32,
    pub max_refs: u32,
    pub mask_edit: bool,
    pub transparent: bool,
    pub edit_note: String,
    #[serde(default)]
    pub params: Vec<ParamDef>,
    #[serde(default)]
    pub size_rule: Option<SizeRule>,
    /// 比例预设（空 = 退回扁平 presets 列表）；分辨率由「比例 × 目标百万像素」计算
    #[serde(default)]
    pub size_ratios: Vec<RatioPreset>,
}

impl ModelProfile {
    pub fn params_for(&self, mode: Mode, advanced: bool) -> Vec<&ParamDef> {
        self.params
            .iter()
            .filter(|d| d.visible_in(mode))
            .filter(|d| if advanced { d.advanced } else { !d.advanced })
            .collect()
    }

    pub fn find_param(&self, key: &str) -> Option<&ParamDef> {
        self.params.iter().find(|d| d.key == key)
    }

    /// 比例 + 目标总百万像素 → 具体 WxH。
    /// 1.0 MP ≈ 1024×1024；总像素按比例分配到宽高，受 SizeRule
    /// 上限约束时等比缩小，边长对齐 step。
    pub fn compute_mp_size(&self, ratio_w: u32, ratio_h: u32, mp: f64) -> (u32, u32) {
        let (step, max_w, max_h, min_side) = match &self.size_rule {
            Some(rule) => (rule.step, rule.max_w, rule.max_h, rule.min_side),
            None => (16, 8192, 8192, 16),
        };
        let pixels = (mp.max(0.05)) * 1024.0 * 1024.0;
        let r = ratio_w as f64 / ratio_h as f64;
        let fw = (pixels * r).sqrt();
        let fh = (pixels / r).sqrt();
        let mut s = 1.0f64;
        if let Some(rule) = &self.size_rule {
            s = s
                .min(rule.max_w as f64 / fw)
                .min(rule.max_h as f64 / fh);
        }
        let snap = |v: f64| -> u32 {
            let n = ((v / step as f64).round() as u64).max(1);
            (n as u32) * step
        };
        (
            snap(fw * s).clamp(min_side, max_w),
            snap(fh * s).clamp(min_side, max_h),
        )
    }

    /// 宽高互换后夹回规则上限（等比缩放 + 边长对齐 step）
    #[allow(dead_code)]
    pub fn clamp_size(&self, w: u32, h: u32) -> (u32, u32) {
        let (step, max_w, max_h, min_side) = match &self.size_rule {
            Some(rule) => (rule.step, rule.max_w, rule.max_h, rule.min_side),
            None => (16, 8192, 8192, 16),
        };
        let mut s = 1.0f32;
        if let Some(rule) = &self.size_rule {
            s = s
                .min(rule.max_w as f32 / w as f32)
                .min(rule.max_h as f32 / h as f32);
        }
        let snap = |v: f32| -> u32 {
            let n = ((v / step as f32).round() as u32).max(1);
            n * step
        };
        (
            snap(w as f32 * s).clamp(min_side, max_w),
            snap(h as f32 * s).clamp(min_side, max_h),
        )
    }

}

/// 资产引用（生成的图 / 上传的参考图 / mask），文件存服务端 data/assets/
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AssetRef {
    pub id: String,
    pub ext: String,
    #[serde(default)]
    pub w: Option<u32>,
    #[serde(default)]
    pub h: Option<u32>,
}

impl AssetRef {
    pub fn url(&self) -> String {
        format!("/asset/{}.{}", self.id, self.ext)
    }
}

/// 输入分组（文件夹）。删除分组时其输入回到未分组，不连带删除。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InputGroup {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub created_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Provider {
    pub id: String,
    pub name: String,
    /// 含 /v1 的根地址，如 https://api.openai.com/v1
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub models: Vec<String>,
    /// 模型元数据覆盖（provider × model），JSON 结构同 ModelProfile 的子集
    #[serde(default)]
    pub overrides: BTreeMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Config {
    pub providers: Vec<Provider>,
    /// 当前激活的 provider id
    pub active_provider: String,
}

impl Config {
    pub fn active(&self) -> Option<&Provider> {
        self.providers.iter().find(|p| p.id == self.active_provider)
    }
}

/// 配方：可复用的创作定义 —— prompt 模板 + 参数 + 固定参考图。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Recipe {
    pub id: String,
    pub provider_id: String,
    pub model_id: String,
    #[serde(default)]
    pub prompt_template: String,
    /// 手动标题；空 = 从模板推导
    #[serde(default)]
    pub title: Option<String>,
    /// 所属分组；None = 未分组
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub params: ParamMap,
    /// 固定参考图（编辑区维护，随配方走）
    #[serde(default)]
    pub refs: Vec<AssetRef>,
    #[serde(default)]
    pub mask: Option<AssetRef>,
    /// 每次修改模板/参数/固定图后 +1；Run 记录当时的版本号
    #[serde(default)]
    pub version: u32,
    pub created_at: u64,
    pub updated_at: u64,
}

impl Recipe {
    #[allow(dead_code)]
    pub fn mode(&self) -> Mode {
        if self.refs.is_empty() {
            Mode::Gen
        } else {
            Mode::Edit
        }
    }

    pub fn display_title(&self) -> String {
        if let Some(t) = self.title.as_deref() {
            let t = t.trim();
            if !t.is_empty() {
                return truncate_label(t);
            }
        }
        let line = self
            .prompt_template
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty() && !l.contains('{'))
            .unwrap_or("");
        if line.is_empty() {
            "未命名配方".into()
        } else {
            truncate_label(line)
        }
    }

// 参数校验统一走自由函数 validate_request(profile, model_id, params, image_count)
}

/// 输入层 mask 覆盖。字段为 None = 跟随配方；Off = 显式不用；Custom = 用指定 mask。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MaskOverride {
    Off,
    Custom(AssetRef),
}

/// 配方下的输入：一次具体化 —— 变量值 + 槽位图片 + 额外参考图 / mask / 参数覆盖。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RecipeInput {
    pub id: String,
    pub recipe_id: String,
    /// 手动标题；空 = 自动推导
    #[serde(default)]
    pub title: Option<String>,
    /// 文字槽 {xxx} 的值
    #[serde(default)]
    pub variables: BTreeMap<String, String>,
    /// 图片槽 {img:xxx} → 图片；未出现的槽 = 未绑定
    #[serde(default)]
    pub images: BTreeMap<String, AssetRef>,
    /// 额外参考图（跟输入走，拼在配方固定图之后、槽位图之前）
    #[serde(default)]
    pub extra_refs: Vec<AssetRef>,
    /// mask 覆盖；None = 跟随配方
    #[serde(default)]
    pub mask_override: Option<MaskOverride>,
    /// 参数覆盖：只存显式覆盖项；缺省键 = 继承配方，Unset = 显式置空不发送
    #[serde(default)]
    pub param_overrides: ParamMap,
    /// 每次修改内容后 +1；Run 记录当时的版本号
    #[serde(default = "default_input_version")]
    pub version: u32,
    pub created_at: u64,
    pub updated_at: u64,
}

fn default_input_version() -> u32 {
    1
}

impl RecipeInput {
    pub fn display_title(&self, index: usize) -> String {
        if let Some(t) = self.title.as_deref() {
            let t = t.trim();
            if !t.is_empty() {
                return truncate_label(t);
            }
        }
        if let Some(v) = self.variables.values().next() {
            let v = v.trim();
            if !v.is_empty() {
                return truncate_label(v);
            }
        }
        format!("输入 {}", index + 1)
    }

    /// 内容性字段是否一致（决定版本是否 +1；标题不算内容）
    pub fn same_content(&self, other: &RecipeInput) -> bool {
        self.variables == other.variables
            && self.images == other.images
            && self.extra_refs == other.extra_refs
            && self.mask_override == other.mask_override
            && self.param_overrides == other.param_overrides
    }

    /// 生效 mask：显式 Off → None；Custom → 指定资产；未覆盖 → 配方 mask
    pub fn effective_mask(&self, recipe_mask: Option<&AssetRef>) -> Option<AssetRef> {
        match &self.mask_override {
            Some(MaskOverride::Off) => None,
            Some(MaskOverride::Custom(a)) => Some(a.clone()),
            None => recipe_mask.cloned(),
        }
    }
}

// ---------- Prompt 模板 ----------

/// 模板片段：{xxx} 文字槽、{img:xxx} 图片槽、其余为字面文本
#[derive(Clone, Debug, PartialEq)]
pub enum TemplatePiece {
    Text(String),
    Var(String),
    Img(String),
}

/// 解析模板。`{{` 为 `{` 的转义；未闭合的 `{` 按字面处理。
pub fn parse_template(template: &str) -> Vec<TemplatePiece> {
    let mut out = vec![];
    let mut text = String::new();
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            if chars.peek() == Some(&'{') {
                chars.next();
                text.push('{');
                continue;
            }
            let mut inner = String::new();
            let mut closed = false;
            for c2 in chars.by_ref() {
                if c2 == '}' {
                    closed = true;
                    break;
                }
                inner.push(c2);
            }
            if !closed {
                text.push('{');
                text.push_str(&inner);
                continue;
            }
            let name = inner.trim();
            if let Some(img) = name.strip_prefix("img:") {
                let img = img.trim();
                if !img.is_empty() {
                    if !text.is_empty() {
                        out.push(TemplatePiece::Text(std::mem::take(&mut text)));
                    }
                    out.push(TemplatePiece::Img(img.into()));
                    continue;
                }
            }
            if !name.is_empty() {
                if !text.is_empty() {
                    out.push(TemplatePiece::Text(std::mem::take(&mut text)));
                }
                out.push(TemplatePiece::Var(name.into()));
                continue;
            }
            // 空槽按字面输出
            text.push('{');
            text.push_str(&inner);
            text.push('}');
        } else {
            text.push(c);
        }
    }
    if !text.is_empty() {
        out.push(TemplatePiece::Text(text));
    }
    out
}

/// 模板里的全部槽名（去重，保持首次出现顺序）
pub fn template_variables(template: &str) -> (Vec<String>, Vec<String>) {
    let mut vars: Vec<String> = vec![];
    let mut imgs: Vec<String> = vec![];
    for piece in parse_template(template) {
        match piece {
            TemplatePiece::Var(v) => {
                if !vars.contains(&v) {
                    vars.push(v);
                }
            }
            TemplatePiece::Img(i) => {
                if !imgs.contains(&i) {
                    imgs.push(i);
                }
            }
            TemplatePiece::Text(_) => {}
        }
    }
    (vars, imgs)
}

/// 渲染结果
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RenderedPrompt {
    pub prompt: String,
    /// 按模板出现顺序排列的槽位图片（拼在固定参考图之后发送）
    pub slot_images: Vec<AssetRef>,
    pub missing_vars: Vec<String>,
    pub missing_imgs: Vec<String>,
}

impl RenderedPrompt {
    pub fn is_complete(&self) -> bool {
        self.missing_vars.is_empty() && self.missing_imgs.is_empty()
    }
}

/// 用输入的变量值与槽位图片渲染模板。
/// `base_refs` 为配方固定参考图数量——{img:xxx} 渲染为 [图N]，
/// N 是它在「固定图 + 槽位图」合并序列中的位置（从 1 起）。
pub fn render_recipe(
    template: &str,
    values: &BTreeMap<String, String>,
    images: &BTreeMap<String, AssetRef>,
    base_refs: usize,
) -> RenderedPrompt {
    let mut out = RenderedPrompt::default();
    let mut slot_seen: Vec<String> = vec![];
    for piece in parse_template(template) {
        match piece {
            TemplatePiece::Text(t) => out.prompt.push_str(&t),
            TemplatePiece::Var(name) => match values.get(&name).map(|s| s.trim()) {
                Some(v) if !v.is_empty() => out.prompt.push_str(v),
                _ => {
                    out.missing_vars.push(name.clone());
                    out.prompt.push('{');
                    out.prompt.push_str(&name);
                    out.prompt.push('}');
                }
            },
            TemplatePiece::Img(name) => {
                let idx = if let Some(pos) = slot_seen.iter().position(|s| s == &name) {
                    pos
                } else {
                    slot_seen.push(name.clone());
                    slot_seen.len() - 1
                };
                out.prompt.push_str(&format!("[图{}]", base_refs + idx + 1));
                if let Some(asset) = images.get(&name) {
                    if out.slot_images.len() == idx {
                        out.slot_images.push(asset.clone());
                    } else {
                        out.slot_images[idx] = asset.clone();
                    }
                } else {
                    out.missing_imgs.push(name.clone());
                }
            }
        }
    }
    out
}

// ---------- 请求解析与快照 ----------

/// 模板快照：执行时刻配方的内容性字段。自包含，不随配方后续修改变化。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TemplateSnapshot {
    pub provider_id: String,
    pub model_id: String,
    pub version: u32,
    pub prompt_template: String,
    pub params: ParamMap,
    pub refs: Vec<AssetRef>,
    pub mask: Option<AssetRef>,
}

impl TemplateSnapshot {
    /// 模板快照（内容性字段；执行批次时物化进 Run）
    #[cfg(any(feature = "server", test))]
    pub fn capture(recipe: &Recipe) -> Self {
        Self {
            provider_id: recipe.provider_id.clone(),
            model_id: recipe.model_id.clone(),
            version: recipe.version,
            prompt_template: recipe.prompt_template.clone(),
            params: recipe.params.clone(),
            refs: recipe.refs.clone(),
            mask: recipe.mask.clone(),
        }
    }
}

/// 输入快照：执行时刻输入的内容性字段。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InputSnapshot {
    pub version: u32,
    pub variables: BTreeMap<String, String>,
    pub images: BTreeMap<String, AssetRef>,
    pub extra_refs: Vec<AssetRef>,
    pub mask_override: Option<MaskOverride>,
    pub param_overrides: ParamMap,
}

impl InputSnapshot {
    #[cfg(any(feature = "server", test))]
    pub fn capture(input: &RecipeInput) -> Self {
        Self {
            version: input.version,
            variables: input.variables.clone(),
            images: input.images.clone(),
            extra_refs: input.extra_refs.clone(),
            mask_override: input.mask_override.clone(),
            param_overrides: input.param_overrides.clone(),
        }
    }
}

/// 模板 + 输入合并后的最终请求 = 实际发送的内容。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ResolvedRequest {
    pub prompt: String,
    pub params: ParamMap,
    /// 发送图片序列：配方固定图 → 输入额外图 → 槽位图（{img:x} 的 [图N] 按此序编号）
    pub images: Vec<AssetRef>,
    pub mask: Option<AssetRef>,
    pub mode: Mode,
}

/// 批次快照：执行时刻的模板 / 输入 / 最终请求，共同构成可重放、可回溯的完整档案。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RunRequest {
    pub template: TemplateSnapshot,
    pub input: InputSnapshot,
    pub resolved: ResolvedRequest,
}

/// 合并逻辑唯一来源（不做完整性校验，预览与正式执行共用）。
pub fn merge_request(recipe: &Recipe, input: &RecipeInput) -> ResolvedRequest {
    let mut params = recipe.params.clone();
    for (k, v) in &input.param_overrides {
        params.insert(k.clone(), v.clone());
    }
    let base_refs = recipe.refs.len() + input.extra_refs.len();
    let rendered = render_recipe(&recipe.prompt_template, &input.variables, &input.images, base_refs);
    let mut images = recipe.refs.clone();
    images.extend(input.extra_refs.iter().cloned());
    images.extend(rendered.slot_images.iter().cloned());
    let mode = if images.is_empty() { Mode::Gen } else { Mode::Edit };
    ResolvedRequest {
        prompt: rendered.prompt,
        params,
        images,
        mask: input.effective_mask(recipe.mask.as_ref()),
        mode,
    }
}

/// 模板 + 输入 → 最终请求。槽位不齐 / prompt 为空时返回错误。
pub fn resolve_request(recipe: &Recipe, input: &RecipeInput) -> Result<ResolvedRequest, String> {
    let base_refs = recipe.refs.len() + input.extra_refs.len();
    let rendered = render_recipe(&recipe.prompt_template, &input.variables, &input.images, base_refs);
    if !rendered.is_complete() {
        let mut problems = vec![];
        if !rendered.missing_vars.is_empty() {
            problems.push(format!("未填变量：{}", rendered.missing_vars.join("、")));
        }
        if !rendered.missing_imgs.is_empty() {
            problems.push(format!("未绑定图片槽：{}", rendered.missing_imgs.join("、")));
        }
        return Err(problems.join("；"));
    }
    let merged = merge_request(recipe, input);
    if merged.prompt.trim().is_empty() {
        return Err("Prompt 模板为空".into());
    }
    Ok(merged)
}

/// 请求级校验：模型已选、图片总数上限、参数取值（按 profile 元数据）。
pub fn validate_request(
    profile: &ModelProfile,
    model_id: &str,
    params: &ParamMap,
    image_count: usize,
) -> Result<(), String> {
    if model_id.is_empty() {
        return Err("请先选择模型".into());
    }
    if image_count > profile.max_refs as usize {
        return Err(format!("图片总数超过上限 {} 张", profile.max_refs));
    }
    for (key, value) in params {
        let Some(def) = profile.find_param(key) else { continue };
        if let (Some(max), ParamValue::Number(n)) = (def.max, value) {
            if *n > max || *n < def.min.unwrap_or(f64::NEG_INFINITY) {
                return Err(format!("「{}」超出范围", def.label));
            }
        }
        if def.kind == ParamKind::Select {
            if let ParamValue::Text(s) = value {
                if !def.options.contains(s) {
                    return Err(format!("「{}」的取值 {s} 不在可选列表", def.label));
                }
            }
        }
    }
    Ok(())
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RunStatus {
    Running,
    Done,
    Error,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Usage {
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens: Option<u64>,
    #[serde(default)]
    pub total_tokens: Option<u64>,
    #[serde(default)]
    pub image_tokens: Option<u64>,
}

/// 一次生成 = 一个批次。创建时完整快照当时的模板/输入/最终请求（request），
/// 此后配方与输入的修改不影响历史批次；重跑即重放快照。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Run {
    pub id: String,
    /// 出自哪个配方
    pub recipe_id: String,
    /// 出自哪个输入（快捷运行也会自动落成输入）
    pub input_id: Option<String>,
    /// 执行时配方版本号
    pub recipe_version: u32,
    /// 执行时输入版本号（旧批次为 0）
    #[serde(default)]
    pub input_version: u32,
    pub provider_id: String,
    pub model_id: String,
    pub mode: Mode,
    /// 执行时刻的完整快照；None = 旧版批次（仅有下方 legacy 字段）
    #[serde(default)]
    pub request: Option<RunRequest>,
    /// 若本批次是对另一批次的快照重放，记录原批次 id
    #[serde(default)]
    pub rerun_of: Option<String>,
    pub status: RunStatus,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub images: Vec<AssetRef>,
    #[serde(default)]
    pub usage: Option<Usage>,
    pub created_at: u64,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    // ---- legacy：旧版批次专用，新批次不再写入 ----
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub prompt: String,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub params: ParamMap,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub ref_count: usize,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

impl Run {
    /// 渲染后的完整 prompt（快照优先，旧批次回退 legacy 字段）
    pub fn display_prompt(&self) -> &str {
        match &self.request {
            Some(req) => &req.resolved.prompt,
            None => &self.prompt,
        }
    }

    /// 实际发送的图片序列；旧批次未知 → None
    pub fn sent_images(&self) -> Option<&[AssetRef]> {
        self.request
            .as_ref()
            .map(|r| r.resolved.images.as_slice())
    }

    /// 实际发送图片数量
    pub fn image_count(&self) -> usize {
        self.sent_images().map(|s| s.len()).unwrap_or(self.ref_count)
    }

    /// 最终生效参数（模板参数合并输入覆盖后）
    pub fn effective_params(&self) -> &ParamMap {
        match &self.request {
            Some(req) => &req.resolved.params,
            None => &self.params,
        }
    }
}

fn truncate_label(s: &str) -> String {
    if s.chars().count() > 42 {
        let head: String = s.chars().take(42).collect();
        format!("{head}…")
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(id: &str) -> AssetRef {
        AssetRef {
            id: id.into(),
            ext: "png".into(),
            w: None,
            h: None,
        }
    }

    fn recipe() -> Recipe {
        Recipe {
            id: "r1".into(),
            provider_id: "p1".into(),
            model_id: "m1".into(),
            prompt_template: "画一只{animal}，参考 {img:subject}，{style} 风格".into(),
            title: None,
            group_id: None,
            params: [("size".to_string(), ParamValue::Size("1024x1024".into()))]
                .into_iter()
                .collect(),
            refs: vec![asset("fix1")],
            mask: Some(asset("mask1")),
            version: 3,
            created_at: 0,
            updated_at: 0,
        }
    }

    fn input() -> RecipeInput {
        RecipeInput {
            id: "i1".into(),
            recipe_id: "r1".into(),
            title: None,
            variables: [
                ("animal".to_string(), "橘猫".into()),
                ("style".to_string(), "水彩".into()),
            ]
            .into_iter()
            .collect(),
            images: [("subject".to_string(), asset("slot1"))].into_iter().collect(),
            extra_refs: vec![asset("extra1")],
            mask_override: None,
            param_overrides: Default::default(),
            version: 2,
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn merge_images_order_and_numbering() {
        let r = recipe();
        let i = input();
        let resolved = resolve_request(&r, &i).unwrap();
        let ids: Vec<&str> = resolved.images.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, vec!["fix1", "extra1", "slot1"]);
        // {img:subject} 是合并序列第 3 张 → 渲染为 [图3]
        assert!(resolved.prompt.contains("[图3]"));
        assert_eq!(resolved.mode, Mode::Edit);
    }

    #[test]
    fn merge_params_overlay_and_mask() {
        let mut r = recipe();
        r.params.insert("n".into(), ParamValue::Number(1.0));
        let mut i = input();
        // 覆盖：size 改值、n 显式置空
        i.param_overrides.insert(
            "size".into(),
            ParamValue::Size("1536x1024".into()),
        );
        i.param_overrides.insert("n".into(), ParamValue::Unset);
        let resolved = merge_request(&r, &i);
        assert_eq!(resolved.params.get("size"), Some(&ParamValue::Size("1536x1024".into())));
        assert_eq!(resolved.params.get("n"), Some(&ParamValue::Unset));
        // 未覆盖 mask → 跟随配方
        assert_eq!(resolved.mask.as_ref().map(|m| m.id.as_str()), Some("mask1"));
    }

    #[test]
    fn mask_override_semantics() {
        let r = recipe();
        let mut i = input();
        i.mask_override = Some(MaskOverride::Off);
        assert_eq!(merge_request(&r, &i).mask, None);
        i.mask_override = Some(MaskOverride::Custom(asset("mask2")));
        assert_eq!(merge_request(&r, &i).mask.as_ref().map(|m| m.id.as_str()), Some("mask2"));
    }

    #[test]
    fn resolve_missing_var_errors() {
        let r = recipe();
        let mut i = input();
        i.variables.remove("style");
        let err = resolve_request(&r, &i).unwrap_err();
        assert!(err.contains("style"));
    }

    #[test]
    fn input_version_predicate() {
        let a = input();
        let mut b = a.clone();
        assert!(a.same_content(&b));
        b.title = Some("改名".into());
        assert!(a.same_content(&b), "标题不算内容");
        b.extra_refs.push(asset("x"));
        assert!(!a.same_content(&b));
    }

    #[test]
    fn param_value_serde_roundtrip_and_compat() {
        // 旧数据格式（内部标签的 unset）必须仍可读
        let old: ParamValue = serde_json::from_str(r#"{"t":"unset"}"#).unwrap();
        assert_eq!(old, ParamValue::Unset);
        // 带值变体必须可序列化（此前内部标签下会直接 Err 导致静默丢写）
        for v in [
            ParamValue::Text("hello".into()),
            ParamValue::Number(2.0),
            ParamValue::Size("1024x1024".into()),
            ParamValue::Unset,
        ] {
            let s = serde_json::to_string(&v).unwrap();
            assert_eq!(serde_json::from_str::<ParamValue>(&s).unwrap(), v);
        }
    }

    #[test]
    fn legacy_run_json_still_parses() {
        // 旧版批次文件：没有 request/input_version/rerun_of 字段
        let old = r#"{
            "id": "abc",
            "recipe_id": "r1",
            "input_id": "i1",
            "recipe_version": 2,
            "provider_id": "p1",
            "model_id": "m1",
            "mode": "edit",
            "prompt": "旧 prompt",
            "params": {"quality": {"t": "unset"}},
            "ref_count": 3,
            "status": "done",
            "images": [],
            "created_at": 123
        }"#;
        let run: Run = serde_json::from_str(old).unwrap();
        assert!(run.request.is_none());
        assert_eq!(run.display_prompt(), "旧 prompt");
        assert_eq!(run.image_count(), 3);
        assert_eq!(run.input_version, 0);
    }

    #[test]
    fn run_snapshot_roundtrip() {
        let r = recipe();
        let i = input();
        let resolved = resolve_request(&r, &i).unwrap();
        let req = RunRequest {
            template: TemplateSnapshot::capture(&r),
            input: InputSnapshot::capture(&i),
            resolved,
        };
        let run = Run {
            id: "x".into(),
            recipe_id: "r1".into(),
            input_id: Some("i1".into()),
            recipe_version: 3,
            input_version: 2,
            provider_id: "p1".into(),
            model_id: "m1".into(),
            mode: Mode::Edit,
            request: Some(req.clone()),
            rerun_of: None,
            status: RunStatus::Done,
            error: None,
            images: vec![asset("out1")],
            usage: None,
            created_at: 1,
            duration_ms: Some(2),
            prompt: String::new(),
            params: Default::default(),
            ref_count: 0,
        };
        let bytes = serde_json::to_vec(&run).unwrap();
        // 新批次不再序列化 legacy 字段
        let text = String::from_utf8(bytes.clone()).unwrap();
        assert!(!text.contains("\"ref_count\""));
        let back: Run = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back.request, Some(req));
    }
}

/// 客户端 / 服务端共享的轻量工具
pub mod fmt {
    use super::*;

    pub fn relative_time(ts_ms: u64, now_ms: u64) -> String {
        let diff = now_ms.saturating_sub(ts_ms);
        let sec = diff / 1000;
        match sec {
            0..=44 => "刚刚".into(),
            45..=89 => "1 分钟前".into(),
            90..=2699 => format!("{} 分钟前", sec / 60),
            2700..=5339 => "1 小时前".into(),
            5340..=86_069 => format!("{} 小时前", sec / 3600),
            86_070..=172_739 => "1 天前".into(),
            _ => format!("{} 天前", sec / 86_400),
        }
    }

    pub fn usage_text(u: &Usage) -> String {
        let mut parts = vec![];
        if let Some(t) = u.total_tokens {
            parts.push(format!("tokens {t}"));
        }
        if let Some(i) = u.image_tokens {
            parts.push(format!("图入 {i}"));
        }
        if let Some(o) = u.output_tokens {
            parts.push(format!("出 {o}"));
        }
        parts.join(" · ")
    }
}
