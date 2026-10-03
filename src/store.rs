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

// ---------- image store（两层：全局库 + 图私有） ----------
//
//   data/stores/                 全局库：平铺，用户显式收藏，跨图复用。
//                                 删图不动它；由用户显式删。
//   data/graphs/{gid}/store/     图私有：内联感知，UI 零暴露。删图级联删。
//
// 两层都是「受控命名空间」：目录名 = 显示名，manifest.json 存 w/h/bytes，
// 引用是相对文件名。画布节点只引用图私有层；全局库与画布之间经
// 「拖入即复制」打通（复制进 graphs/{gid}/store/ 后被节点引用）。

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
}

/// 图片文件名：单段 + 扩展名白名单（jpeg 归一为 jpg）
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

/// 全局库的根目录（平铺，无子目录）
fn stores_root() -> std::path::PathBuf {
    sub_dir(&["stores"])
}

fn graph_store_dir(gid: &str) -> std::path::PathBuf {
    sub_dir(&["graphs", gid, "store"])
}

async fn read_manifest_at(root: &std::path::Path) -> StoreManifest {
    read_json_at(&root.join("manifest.json")).await.unwrap_or_default()
}

async fn write_manifest_at(root: &std::path::Path, manifest: &StoreManifest) {
    write_json_at(&root.join("manifest.json"), manifest).await;
}

/// 存入一张图（写文件 + 更新 manifest）；已存在同名则覆盖。返回元信息。
async fn put_store_file(
    root: &std::path::Path,
    filename: &str,
    bytes: &[u8],
) -> Result<StoreFile, String> {
    let name = safe_store_file(filename).ok_or("文件名不合法")?;
    if bytes.is_empty() {
        return Err("空文件".into());
    }
    tokio::fs::create_dir_all(root)
        .await
        .map_err(|e| format!("创建目录失败：{e}"))?;
    tokio::fs::write(root.join(&name), bytes)
        .await
        .map_err(|e| format!("写文件失败：{e}"))?;
    let (w, h) = sniff_dimensions(bytes);
    let meta = StoreFile {
        name: name.clone(),
        w,
        h,
        bytes: bytes.len() as u64,
    };
    let mut manifest = read_manifest_at(root).await;
    manifest.files.insert(name, meta.clone());
    write_manifest_at(root, &manifest).await;
    Ok(meta)
}

async fn remove_store_file(root: &std::path::Path, name: &str) -> Result<(), String> {
    let name = safe_store_file(name).ok_or("文件名不合法")?;
    let _ = tokio::fs::remove_file(root.join(&name)).await;
    let mut manifest = read_manifest_at(root).await;
    if manifest.files.remove(&name).is_some() {
        write_manifest_at(root, &manifest).await;
    }
    Ok(())
}

/// 列出目录下全部图片（manifest 与磁盘并集，按名排序）
pub async fn list_store_files(root: &std::path::Path) -> Vec<StoreFile> {
    let manifest = read_manifest_at(root).await;
    let mut out: Vec<StoreFile> = manifest.files.values().cloned().collect();
    if let Ok(mut rd) = tokio::fs::read_dir(root).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "manifest.json" || safe_store_file(&name).is_none() {
                continue;
            }
            if out.iter().any(|f| f.name == name) {
                continue;
            }
            // 磁盘上有、manifest 没有（旧数据/外部放入）：补一条
            let bytes = entry.metadata().await.map(|m| m.len()).unwrap_or(0);
            out.push(StoreFile {
                name: name.clone(),
                w: None,
                h: None,
                bytes,
            });
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

// ---------- 全局库（data/stores/） ----------

pub async fn list_global_store() -> Vec<StoreFile> {
    list_store_files(&stores_root()).await
}

pub async fn save_global_store_file(filename: &str, bytes: &[u8]) -> Result<StoreFile, String> {
    put_store_file(&stores_root(), filename, bytes).await
}

pub async fn delete_global_store_file(name: &str) -> Result<(), String> {
    remove_store_file(&stores_root(), name).await
}

/// GET /store/{name} —— 全局库图片（immutable 缓存）
pub async fn serve_global_store_file(
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Response {
    if safe_store_file(&name).as_deref() != Some(name.as_str()) {
        return (StatusCode::BAD_REQUEST, "bad file name").into_response();
    }
    match tokio::fs::read(stores_root().join(&name)).await {
        Ok(bytes) => image_response(&name, bytes),
        Err(_) => (StatusCode::NOT_FOUND, "file not found").into_response(),
    }
}

// ---------- 图私有 store（graphs/{gid}/store/，内联感知） ----------

pub async fn list_graph_store(gid: &str) -> Vec<StoreFile> {
    list_store_files(&graph_store_dir(gid)).await
}

pub async fn save_graph_store_file(
    gid: &str,
    filename: &str,
    bytes: &[u8],
) -> Result<StoreFile, String> {
    let root = graph_store_dir(gid);
    let name = safe_store_file(filename).ok_or("文件名不合法")?;
    // 同名同内容 → 保持原名（幂等，重复上传不涨数）；
    // 同名不同内容 → 加序号另存，绝不能互相覆盖（一个节点可引用多张图）
    let name = match tokio::fs::read(root.join(&name)).await {
        Ok(existing) if existing == bytes => name,
        Ok(_) => free_store_name(&root, &name).await,
        Err(_) => name,
    };
    put_store_file(&root, &name, bytes).await
}

/// 取一个未被占用（磁盘 + manifest 都没这个名）的 store 文件名：`x.png` → `x-2.png`
async fn free_store_name(root: &std::path::Path, name: &str) -> String {
    let manifest = read_manifest_at(root).await;
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) => (s.to_string(), format!(".{e}")),
        None => (name.to_string(), String::new()),
    };
    for n in 2..1000 {
        let cand = format!("{stem}-{n}{ext}");
        if !manifest.files.contains_key(&cand) && !root.join(&cand).exists() {
            return cand;
        }
    }
    format!("{stem}-{}{ext}", new_asset_id())
}

pub async fn delete_graph_store_file(gid: &str, name: &str) -> Result<(), String> {
    remove_store_file(&graph_store_dir(gid), name).await
}

/// 读图私有 store 里的图片字节（执行引用时用）
pub async fn read_graph_store_file(gid: &str, name: &str) -> Result<Vec<u8>, String> {
    let name = safe_store_file(name).ok_or("文件名不合法")?;
    let dir = graph_store_dir(gid);
    if !dir.join("manifest.json").exists() {
        return Err("图 store 不存在".into());
    }
    tokio::fs::read(dir.join(name))
        .await
        .map_err(|_| "图 store 里没有这个文件".into())
}

/// 图私有 store 里某个文件的元信息（manifest 优先，磁盘兜底）
pub async fn graph_store_file_meta(gid: &str, name: &str) -> Option<StoreFile> {
    let name = safe_store_file(name)?;
    let manifest = read_manifest_at(&graph_store_dir(gid)).await;
    if let Some(f) = manifest.files.get(&name) {
        return Some(f.clone());
    }
    let bytes = tokio::fs::read(graph_store_dir(gid).join(&name)).await.ok()?;
    let (w, h) = sniff_dimensions(&bytes);
    Some(StoreFile {
        name,
        w,
        h,
        bytes: bytes.len() as u64,
    })
}

/// GET /gstore/{gid}/{name} —— 图私有 store 图片（immutable 缓存）
pub async fn serve_graph_store_file(
    axum::extract::Path((gid, name)): axum::extract::Path<(String, String)>,
) -> Response {
    if safe_store_file(&name).as_deref() != Some(name.as_str()) {
        return (StatusCode::BAD_REQUEST, "bad file name").into_response();
    }
    if !graph_store_dir(&gid).join("manifest.json").exists() {
        return (StatusCode::NOT_FOUND, "store not found").into_response();
    }
    match tokio::fs::read(graph_store_dir(&gid).join(&name)).await {
        Ok(bytes) => image_response(&name, bytes),
        Err(_) => (StatusCode::NOT_FOUND, "file not found").into_response(),
    }
}

/// 图片响应（按扩展名取 mime + immutable 缓存）
fn image_response(name: &str, bytes: Vec<u8>) -> Response {
    let mime = match name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()) {
        Some(e) => match e.as_str() {
            "png" => "image/png",
            "jpg" => "image/jpeg",
            "webp" => "image/webp",
            "gif" => "image/gif",
            _ => "application/octet-stream",
        },
        None => "application/octet-stream",
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

#[cfg(test)]
mod store_tests {
    use super::*;

    #[test]
    fn sniff_reads_png_ihdr() {
        // 60 字节最小 PNG：IHDR width/height = 2x2
        let png: Vec<u8> = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0x0D, 0x49, 0x48, 0x44, 0x52,
            0, 0, 0, 2, 0, 0, 0, 2, 8, 6, 0, 0, 0, 0x1F, 0x15, 0xC4, 0x89,
        ];
        assert_eq!(sniff_dimensions(&png), (Some(2), Some(2)));
    }

    #[test]
    fn safe_file_names_allow_unicode_and_reject_traversal() {
        // 中文文件名原样保留（目录名 = 显示名）
        assert_eq!(safe_store_file("猫.png").as_deref(), Some("猫.png"));
        assert_eq!(safe_store_file("x.PNG").as_deref(), Some("x.png"));
        assert_eq!(safe_store_file("x.jpeg").as_deref(), Some("x.jpg"));
        // 穿越 / 缺扩展名 / 非图片扩展名拒绝
        assert!(safe_store_file("../evil.png").is_none());
        assert!(safe_store_file("a/b.png").is_none());
        assert!(safe_store_file("no-ext").is_none());
        assert!(safe_store_file("a.svg").is_none());
        assert!(safe_store_file("..").is_none());
    }

    #[tokio::test]
    async fn two_layer_roundtrip() {
        // 测试走独立数据目录，绝不碰真实 data/（此前测试曾污染用户数据）
        let tmp = std::env::temp_dir().join(format!("atelier-store-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        unsafe { std::env::set_var("ATELIER_DATA_DIR", &tmp) };
        // 1x1 PNG
        let png: Vec<u8> = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0x0D, 0x49, 0x48, 0x44, 0x52,
            0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 0x1F, 0x15, 0xC4, 0x89,
        ];
        // 全局库
        let g = save_global_store_file("test.png", &png).await.unwrap();
        assert_eq!(g.name, "test.png");
        assert_eq!(g.w, Some(1));
        assert!(list_global_store().await.iter().any(|f| f.name == "test.png"));
        // 图私有（两层互不影响）
        let gr = save_graph_store_file("unit-test-graph", "inner.png", &png).await.unwrap();
        assert_eq!(gr.name, "inner.png");
        assert!(list_graph_store("unit-test-graph").await.iter().any(|f| f.name == "inner.png"));
        // 删全局不影响图私有
        delete_global_store_file("test.png").await.unwrap();
        assert!(!list_global_store().await.iter().any(|f| f.name == "test.png"));
        assert!(list_graph_store("unit-test-graph").await.iter().any(|f| f.name == "inner.png"));
        delete_graph_store_file("unit-test-graph", "inner.png").await.unwrap();
        assert!(list_graph_store("unit-test-graph").await.is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn graph_store_same_name_never_overwrites() {
        // 只测「同名不互相覆盖」的判定（显式 root，不碰 data/：
        // 不设 ATELIER_DATA_DIR，免得和其他测试抢进程级环境变量）
        let root = std::env::temp_dir().join(format!("atelier-gstore-collide-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let first = b"first".to_vec();
        let second = b"second".to_vec();

        // 还没有 dup.png：落原名
        put_store_file(&root, "dup.png", &first).await.unwrap();
        // 已存在 → 让位 dup-2.png（manifest 与磁盘都算占用）
        assert_eq!(free_store_name(&root, "dup.png").await, "dup-2.png");
        put_store_file(&root, "dup-2.png", &second).await.unwrap();
        put_store_file(&root, "dup-3.png", &second).await.unwrap();

        let mut names: Vec<String> =
            list_store_files(&root).await.into_iter().map(|f| f.name).collect();
        names.sort();
        assert_eq!(names, vec!["dup-2.png", "dup-3.png", "dup.png"]);
        // 原文件内容没被后来的同名上传覆盖
        assert_eq!(tokio::fs::read(root.join("dup.png")).await.unwrap(), first);
        let _ = std::fs::remove_dir_all(&root);
    }

}
