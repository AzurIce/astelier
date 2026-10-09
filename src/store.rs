//! 服务端持久化：data/ 下的目录结构 + 资产文件。
//!
//! ```text
//! data/
//! ├── config.json            Provider 配置
//! ├── groups.json            节点图分组（文件夹）
//! ├── assets/{id}.{ext}      显式上传的独立图片（生成不写入）
//! ├── graphs/{gid}/
//! │   ├── graph.json         节点图：节点（种类/参数）+ 连线
//! │   ├── view.json          布局和视口
//! │   └── store/             图内导入的参考图
//! └── stores/               用户显式收藏的图片库
//! ```
//!
//! 单用户本地工具，JSON 落盘足够；写入用 tmp+rename 原子替换。

use crate::model::{AssetRef, Config, Graph, GraphGroup, GraphView};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::sync::Mutex;

const DATA_DIR: &str = "data";

static STORE_LOCK: Mutex<()> = Mutex::const_new(());

/// 数据根目录。可用 ATELIER_DATA_DIR 覆盖（多实例隔离 / 测试用），
/// 默认 data/。注意多个实例共用同一目录时互相对彼此的图和图片库可见。
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
async fn write_json_at_unlocked<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("创建保存目录失败：{e}"))?;
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| format!("序列化失败：{e}"))?;
    let tmp = path.with_extension("tmp");
    tokio::fs::write(&tmp, &bytes)
        .await
        .map_err(|e| format!("写入失败：{e}"))?;
    tokio::fs::rename(&tmp, path)
        .await
        .map_err(|e| format!("替换保存文件失败：{e}"))?;
    Ok(())
}

async fn write_json_at<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let _guard = STORE_LOCK.lock().await;
    write_json_at_unlocked(path, value).await
}

async fn write_json<T: serde::Serialize>(rel: &[&str], value: &T) -> Result<(), String> {
    write_json_at(&sub_dir(rel), value).await
}

/// Persistent instance identity allows graph/provider references to resolve across clients.
pub async fn backend_id() -> Result<String, String> {
    let _guard = STORE_LOCK.lock().await;
    let path = sub_dir(&["backend.json"]);
    match tokio::fs::read(&path).await {
        Ok(bytes) => {
            let value: serde_json::Value =
                serde_json::from_slice(&bytes).map_err(|_| "backend.json 无法解析".to_string())?;
            let id = value
                .get("id")
                .and_then(serde_json::Value::as_str)
                .filter(|id| uuid::Uuid::parse_str(id).is_ok())
                .ok_or_else(|| "backend.json 的后端身份无效".to_string())?;
            return Ok(id.to_string());
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("读取后端身份失败：{error}")),
    }
    let id = uuid::Uuid::new_v4().simple().to_string();
    write_json_at_unlocked(&path, &serde_json::json!({ "id": id })).await?;
    Ok(id)
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
    let _ = write_json(&["config.json"], cfg).await;
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
    let _ = write_json(&["groups.json"], &v).await;
}

pub async fn write_groups(groups: &[GraphGroup]) {
    let _ = write_json(&["groups.json"], &groups).await;
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
            let _ = write_json(&["graphs", &g.id, "graph.json"], &g).await;
        }
    }
}

pub async fn set_graph_group(graph_id: &str, group_id: Option<String>) -> Result<(), String> {
    let Some(mut graph) = get_graph(graph_id).await else {
        return Err("图不存在".into());
    };
    graph.group_id = group_id;
    save_graph(&graph).await
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

pub async fn save_graph(graph: &Graph) -> Result<(), String> {
    write_json(&["graphs", &graph.id, "graph.json"], graph).await
}

pub async fn delete_graph(id: &str) {
    let _ = tokio::fs::remove_dir_all(sub_dir(&["graphs", id])).await;
}

// ---------- graph view（表现文档：布局/视口） ----------

pub async fn get_view(id: &str) -> Option<GraphView> {
    read_json(&["graphs", id, "view.json"]).await
}

/// 保存表现文档。与结构文档分离：高频保存不推进图 updated_at。
pub async fn save_view(id: &str, view: &GraphView) -> Result<(), String> {
    write_json(&["graphs", id, "view.json"], view).await
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
    let path = dir()
        .join("assets")
        .join(format!("{}.{}", asset.id, asset.ext));
    tokio::fs::read(&path)
        .await
        .map_err(|e| format!("读资产失败：{e}"))
}

/// GET /asset/{name} —— name 形如 {uuid}.{ext}
pub async fn serve_asset(axum::extract::Path(name): axum::extract::Path<String>) -> Response {
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
                    (header::CACHE_CONTROL, "no-cache".into()),
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
            if (0xC0..=0xCF).contains(&marker) && marker != 0xC4 && marker != 0xC8 && marker != 0xCC
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
    let ext = if ext == "jpeg" {
        "jpg".to_string()
    } else {
        ext
    };
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

fn graph_store_dir(gid: &str) -> std::path::PathBuf {
    sub_dir(&["graphs", gid, "store"])
}

/// 存入一张图（写文件，sniff 尺寸）；已存在同名则覆盖。返回元信息。
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
    Ok(StoreFile {
        name,
        w,
        h,
        bytes: bytes.len() as u64,
    })
}

async fn remove_store_file(root: &std::path::Path, name: &str) -> Result<(), String> {
    let name = safe_store_file(name).ok_or("文件名不合法")?;
    tokio::fs::remove_file(root.join(&name))
        .await
        .map_err(|e| format!("删除文件失败：{e}"))?;
    Ok(())
}

/// 列出目录下全部图片（磁盘即唯一真相：逐文件读头 sniff 尺寸，按名排序）
pub async fn list_store_files(root: &std::path::Path) -> Vec<StoreFile> {
    drop_legacy_manifest(root).await;
    let mut out: Vec<StoreFile> = Vec::new();
    if let Ok(mut rd) = tokio::fs::read_dir(root).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            if safe_store_file(&name).is_none() {
                continue;
            }
            let path = entry.path();
            let bytes = entry.metadata().await.map(|m| m.len()).unwrap_or(0);
            let (w, h) = sniff_dimensions_file(&path).await;
            out.push(StoreFile { name, w, h, bytes });
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// 读文件头部 sniff 尺寸（≤256KB：覆盖 EXIF 抢占的 JPEG SOF；PNG/WebP 只需 30B）
async fn sniff_dimensions_file(path: &std::path::Path) -> (Option<u32>, Option<u32>) {
    use tokio::io::AsyncReadExt;
    let Ok(mut f) = tokio::fs::File::open(path).await else {
        return (None, None);
    };
    let mut buf = vec![0u8; 256 * 1024];
    let n = f.read(&mut buf).await.unwrap_or(0);
    sniff_dimensions(&buf[..n])
}

/// 一次性清理 legacy manifest.json：早期版本库有独立的 manifest，造成
/// 「磁盘改了名 manifest 还是旧 key」的双事实来源；现在磁盘即真相，见到就删。
async fn drop_legacy_manifest(root: &std::path::Path) {
    let _ = tokio::fs::remove_file(root.join("manifest.json")).await;
}

// ---------- 全局库（data/stores/，层级：任意深度子目录） ----------
//
// 单个根 manifest.json（key = 相对路径，如 "角色/猫.png"）只提供 w/h 覆盖；
// 目录与文件以递归走磁盘为准（外部直接放入的文件也能被拾取）。

/// 库条目：目录或图片（前端据此建树 + 网格）
#[derive(Serialize, Clone, Debug)]
pub struct StoreTree {
    pub dirs: Vec<String>,
    pub files: Vec<StoreFileEntry>,
}

/// 带相对路径的图片条目
#[derive(Serialize, Clone, Debug)]
pub struct StoreFileEntry {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub w: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h: Option<u32>,
    pub bytes: u64,
}

/// 全局库的根目录
fn stores_root() -> std::path::PathBuf {
    sub_dir(&["stores"])
}

/// 相对路径安全化：按 '/' 分段校验。目录段容忍任意可见字符（中文原样），
/// 文件段额外要求扩展名白名单。禁绝对路径/穿越/超深。
pub fn safe_store_path(path: &str) -> Option<String> {
    let path = path.trim();
    if path.is_empty() || path.len() > 240 {
        return None;
    }
    let segs: Vec<&str> = path.split('/').collect();
    if segs.len() > 8 {
        return None;
    }
    let mut out: Vec<String> = Vec::with_capacity(segs.len());
    for (i, seg) in segs.iter().enumerate() {
        let seg = seg.trim();
        if seg.is_empty() || seg == "." || seg == ".." {
            return None;
        }
        if seg.contains('\\')
            || seg.contains(':')
            || seg.contains('*')
            || seg.contains('?')
            || seg.contains('"')
            || seg.contains('<')
            || seg.contains('>')
            || seg.contains('|')
        {
            return None;
        }
        if i + 1 == segs.len() {
            // 末段 = 文件：扩展名白名单
            let name = safe_store_file(seg)?;
            out.push(name);
        } else {
            out.push(seg.to_string());
        }
    }
    // 解析结果必须仍落在库根内（双重保险）
    let joined = out.join("/");
    let real = stores_root().join(&joined);
    if !real.starts_with(stores_root()) {
        return None;
    }
    Some(joined)
}

/// 列全树：磁盘即唯一真相（逐文件 sniff 尺寸）
pub async fn list_global_store() -> StoreTree {
    let root = stores_root();
    drop_legacy_manifest(&root).await;
    let mut dirs: Vec<String> = Vec::new();
    let mut files: Vec<StoreFileEntry> = Vec::new();
    collect_tree(&root, "", &mut dirs, &mut files).await;
    dirs.sort();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    StoreTree { dirs, files }
}

/// 递归收集（相对路径用 '/' 连接；深度与总量上限防失控）
async fn collect_tree(
    dir: &std::path::Path,
    rel: &str,
    dirs: &mut Vec<String>,
    files: &mut Vec<StoreFileEntry>,
) {
    if dirs.len() + files.len() > 5000 {
        return;
    }
    let Ok(mut rd) = tokio::fs::read_dir(dir).await else {
        return;
    };
    while let Ok(Some(entry)) = rd.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "manifest.json" {
            continue;
        }
        let child_rel = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        let Ok(ft) = entry.file_type().await else {
            continue;
        };
        if ft.is_dir() {
            dirs.push(child_rel.clone());
            Box::pin(collect_tree(&entry.path(), &child_rel, dirs, files)).await;
        } else if safe_store_file(&name).is_some() {
            let bytes = entry.metadata().await.map(|m| m.len()).unwrap_or(0);
            let (w, h) = sniff_dimensions_file(&entry.path()).await;
            files.push(StoreFileEntry {
                path: child_rel,
                w,
                h,
                bytes,
            });
        }
    }
}

/// 上传到指定子目录（dir 空 = 根）；同名覆盖。返回带相对路径的条目。
pub async fn save_global_store_file(
    dir: &str,
    filename: &str,
    bytes: &[u8],
) -> Result<StoreFileEntry, String> {
    let dir = if dir.trim().is_empty() {
        String::new()
    } else {
        resolve_store_dir_path(dir).ok_or("目标目录不合法")?
    };
    let rel = if dir.is_empty() {
        safe_store_file(filename).ok_or("文件名不合法")?
    } else {
        format!("{dir}/{}", safe_store_file(filename).ok_or("文件名不合法")?)
    };
    if bytes.is_empty() {
        return Err("空文件".into());
    }
    let abs = stores_root().join(&rel);
    if let Some(parent) = abs.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("创建目录失败：{e}"))?;
    }
    tokio::fs::write(&abs, bytes)
        .await
        .map_err(|e| format!("写文件失败：{e}"))?;
    let (w, h) = sniff_dimensions(bytes);
    Ok(StoreFileEntry {
        path: rel,
        w,
        h,
        bytes: bytes.len() as u64,
    })
}

/// 新建文件夹（可多级，如 "角色/猫"）
pub async fn make_global_store_dir(path: &str) -> Result<(), String> {
    // 目录路径不带扩展名，逐段按目录名校验
    let segs: Vec<&str> = path.trim().split('/').collect();
    if segs.is_empty() || segs.len() > 8 {
        return Err("目录路径不合法".into());
    }
    let mut norm: Vec<String> = Vec::with_capacity(segs.len());
    for seg in segs {
        let seg = seg.trim();
        if seg.is_empty() || seg == "." || seg == ".." {
            return Err("目录路径不合法".into());
        }
        if seg.contains('\\')
            || seg.contains('/')
            || seg.contains(':')
            || seg.contains('*')
            || seg.contains('?')
            || seg.contains('"')
            || seg.contains('<')
            || seg.contains('>')
            || seg.contains('|')
        {
            return Err("目录路径不合法".into());
        }
        norm.push(seg.to_string());
    }
    let joined = norm.join("/");
    let abs = stores_root().join(&joined);
    if !abs.starts_with(stores_root()) {
        return Err("目录路径不合法".into());
    }
    tokio::fs::create_dir_all(&abs)
        .await
        .map_err(|e| format!("创建目录失败：{e}"))?;
    Ok(())
}

/// 重命名 / 移动：文件或目录整体搬到新相对路径。manifest 同步改 key。
pub async fn move_global_store_path(from: &str, to: &str) -> Result<(), String> {
    let from = resolve_store_dir_path(from).ok_or("源路径不合法")?;
    let to = resolve_store_dir_path(to).ok_or("目标路径不合法")?;
    if from == to {
        return Ok(());
    }
    if to.starts_with(&format!("{from}/")) {
        return Err("不能移动到自己的子目录".into());
    }
    let src = stores_root().join(&from);
    let dst = stores_root().join(&to);
    if !dst.starts_with(stores_root()) {
        return Err("目标路径不合法".into());
    }
    if let Some(parent) = dst.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("创建目录失败：{e}"))?;
    }
    tokio::fs::rename(&src, &dst)
        .await
        .map_err(|e| format!("移动失败：{e}"))?;
    // 磁盘即真相：renaming 后无需同步任何索引
    Ok(())
}

/// 目录路径（末段也按目录名规则，不带扩展名）
fn resolve_store_dir_path(path: &str) -> Option<String> {
    let path = path.trim();
    if path.is_empty() || path.len() > 240 {
        return None;
    }
    let segs: Vec<&str> = path.split('/').collect();
    if segs.len() > 8 {
        return None;
    }
    let mut out: Vec<String> = Vec::with_capacity(segs.len());
    for seg in segs {
        let seg = seg.trim();
        if seg.is_empty()
            || seg == "."
            || seg == ".."
            || seg.contains('\\')
            || seg.contains(':')
            || seg.contains('*')
            || seg.contains('?')
            || seg.contains('"')
            || seg.contains('<')
            || seg.contains('>')
            || seg.contains('|')
        {
            return None;
        }
        out.push(seg.to_string());
    }
    Some(out.join("/"))
}

/// 删除文件或目录（目录递归）；manifest 同步清理（含子文件前缀）。
pub async fn delete_global_store_file(name: &str) -> Result<(), String> {
    // 文件路径（带扩展名）或目录路径都接受
    let rel = if safe_store_path(name).is_some() {
        safe_store_path(name).unwrap()
    } else {
        resolve_store_dir_path(name).ok_or("路径不合法")?
    };
    let abs = stores_root().join(&rel);
    if !abs.starts_with(stores_root()) {
        return Err("路径不合法".into());
    }
    let meta = tokio::fs::metadata(&abs).await;
    match meta {
        Ok(m) if m.is_dir() => {
            tokio::fs::remove_dir_all(&abs)
                .await
                .map_err(|e| format!("删除目录失败：{e}"))?;
        }
        Ok(_) => {
            tokio::fs::remove_file(&abs)
                .await
                .map_err(|e| format!("删除文件失败：{e}"))?;
        }
        Err(_) => {
            // 磁盘上已不存在：可能是 manifest 残留，继续清 manifest
        }
    }
    Ok(())
}

/// GET /store/*path —— 全局库图片（覆盖后必须重新读取）
pub async fn serve_global_store_file(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Response {
    let rel = match safe_store_path(&path) {
        Some(r) => r,
        None => return (StatusCode::BAD_REQUEST, "bad path").into_response(),
    };
    match tokio::fs::read(stores_root().join(&rel)).await {
        Ok(bytes) => image_response(&rel, bytes),
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

/// 取一个磁盘上未被占用的 store 文件名：`x.png` → `x-2.png`
async fn free_store_name(root: &std::path::Path, name: &str) -> String {
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) => (s.to_string(), format!(".{e}")),
        None => (name.to_string(), String::new()),
    };
    for n in 2..1000 {
        let cand = format!("{stem}-{n}{ext}");
        if !root.join(&cand).exists() {
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
    tokio::fs::read(dir.join(name))
        .await
        .map_err(|_| "图 store 里没有这个文件".into())
}

/// GET /gstore/{gid}/{name} —— 图私有 store 图片
pub async fn serve_graph_store_file(
    axum::extract::Path((gid, name)): axum::extract::Path<(String, String)>,
) -> Response {
    if safe_store_file(&name).as_deref() != Some(name.as_str()) {
        return (StatusCode::BAD_REQUEST, "bad file name").into_response();
    }
    match tokio::fs::read(graph_store_dir(&gid).join(&name)).await {
        Ok(bytes) => image_response(&name, bytes),
        Err(_) => (StatusCode::NOT_FOUND, "file not found").into_response(),
    }
}

/// 图片响应（按扩展名取 mime，路径资源允许覆盖）
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
            (header::CACHE_CONTROL, "no-cache".into()),
        ],
        bytes,
    )
        .into_response()
}

#[cfg(test)]
pub(crate) mod store_tests {
    use super::*;

    /// 测试全部走独立数据目录，绝不碰真实 data/。
    /// ATELIER_DATA_DIR 是进程级环境变量、多个 #[tokio::test] 并行跑会互相踩，
    /// 故用全局锁串行化（曾出现「这轮失败下轮又过」的灵异现象，根因就在此）。
    /// 返回的 guard 必须活到测试结束（`let (p, _g) = use_tmp(..)`）：一旦提前松手，
    /// 后来的测试会永远排在锁上，整个 cargo test 挂住。
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    pub(crate) fn use_tmp(tag: &str) -> (std::path::PathBuf, std::sync::MutexGuard<'static, ()>) {
        let guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let p = std::env::temp_dir().join(format!("atelier-store-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        unsafe { std::env::set_var("ATELIER_DATA_DIR", &p) };
        (p, guard)
    }

    /// 尺寸检测只需 PNG IHDR 头，fixture 不依赖真实用户文件。
    fn sample_png() -> Vec<u8> {
        let mut header = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        header.extend_from_slice(&1580u32.to_be_bytes());
        header.extend_from_slice(&996u32.to_be_bytes());
        header.extend_from_slice(&[8, 6, 0, 0, 0]);
        header
    }

    #[test]
    fn safe_file_names_allow_unicode_and_reject_traversal() {
        assert_eq!(safe_store_file("猫.png").as_deref(), Some("猫.png"));
        assert_eq!(safe_store_file("x.PNG").as_deref(), Some("x.png"));
        assert_eq!(safe_store_file("x.jpeg").as_deref(), Some("x.jpg"));
        assert!(safe_store_file("../evil.png").is_none());
        assert!(safe_store_file("a/b.png").is_none());
        assert!(safe_store_file("no-ext").is_none());
        assert!(safe_store_file("a.svg").is_none());
    }

    #[test]
    fn safe_paths_allow_nested_dirs_and_reject_escape() {
        // 层级路径：中文目录名原样保留
        assert_eq!(
            safe_store_path("角色/猫.png").as_deref(),
            Some("角色/猫.png")
        );
        assert_eq!(safe_store_path("a/b/c.png").as_deref(), Some("a/b/c.png"));
        assert_eq!(safe_store_path("猫.png").as_deref(), Some("猫.png"));
        // 穿越 / 绝对路径 / 空段 / 超深 / 坏扩展名
        assert!(safe_store_path("../x.png").is_none());
        assert!(safe_store_path("a/../x.png").is_none());
        assert!(safe_store_path("/abs/x.png").is_none());
        assert!(safe_store_path("a//x.png").is_none());
        assert!(safe_store_path("a/b/c/d/e/f/g/h/i.png").is_none());
        assert!(safe_store_path("a/x.svg").is_none());
        assert!(safe_store_path("").is_none());
    }

    #[tokio::test]
    async fn global_tree_roundtrip() {
        let (tmp, _g) = use_tmp("tree");
        let png = sample_png();
        // 建目录 → 上传根与子目录各一张
        make_global_store_dir("角色/猫").await.unwrap();
        let a = save_global_store_file("", "root.png", &png).await.unwrap();
        let b = save_global_store_file("角色", "a.png", &png).await.unwrap();
        let c = save_global_store_file("角色/猫", "b.png", &png)
            .await
            .unwrap();

        let tree = list_global_store().await;
        assert!(tree.dirs.contains(&"角色".to_string()));
        assert!(tree.dirs.contains(&"角色/猫".to_string()));
        assert_eq!(a.path, "root.png");
        assert_eq!(b.path, "角色/a.png");
        assert_eq!(c.path, "角色/猫/b.png");
        let b_entry = tree.files.iter().find(|f| f.path == "角色/a.png").unwrap();
        assert_eq!((b_entry.w, b_entry.h), (Some(1580), Some(996)));

        // 移动文件到另一目录
        move_global_store_path("角色/a.png", "场景/a.png")
            .await
            .unwrap();
        // 移动整个目录
        move_global_store_path("角色/猫", "猫").await.unwrap();
        let t2 = list_global_store().await;
        assert!(t2.files.iter().any(|f| f.path == "场景/a.png"));
        assert!(!t2.files.iter().any(|f| f.path == "角色/a.png"));
        assert!(t2.files.iter().any(|f| f.path == "猫/b.png"));
        assert!(t2.dirs.contains(&"猫".to_string()));

        // 禁移入自身子目录
        assert!(move_global_store_path("猫", "猫/子").await.is_err());

        // 删目录递归（同时清 manifest 前缀）
        delete_global_store_file("猫").await.unwrap();
        let t3 = list_global_store().await;
        assert!(!t3.files.iter().any(|f| f.path.starts_with("猫/")));

        // 删单文件
        delete_global_store_file("场景/a.png").await.unwrap();
        let t4 = list_global_store().await;
        assert!(!t4.files.iter().any(|f| f.path == "场景/a.png"));
        assert!(t4.files.iter().any(|f| f.path == "root.png"));

        // 外置文件（用户直接从文件管理器放入）立即被拾取
        std::fs::create_dir_all(tmp.join("stores/外部")).unwrap();
        std::fs::write(tmp.join("stores/外部/x.png"), &png).unwrap();
        let t5 = list_global_store().await;
        assert!(t5.files.iter().any(|f| f.path == "外部/x.png"));
        let ext = t5.files.iter().find(|f| f.path == "外部/x.png").unwrap();
        assert_eq!((ext.w, ext.h), (Some(1580), Some(996)));

        // 外部改名：磁盘是唯一真相，list 立即反映（无需同步任何索引）
        std::fs::rename(
            tmp.join("stores/外部/x.png"),
            tmp.join("stores/外部/改名后.png"),
        )
        .unwrap();
        let t6 = list_global_store().await;
        assert!(t6.files.iter().any(|f| f.path == "外部/改名后.png"));
        assert!(!t6.files.iter().any(|f| f.path == "外部/x.png"));

        // legacy manifest.json：一次性清理（双事实来源时代的残留）
        std::fs::write(
            tmp.join("stores/manifest.json"),
            r#"{"files":{"幽灵.png":{"name":"幽灵.png","bytes":1}}}"#,
        )
        .unwrap();
        let t7 = list_global_store().await;
        assert!(
            !t7.files.iter().any(|f| f.path == "幽灵.png"),
            "manifest 不得再作为数据来源"
        );
        assert!(
            !tmp.join("stores/manifest.json").exists(),
            "legacy manifest 应被清理"
        );

        // 静态服务：层级路径（改名后的新路径）+ 缓存头；旧路径 404
        let resp =
            serve_global_store_file(axum::extract::Path("外部/改名后.png".to_string())).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let gone = serve_global_store_file(axum::extract::Path("外部/x.png".to_string())).await;
        assert_eq!(gone.status(), StatusCode::NOT_FOUND);
        let bad = serve_global_store_file(axum::extract::Path("../x.png".to_string())).await;
        assert_eq!(bad.status(), StatusCode::BAD_REQUEST);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn two_layer_roundtrip() {
        // 图私有层：两层互不影响
        let (tmp, _g) = use_tmp("graph");
        let png = sample_png();
        let g = save_graph_store_file("unit-test-graph", "inner.png", &png)
            .await
            .unwrap();
        assert_eq!(g.name, "inner.png");
        assert!(list_graph_store("unit-test-graph")
            .await
            .iter()
            .any(|f| f.name == "inner.png"));
        // 图 store 已不写 manifest；刚上传的图片必须能直接预览 / 再次拖出。
        assert!(!graph_store_dir("unit-test-graph")
            .join("manifest.json")
            .exists());
        let response = serve_graph_store_file(axum::extract::Path((
            "unit-test-graph".to_string(),
            "inner.png".to_string(),
        )))
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let served = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(served.as_ref(), png.as_slice());
        delete_graph_store_file("unit-test-graph", "inner.png")
            .await
            .unwrap();
        assert!(list_graph_store("unit-test-graph").await.is_empty());
        let missing = serve_graph_store_file(axum::extract::Path((
            "unit-test-graph".to_string(),
            "inner.png".to_string(),
        )))
        .await;
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn graph_store_same_name_never_overwrites() {
        // 只测「同名不互相覆盖」的判定（显式 root，不碰 data/：
        // 不设 ATELIER_DATA_DIR，免得和其他测试抢进程级环境变量）
        let root =
            std::env::temp_dir().join(format!("atelier-gstore-collide-{}", std::process::id()));
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

        let mut names: Vec<String> = list_store_files(&root)
            .await
            .into_iter()
            .map(|f| f.name)
            .collect();
        names.sort();
        assert_eq!(names, vec!["dup-2.png", "dup-3.png", "dup.png"]);
        // 原文件内容没被后来的同名上传覆盖
        assert_eq!(tokio::fs::read(root.join("dup.png")).await.unwrap(), first);
        let _ = std::fs::remove_dir_all(&root);
    }
}
