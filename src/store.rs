//! 服务端持久化：data/ 下的目录结构 + 资产文件。
//!
//! ```text
//! data/
//! ├── config.json            Provider 配置
//! ├── groups.json            节点图分组（文件夹）
//! ├── assets/{id}.{ext}      全部图片资产（统一池，被各方按 id 引用）
//! ├── graphs/{gid}/
//! │   └── graph.json         节点图：节点（种类/位置/装配内容）+ 连线
//! └── runs/{run_id}.json     批次档案：状态 + 最终请求归档（旧批次为配方快照）
//! ```
//!
//! 单用户本地工具，JSON 落盘足够；写入用 tmp+rename 原子替换。

use crate::model::{AssetRef, Config, Graph, GraphGroup, GraphView, Run};
use axum::http::{header, StatusCode};
use axum::extract::Query;
use axum::response::{IntoResponse, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::sync::Mutex;

const DATA_DIR: &str = "data";

static STORE_LOCK: Mutex<()> = Mutex::const_new(());

/// 数据根目录。可用 ATELIER_DATA_DIR 覆盖（多实例隔离 / 测试用），
/// 默认 data/。注意多个实例共用同一目录时互相对彼此的图、批次可见。
fn dir() -> PathBuf {
    let root = std::env::var("ATELIER_DATA_DIR").unwrap_or_else(|_| DATA_DIR.to_string());
    Path::new(&root).to_path_buf()
}

pub fn sub_dir(parts: &[&str]) -> PathBuf {
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

// ---------- groups（节点图分组）----------

pub async fn list_groups() -> Vec<GraphGroup> {
    let mut v: Vec<GraphGroup> = read_json(&["groups.json"]).await.unwrap_or_default();
    v.sort_by_key(|g| g.created_at);
    v
}

pub async fn save_group(group: &GraphGroup) {
    let mut v: Vec<GraphGroup> = read_json(&["groups.json"]).await.unwrap_or_default();
    if let Some(slot) = v.iter_mut().find(|x| x.id == group.id) {
        *slot = group.clone();
    } else {
        v.push(group.clone());
    }
    write_json(&["groups.json"], &v).await;
}

pub async fn write_groups(groups: &[GraphGroup]) {
    write_json(&["groups.json"], &groups).await;
}

/// 删除目录及其全部子目录（递归）；其中的图回到未分组（图是资产，不连带删）。
pub async fn delete_group(id: &str) {
    let mut v: Vec<GraphGroup> = read_json(&["groups.json"]).await.unwrap_or_default();
    let mut doomed: Vec<String> = vec![id.to_string()];
    loop {
        let mut grew = false;
        for g in &v {
            if let Some(p) = &g.parent_id {
                if doomed.contains(p) && !doomed.contains(&g.id) {
                    doomed.push(g.id.clone());
                    grew = true;
                }
            }
        }
        if !grew {
            break;
        }
    }
    v.retain(|x| !doomed.contains(&x.id));
    write_groups(&v).await;
    for graph in list_graphs().await {
        if graph
            .group_id
            .as_deref()
            .map_or(false, |gid| doomed.iter().any(|d| d == gid))
        {
            let mut g = graph;
            g.group_id = None;
            write_json(&["graphs", &g.id, "graph.json"], &g).await;
        }
    }
}

pub async fn set_graph_group(graph_id: &str, group_id: Option<String>) {
    let Some(mut graph) = get_graph(graph_id).await else {
        return;
    };
    graph.group_id = group_id;
    save_graph(&graph).await;
}

// ---------- graphs ----------

pub async fn list_graphs() -> Vec<Graph> {
    let mut out: Vec<Graph> = vec![];
    let root = sub_dir(&["graphs"]);
    let _ = tokio::fs::create_dir_all(&root).await;
    if let Ok(mut rd) = tokio::fs::read_dir(&root).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            if entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
                let path = entry.path().join("graph.json");
                if let Some(graph) = read_json_at::<Graph>(&path).await {
                    if !graph.id.is_empty() {
                        out.push(graph);
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    out
}

pub async fn get_graph(id: &str) -> Option<Graph> {
    read_json(&["graphs", id, "graph.json"]).await
}

pub async fn save_graph(graph: &Graph) {
    write_json(&["graphs", &graph.id, "graph.json"], graph).await;
}

pub async fn delete_graph(id: &str) {
    let _ = tokio::fs::remove_dir_all(sub_dir(&["graphs", id])).await;
}

// ---------- 目录命名（图目录名 = 图名，人类可读） ----------

/// 图目录名净化：替换文件系统非法字符、折叠空白、限长；空则回退默认名
pub fn sanitize_dir_name(title: &str) -> String {
    let cleaned: String = title
        .trim()
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            _ => c,
        })
        .collect();
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut s: String = collapsed.chars().take(64).collect();
    s = s.trim().trim_matches('.').to_string();
    if s.is_empty() {
        "未命名图".into()
    } else {
        s
    }
}

/// 目录名去重：已存在则追加 -2 / -3…
pub fn unique_graph_dir(base: &str) -> String {
    let root = sub_dir(&["graphs"]);
    for cand in std::iter::once(base.to_string()).chain((2..).map(|i| format!("{base}-{i}"))) {
        if !root.join(&cand).exists() {
            return cand;
        }
    }
    unreachable!()
}

/// 图目录改名后，把 runs 档案里对旧图 id 的引用改指新 id
pub async fn retarget_runs_graph(old_id: &str, new_id: &str) {
    for mut run in list_runs(usize::MAX).await {
        if run.graph_id.as_deref() == Some(old_id) {
            run.graph_id = Some(new_id.to_string());
            save_run(&run).await;
        }
    }
}

// ---------- graph view（表现文档：布局/视口/最近产物缓存） ----------

pub async fn get_view(id: &str) -> Option<GraphView> {
    read_json(&["graphs", id, "view.json"]).await
}

/// 保存表现文档。与结构文档分离：高频保存不推进图 updated_at。
pub async fn save_view(id: &str, view: &GraphView) {
    write_json(&["graphs", id, "view.json"], view).await;
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

// ---------- image store（图内资产库：graphs/{gid}/store/{name}/） ----------
//
// 与 assets/ 统一池的区别：store 是「用户显式命名的库」，按图归属，
// 画布上 image store 节点的落点。图删除（remove_dir_all graphs/{gid}）
// 自动级联；图改名只换外层目录名，store 引用用 {store}/{file} 相对址。

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StoreFile {
    pub name: String,
    #[serde(default)]
    pub w: Option<u32>,
    #[serde(default)]
    pub h: Option<u32>,
    #[serde(default)]
    pub bytes: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct StoreManifest {
    #[serde(default)]
    pub files: std::collections::BTreeMap<String, StoreFile>,
    /// 展示名（用户输入的语言文本）；目录名 name 是其 ASCII slug
    #[serde(default)]
    pub title: String,
}

/// URL / 目录段安全名（store slug）：ASCII 保留，其余折叠为 '-'；
/// 折叠后为空（纯非 ASCII 名）或含穿越残留時，用展示名的小写 hex
/// 摘要兜底，保证任何语言输入都能得到 URL 安全、目录安全的标识。
/// 只作标识用；人类可读名走 manifest.title。
pub fn safe_store_name(name: &str) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut out = String::new();
    for c in trimmed.chars() {
        match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => out.push(c),
            // '.' 等特殊字符一律折叠，杜绝 '..' 穿越与点号名
            _ => {
                if !out.ends_with('-') {
                    out.push('-')
                }
            }
        }
    }
    let slug = out.trim_matches('-').to_string();
    if slug.is_empty() || slug == "." || slug == ".." || slug.contains('/') || slug.contains('\\') {
        // 兜底：展示名摘要（ASCII hex），避开 '.' 与 '/' 等特殊字符
        let mut hex = String::new();
        for b in trimmed.as_bytes() {
            hex.push_str(&format!("{b:02x}"));
        }
        let digest: String = hex.chars().take(24).collect();
        if digest.is_empty() {
            return None;
        }
        let out = format!("s-{digest}");
        if out.len() > 64 {
            return None;
        }
        return Some(out);
    }
    if slug.len() > 64 {
        return None;
    }
    Some(slug)
}

/// store 内文件名：单段 + 扩展名白名单
pub fn safe_store_file(name: &str) -> Option<String> {
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return None;
    }
    let (stem, ext) = name.rsplit_once('.')?;
    let ext = ext.to_ascii_lowercase();
    if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp" | "gif") {
        return None;
    }
    let ext = if ext == "jpeg" { "jpg".to_string() } else { ext };
    let clean_stem: String = stem
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        .collect();
    let clean_stem = clean_stem.trim().to_string();
    if clean_stem.is_empty() {
        return None;
    }
    Some(format!("{clean_stem}.{ext}"))
}

fn store_dir(gid: &str, store: &str) -> std::path::PathBuf {
    sub_dir(&["graphs", gid, "store", store])
}

pub async fn list_stores(gid: &str) -> Vec<(String, StoreManifest)> {
    let root = sub_dir(&["graphs", gid, "store"]);
    let mut out = Vec::new();
    let Ok(mut rd) = tokio::fs::read_dir(&root).await else {
        return out;
    };
    while let Ok(Some(entry)) = rd.next_entry().await {
        if !entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        out.push((name.clone(), read_manifest(gid, &name).await));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

pub async fn read_manifest(gid: &str, store: &str) -> StoreManifest {
    read_json(&["graphs", gid, "store", store, "manifest.json"])
        .await
        .unwrap_or_default()
}

async fn write_manifest(gid: &str, store: &str, manifest: &StoreManifest) {
    write_json(&["graphs", gid, "store", store, "manifest.json"], manifest).await;
}

/// 存入一张图（写文件 + 更新 manifest）。返回文件元信息。
pub async fn save_store_asset(
    gid: &str,
    store: &str,
    filename: &str,
    bytes: &[u8],
) -> Result<StoreFile, String> {
    let name = safe_store_file(filename).ok_or("文件名不合法")?;
    if bytes.is_empty() {
        return Err("空文件".into());
    }
    let (w, h) = sniff_dimensions(bytes);
    let dir = store_dir(gid, store);
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("创建 store 失败：{e}"))?;
    tokio::fs::write(dir.join(&name), bytes)
        .await
        .map_err(|e| format!("写文件失败：{e}"))?;
    let meta = StoreFile {
        name: name.clone(),
        w,
        h,
        bytes: bytes.len() as u64,
    };
    let mut manifest = read_manifest(gid, store).await;
    manifest.files.insert(name, meta.clone());
    write_manifest(gid, store, &manifest).await;
    Ok(meta)
}

/// 建库（写 manifest，title = 展示名）。已存在则只更新 title。
pub async fn create_store_dir(gid: &str, slug: &str, title: &str) -> Result<(), String> {
    let dir = store_dir(gid, slug);
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("创建 store 失败：{e}"))?;
    let mut manifest = read_manifest(gid, slug).await;
    manifest.title = title.trim().to_string();
    write_manifest(gid, slug, &manifest).await;
    Ok(())
}

pub async fn delete_store_asset(gid: &str, store: &str, name: &str) -> Result<(), String> {
    let name = safe_store_file(name).ok_or("文件名不合法")?;
    let _ = tokio::fs::remove_file(store_dir(gid, store).join(&name)).await;
    let mut manifest = read_manifest(gid, store).await;
    if manifest.files.remove(&name).is_some() {
        write_manifest(gid, store, &manifest).await;
    }
    Ok(())
}

/// 删除整个 store（目录）。manifest 一并消失。
pub async fn delete_store(gid: &str, store: &str) -> Result<(), String> {
    tokio::fs::remove_dir_all(store_dir(gid, store))
        .await
        .map_err(|e| format!("删除 store 失败：{e}"))
}

/// 重命名 store（目录改名 + manifest 随行）。next 为展示名，
/// 内部 slug 化；只改大小写等 slug 不变的改名只更新 title。
pub async fn rename_store(gid: &str, store: &str, next: &str) -> Result<(), String> {
    let slug = safe_store_name(next).ok_or("名称不合法")?;
    let from = store_dir(gid, store);
    let to = store_dir(gid, &slug);
    let mut manifest = read_manifest(gid, store).await;
    manifest.title = next.trim().to_string();
    if from == to {
        write_manifest(gid, store, &manifest).await;
        return Ok(());
    }
    if to.exists() {
        return Err("同名 store 已存在".into());
    }
    tokio::fs::rename(&from, &to)
        .await
        .map_err(|e| format!("重命名失败：{e}"))?;
    write_manifest(gid, &slug, &manifest).await;
    Ok(())
}

/// GET /gstore?g={gid}&s={store}&f={name} —— store 内图片（immutable 缓存）。
/// 用 query 而非路径段：图名 / store 名可能含非 ASCII，matchit 对路径段
/// 的未编码非 ASCII 匹配会失败并落到 SPA（非 GET 方法显 405）。
#[derive(Deserialize)]
pub struct StoreFileQuery {
    g: String,
    s: String,
    f: String,
}

pub async fn serve_store_file(Query(q): Query<StoreFileQuery>) -> Response {
    if safe_store_file(&q.f).as_deref() != Some(q.f.as_str()) {
        return (StatusCode::BAD_REQUEST, "bad file name").into_response();
    }
    let Some(slug) = safe_store_name(&q.s) else {
        return (StatusCode::BAD_REQUEST, "bad store name").into_response();
    };
    if slug != q.s {
        // 展示名被 slug 化过：直接寻址必须用 slug
        return (StatusCode::NOT_FOUND, "store not found").into_response();
    }
    if !store_dir(&q.g, &slug).join("manifest.json").exists() {
        return (StatusCode::NOT_FOUND, "store not found").into_response();
    }
    match tokio::fs::read(store_dir(&q.g, &slug).join(&q.f)).await {
        Ok(bytes) => {
            let mime = match q
                .f
                .rsplit_once('.')
                .map(|(_, e)| e.to_ascii_lowercase())
                .as_deref()
            {
                Some("png") => "image/png",
                Some("jpg") => "image/jpeg",
                Some("webp") => "image/webp",
                Some("gif") => "image/gif",
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
        Err(_) => (StatusCode::NOT_FOUND, "file not found").into_response(),
    }
}

#[cfg(test)]
mod store_tests {
    use super::*;

    fn tmp_root(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("atelier-store-test-{tag}"));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn safe_names_reject_traversal_and_bad_ext() {
        // ASCII 直通；空格折叠为 '-'
        assert_eq!(safe_store_name("lib").as_deref(), Some("lib"));
        assert_eq!(safe_store_name("ref images").as_deref(), Some("ref-images"));
        // ".." 被折叠为空 → 走摘要兜底（不为 None，且不含 '.'
        // 保证目录/URL 安全）；纯空白输入仍拒绝
        let dots = safe_store_name("..").unwrap();
        assert!(dots.starts_with("s-") && !dots.contains('.'));
        assert!(safe_store_name("  ").is_none());
        // 纯非 ASCII 名 → hex 摘要兜底，且 URL/目录安全
        let cn = safe_store_name("参考图").unwrap();
        assert!(cn.starts_with("s-"));
        assert!(cn.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
        // 混合：ASCII 段 + 折叠
        assert_eq!(safe_store_name("my 图库").as_deref(), Some("my"));
        assert_eq!(safe_store_file("a.PNG").as_deref(), Some("a.png"));
        assert_eq!(safe_store_file("x.jpeg").as_deref(), Some("x.jpg"));
        assert!(safe_store_file("../evil.png").is_none());
        assert!(safe_store_file("no-ext").is_none());
        assert!(safe_store_file("a.svg").is_none());
    }

    #[tokio::test]
    async fn store_asset_roundtrip_and_delete() {
        // 1x1 PNG
        let png: Vec<u8> = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0x0D, 0x49, 0x48, 0x44, 0x52,
            0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 0x1F, 0x15, 0xC4, 0x89,
        ];
        let gid = "unit-test-graph";
        let manifest = save_store_asset(gid, "图库", "a.png", &png).await.unwrap();
        assert_eq!(manifest.name, "a.png");
        assert_eq!(manifest.w, Some(1));
        let list = read_manifest(gid, "图库").await;
        assert!(list.files.contains_key("a.png"));
        delete_store_asset(gid, "图库", "a.png").await.unwrap();
        assert!(read_manifest(gid, "图库").await.files.is_empty());
        assert!(safe_store_file("a.svg").is_none());
        let _ = tmp_root("unused");
    }
}
