//! 服务端持久化：data/ 下的目录结构 + 资产文件。
//!
//! ```text
//! data/
//! ├── config.json            Provider 配置
//! ├── groups.json            配方分组（文件夹）
//! ├── assets/{id}.{ext}      全部图片资产（统一池，被各方按 id 引用）
//! ├── recipes/{rid}/
//! │   ├── recipe.json        配方定义
//! │   └── inputs/{iid}.json  配方下的输入（变量值 + 槽位图片）
//! └── runs/{run_id}.json     批次档案：状态 + 执行时刻的模板/输入/最终请求完整快照
//! ```
//!
//! 单用户本地工具，JSON 落盘足够；写入用 tmp+rename 原子替换。

use crate::model::{AssetRef, Config, InputGroup, Recipe, RecipeInput, Run};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};
use tokio::sync::Mutex;

const DATA_DIR: &str = "data";

static STORE_LOCK: Mutex<()> = Mutex::const_new(());

fn dir() -> PathBuf {
    Path::new(DATA_DIR).to_path_buf()
}

fn sub_dir(parts: &[&str]) -> PathBuf {
    let mut p = dir();
    for part in parts {
        p.push(part);
    }
    p
}

async fn read_json_at<T: DeserializeOwned>(path: &Path) -> Option<T> {
    match tokio::fs::read(path).await {
        Ok(bytes) => serde_json::from_slice(&bytes).ok(),
        Err(_) => None,
    }
}

async fn read_json<T: DeserializeOwned>(rel: &[&str]) -> Option<T> {
    read_json_at(&sub_dir(rel)).await
}

/// 无锁内部写。仅供已持有 STORE_LOCK 的调用方使用。
async fn write_json_at_unlocked<T: serde::Serialize>(path: &Path, value: &T) {
    if let Some(parent) = path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    let tmp = path.with_extension("tmp");
    if let Ok(bytes) = serde_json::to_vec_pretty(value) {
        if tokio::fs::write(&tmp, &bytes).await.is_ok() {
            let _ = tokio::fs::rename(&tmp, &path).await;
        }
    }
}

async fn write_json_at<T: serde::Serialize>(path: &Path, value: &T) {
    let _guard = STORE_LOCK.lock().await;
    write_json_at_unlocked(path, value).await;
}

async fn write_json<T: serde::Serialize>(rel: &[&str], value: &T) {
    write_json_at(&sub_dir(rel), value).await;
}

/// 旧版扁平存储（inputs.json / runs.json）迁移到目录结构。
/// 幂等：recipes 目录非空或旧文件不存在时直接跳过。
pub async fn migrate_legacy() {
    let legacy_inputs = sub_dir(&["inputs.json"]);
    let legacy_runs = sub_dir(&["runs.json"]);
    if !legacy_inputs.exists() || sub_dir(&["recipes"]).exists() {
        return;
    }
    let _guard = STORE_LOCK.lock().await;

    #[derive(serde::Deserialize)]
    struct LegacyInput {
        id: String,
        provider_id: String,
        model_id: String,
        #[serde(default)]
        prompt: String,
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        group_id: Option<String>,
        #[serde(default)]
        params: crate::model::ParamMap,
        #[serde(default)]
        refs: Vec<AssetRef>,
        #[serde(default)]
        mask: Option<AssetRef>,
        #[serde(default)]
        created_at: u64,
        #[serde(default)]
        updated_at: u64,
    }

    let old: Vec<LegacyInput> = read_json_at(&legacy_inputs).await.unwrap_or_default();
    for old_input in old.iter() {
        // 旧 refs 直接成为配方固定参考图 —— 迁移后行为与旧版一致（edits 端点、prompt 原样）
        let recipe = Recipe {
            id: old_input.id.clone(),
            provider_id: old_input.provider_id.clone(),
            model_id: old_input.model_id.clone(),
            prompt_template: old_input.prompt.clone(),
            title: old_input.title.clone(),
            group_id: old_input.group_id.clone(),
            params: old_input.params.clone(),
            refs: old_input.refs.clone(),
            mask: old_input.mask.clone(),
            version: 1,
            created_at: old_input.created_at,
            updated_at: old_input.updated_at,
        };
        write_json_at_unlocked(
            &sub_dir(&["recipes", &recipe.id, "recipe.json"]),
            &recipe,
        )
        .await;
        let input = RecipeInput {
            id: format!("{}-m0", old_input.id),
            recipe_id: old_input.id.clone(),
            title: Some("迁移".into()),
            variables: Default::default(),
            images: Default::default(),
            extra_refs: vec![],
            mask_override: None,
            param_overrides: Default::default(),
            version: 1,
            created_at: old_input.created_at,
            updated_at: old_input.updated_at,
        };
        write_json_at_unlocked(
            &sub_dir(&["recipes", &recipe.id, "inputs", &format!("{}.json", input.id)]),
            &input,
        )
        .await;
    }

    #[derive(serde::Deserialize)]
    struct LegacyRun {
        id: String,
        input_id: String,
        #[serde(default)]
        #[allow(dead_code)]
        recipe_version: Option<u32>,
        provider_id: String,
        model_id: String,
        mode: crate::model::Mode,
        #[serde(default)]
        prompt: String,
        #[serde(default)]
        params: crate::model::ParamMap,
        #[serde(default)]
        ref_count: usize,
        status: crate::model::RunStatus,
        #[serde(default)]
        error: Option<String>,
        #[serde(default)]
        images: Vec<AssetRef>,
        #[serde(default)]
        usage: Option<crate::model::Usage>,
        #[serde(default)]
        created_at: u64,
        #[serde(default)]
        duration_ms: Option<u64>,
    }

    let old_runs: Vec<LegacyRun> = read_json_at(&legacy_runs).await.unwrap_or_default();
    for r in old_runs {
        let run = Run {
            id: r.id.clone(),
            recipe_id: r.input_id.clone(),
            input_id: Some(format!("{}-m0", r.input_id)),
            recipe_version: 1,
            input_version: 0,
            provider_id: r.provider_id,
            model_id: r.model_id,
            mode: r.mode,
            request: None,
            rerun_of: None,
            status: r.status,
            error: r.error,
            images: r.images,
            usage: r.usage,
            created_at: r.created_at,
            duration_ms: r.duration_ms,
            prompt: r.prompt,
            params: r.params,
            ref_count: r.ref_count,
        };
        write_json_at_unlocked(
            &sub_dir(&["runs", &format!("{}.json", run.id)]),
            &run,
        )
        .await;
    }

    // 旧文件挪进 legacy/ 留档
    let legacy_dir = sub_dir(&["legacy"]);
    let _ = tokio::fs::create_dir_all(&legacy_dir).await;
    let _ = tokio::fs::rename(&legacy_inputs, legacy_dir.join("inputs.json")).await;
    let _ = tokio::fs::rename(&legacy_runs, legacy_dir.join("runs.json")).await;
}

// ---------- config ----------

pub async fn load_config() -> Config {
    let mut cfg: Config = read_json(&["config.json"]).await.unwrap_or_default();
    if cfg.providers.is_empty() {
        cfg = default_config();
        save_config(&cfg).await;
    }
    cfg
}

fn default_config() -> Config {
    Config {
        active_provider: "openai".into(),
        providers: vec![
            crate::model::Provider {
                id: "openai".into(),
                name: "OpenAI 官方".into(),
                base_url: "https://api.openai.com/v1".into(),
                api_key: String::new(),
                models: vec![
                    "gpt-image-2".into(),
                    "gpt-image-2.5-sunburst".into(),
                    "gpt-image-2.5-flare".into(),
                ],
                overrides: Default::default(),
            },
            crate::model::Provider {
                id: "poke".into(),
                name: "Poke API".into(),
                base_url: "https://www.poke2api.com/v1".into(),
                api_key: String::new(),
                models: vec![
                    "gpt-image-2".into(),
                    "gpt-image-2.5-sunburst".into(),
                    "gpt-image-2.5-flare".into(),
                ],
                overrides: Default::default(),
            },
        ],
    }
}

pub async fn save_config(cfg: &Config) {
    write_json(&["config.json"], cfg).await;
}

// ---------- groups（配方分组）----------

pub async fn list_groups() -> Vec<InputGroup> {
    let mut v: Vec<InputGroup> = read_json(&["groups.json"]).await.unwrap_or_default();
    v.sort_by_key(|g| g.created_at);
    v
}

pub async fn save_group(group: &InputGroup) {
    let mut v: Vec<InputGroup> = read_json(&["groups.json"]).await.unwrap_or_default();
    if let Some(slot) = v.iter_mut().find(|x| x.id == group.id) {
        *slot = group.clone();
    } else {
        v.push(group.clone());
    }
    write_json(&["groups.json"], &v).await;
}

pub async fn write_groups(groups: &[InputGroup]) {
    write_json(&["groups.json"], &groups).await;
}

/// 删除分组，其中的配方回到未分组
pub async fn delete_group(id: &str) {
    let mut v: Vec<InputGroup> = read_json(&["groups.json"]).await.unwrap_or_default();
    v.retain(|x| x.id != id);
    write_json(&["groups.json"], &v).await;
    for recipe in list_recipes().await {
        if recipe.group_id.as_deref() == Some(id) {
            let mut r = recipe;
            r.group_id = None;
            write_json(&["recipes", &r.id, "recipe.json"], &r).await;
        }
    }
}

pub async fn set_recipe_group(recipe_id: &str, group_id: Option<String>) {
    let Some(mut recipe) = read_json::<Recipe>(&["recipes", recipe_id, "recipe.json"]).await else {
        return;
    };
    recipe.group_id = group_id;
    write_json(&["recipes", recipe_id, "recipe.json"], &recipe).await;
}

// ---------- recipes ----------

pub async fn list_recipes() -> Vec<Recipe> {
    let mut out: Vec<Recipe> = vec![];
    let root = sub_dir(&["recipes"]);
    let _ = tokio::fs::create_dir_all(&root).await;
    if let Ok(mut rd) = tokio::fs::read_dir(&root).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            if entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
                let path = entry.path().join("recipe.json");
                if let Some(recipe) = read_json_at::<Recipe>(&path).await {
                    if !recipe.id.is_empty() {
                        out.push(recipe);
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    out
}

pub async fn get_recipe(id: &str) -> Option<Recipe> {
    read_json(&["recipes", id, "recipe.json"]).await
}

pub async fn save_recipe(recipe: &Recipe) {
    write_json(&["recipes", &recipe.id, "recipe.json"], recipe).await;
}

pub async fn delete_recipe(id: &str) {
    let _ = tokio::fs::remove_dir_all(sub_dir(&["recipes", id])).await;
}

// ---------- recipe inputs ----------

pub async fn list_recipe_inputs(recipe_id: &str) -> Vec<RecipeInput> {
    let mut out: Vec<RecipeInput> = vec![];
    let root = sub_dir(&["recipes", recipe_id, "inputs"]);
    if let Ok(mut rd) = tokio::fs::read_dir(&root).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            if entry.file_type().await.map(|t| t.is_file()).unwrap_or(false) {
                if let Some(input) = read_json_at::<RecipeInput>(&entry.path()).await {
                    if !input.id.is_empty() {
                        out.push(input);
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    out
}

pub async fn get_recipe_input(recipe_id: &str, input_id: &str) -> Option<RecipeInput> {
    read_json(&["recipes", recipe_id, "inputs", &format!("{input_id}.json")]).await
}

pub async fn save_recipe_input(recipe_id: &str, input: &RecipeInput) {
    write_json(
        &["recipes", recipe_id, "inputs", &format!("{}.json", input.id)],
        input,
    )
    .await;
}

pub async fn delete_recipe_input(recipe_id: &str, input_id: &str) {
    let _ = tokio::fs::remove_file(sub_dir(&[
        "recipes",
        recipe_id,
        "inputs",
        &format!("{input_id}.json"),
    ]))
    .await;
}

// ---------- README ----------

pub fn readme_path(recipe_id: &str) -> PathBuf {
    sub_dir(&["recipes", recipe_id, "README.md"])
}

pub async fn read_readme(recipe_id: &str) -> String {
    tokio::fs::read_to_string(readme_path(recipe_id))
        .await
        .unwrap_or_default()
}

pub async fn write_readme(recipe_id: &str, content: &str) {
    let _ = tokio::fs::create_dir_all(sub_dir(&["recipes", recipe_id])).await;
    let _ = tokio::fs::write(readme_path(recipe_id), content).await;
}

// ---------- runs ----------

pub async fn list_runs(limit: usize) -> Vec<Run> {
    let mut out: Vec<Run> = vec![];
    let root = sub_dir(&["runs"]);
    if let Ok(mut rd) = tokio::fs::read_dir(&root).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            if entry.file_type().await.map(|t| t.is_file()).unwrap_or(false) {
                if let Some(run) = read_json_at::<Run>(&entry.path()).await {
                    if !run.id.is_empty() {
                        out.push(run);
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    out.truncate(limit);
    out
}

pub async fn get_run(id: &str) -> Option<Run> {
    read_json(&["runs", &format!("{id}.json")]).await
}

pub async fn save_run(run: &Run) {
    write_json(&["runs", &format!("{}.json", run.id)], run).await;
}

pub async fn delete_run(id: &str) {
    let _ = tokio::fs::remove_file(sub_dir(&["runs", &format!("{id}.json")])).await;
}

// ---------- assets ----------

pub fn new_asset_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

/// 保存资产字节，返回引用。ext 从文件名或字节嗅探。
pub async fn save_asset(bytes: &[u8], ext: &str) -> Result<AssetRef, String> {
    let id = new_asset_id();
    write_asset_file(&id, bytes, ext).await?;
    let (w, h) = sniff_dimensions(bytes);
    Ok(AssetRef {
        id,
        ext: ext.into(),
        w,
        h,
    })
}

async fn write_asset_file(id: &str, bytes: &[u8], ext: &str) -> Result<(), String> {
    let path = dir().join("assets").join(format!("{id}.{ext}"));
    if let Some(parent) = path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    tokio::fs::write(&path, bytes)
        .await
        .map_err(|e| format!("写资产失败：{e}"))
}

pub async fn read_asset(asset: &AssetRef) -> Result<Vec<u8>, String> {
    let path = dir().join("assets").join(format!("{}.{}", asset.id, asset.ext));
    tokio::fs::read(&path)
        .await
        .map_err(|e| format!("读资产失败：{e}"))
}

/// GET /asset/{name} —— name 形如 {uuid}.{ext}
pub async fn serve_asset(
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Response {
    let Some((id, ext)) = name.rsplit_once('.') else {
        return (StatusCode::BAD_REQUEST, "bad asset name").into_response();
    };
    if !id.chars().all(|c| c.is_ascii_alphanumeric()) || ext.len() > 5 {
        return (StatusCode::BAD_REQUEST, "bad asset name").into_response();
    }
    let asset = AssetRef {
        id: id.into(),
        ext: ext.into(),
        w: None,
        h: None,
    };
    match read_asset(&asset).await {
        Ok(bytes) => {
            let mime = match ext {
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "webp" => "image/webp",
                "gif" => "image/gif",
                _ => "application/octet-stream",
            };
            (
                [
                    (header::CONTENT_TYPE, mime.to_string()),
                    (
                        header::CACHE_CONTROL,
                        "public, max-age=31536000, immutable".into(),
                    ),
                ],
                bytes,
            )
                .into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, "asset not found").into_response(),
    }
}

/// 从 PNG / JPEG / WebP 字节里嗅探宽高（尽力而为）
fn sniff_dimensions(bytes: &[u8]) -> (Option<u32>, Option<u32>) {
    if bytes.len() > 24 && &bytes[0..8] == b"\x89PNG\r\n\x1a\n" {
        let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        return (Some(w), Some(h));
    }
    if bytes.len() > 3 && &bytes[0..3] == b"\xff\xd8\xff" {
        let mut i = 2;
        while i + 9 < bytes.len() {
            if bytes[i] != 0xFF {
                break;
            }
            let marker = bytes[i + 1];
            let len = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
            if (0xC0..=0xCF).contains(&marker)
                && marker != 0xC4
                && marker != 0xC8
                && marker != 0xCC
            {
                if i + 9 <= bytes.len() {
                    let h = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]) as u32;
                    let w = u16::from_be_bytes([bytes[i + 7], bytes[i + 8]]) as u32;
                    return (Some(w), Some(h));
                }
            }
            i += 2 + len;
        }
    }
    if bytes.len() > 30 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        if &bytes[12..16] == b"VP8X" && bytes.len() > 30 {
            let w = 1 + (bytes[24] as u32 | (bytes[25] as u32) << 8 | (bytes[26] as u32) << 16);
            let h = 1 + (bytes[27] as u32 | (bytes[28] as u32) << 8 | (bytes[29] as u32) << 16);
            return (Some(w), Some(h));
        }
    }
    (None, None)
}
