//! server functions —— 客户端调用、服务端实现（dioxus fullstack）。
//!
//! Run 不依赖模板：`start_node_run` 接收节点图上生图节点装配好的请求，
//! 归档最终请求后执行；配方时代的 `start_run` 已随重构移除，旧批次仍可读可重放。

use crate::model::*;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(feature = "server")]
use crate::util::now_ms;

// ---------- 配置 ----------

#[server]
pub async fn get_config() -> Result<Config, ServerFnError> {
    Ok(crate::store::load_config().await)
}

#[server]
pub async fn save_config(cfg: Config) -> Result<(), ServerFnError> {
    crate::store::save_config(&cfg).await;
    Ok(())
}

/// 当前 provider 的全部模型档案（内置 + override 合并）
#[server]
pub async fn resolve_profiles(provider_id: String) -> Result<Vec<ModelProfile>, ServerFnError> {
    let cfg = crate::store::load_config().await;
    let Some(provider) = cfg.providers.iter().find(|p| p.id == provider_id) else {
        return Ok(vec![]);
    };
    Ok(provider
        .models
        .iter()
        .map(|m| crate::profiles::merged(m, provider.overrides.get(m)))
        .collect())
}

#[server]
pub async fn set_model_override(
    provider_id: String,
    model_id: String,
    override_json: Option<serde_json::Value>,
) -> Result<(), ServerFnError> {
    let mut cfg = crate::store::load_config().await;
    if let Some(p) = cfg.providers.iter_mut().find(|p| p.id == provider_id) {
        match override_json {
            None | Some(serde_json::Value::Null) => {
                p.overrides.remove(&model_id);
            }
            Some(v) => {
                p.overrides.insert(model_id, v);
            }
        }
        crate::store::save_config(&cfg).await;
    }
    Ok(())
}

// ---------- 节点图 ----------

#[server]
pub async fn list_graphs() -> Result<Vec<Graph>, ServerFnError> {
    Ok(crate::store::list_graphs().await)
}

/// 新建节点图：种入「生图 → 显示」最小闭环，开箱即可跑第一个 run。
#[server]
pub async fn create_graph() -> Result<Graph, ServerFnError> {
    let now = now_ms();
    let cfg = crate::store::load_config().await;
    let (provider_id, model_id) = cfg
        .active()
        .map(|p| {
            (
                p.id.clone(),
                p.models.first().cloned().unwrap_or_default(),
            )
        })
        .unwrap_or_default();
    let graph = Graph {
        id: uuid::Uuid::new_v4().simple().to_string(),
        title: "未命名图".into(),
        group_id: None,
        nodes: vec![
            GraphNode {
                id: format!("gen-{}", uuid::Uuid::new_v4().simple()),
                x: 80.0,
                y: 160.0,
                data: NodeData::Gen(GenNodeData {
                    provider_id,
                    model_id,
                    prompt: String::new(),
                    params: Default::default(),
                }),
            },
            GraphNode {
                id: format!("disp-{}", uuid::Uuid::new_v4().simple()),
                x: 480.0,
                y: 160.0,
                data: NodeData::Display(DisplayNodeData::default()),
            },
        ],
        edges: vec![],
        created_at: now,
        updated_at: now,
    };
    let mut graph = graph;
    let gen_id = graph.nodes[0].id.clone();
    let disp_id = graph.nodes[1].id.clone();
    graph.edges.push(GraphEdge {
        id: format!("e-{}", uuid::Uuid::new_v4().simple()),
        source: gen_id,
        target: disp_id,
        source_handle: None,
        target_handle: None,
    });
    crate::store::save_graph(&graph).await;
    Ok(graph)
}

/// 整图保存（节点/连线/标题/分组）。客户端以图为单位落盘。
#[server]
pub async fn update_graph(mut graph: Graph) -> Result<Graph, ServerFnError> {
    graph.updated_at = now_ms();
    crate::store::save_graph(&graph).await;
    Ok(graph)
}

#[server]
pub async fn delete_graph(id: String) -> Result<(), ServerFnError> {
    crate::store::delete_graph(&id).await;
    Ok(())
}

#[server]
pub async fn set_graph_group(
    graph_id: String,
    group_id: Option<String>,
) -> Result<(), ServerFnError> {
    crate::store::set_graph_group(&graph_id, group_id).await;
    Ok(())
}

// ---------- 分组 ----------

#[server]
pub async fn list_groups() -> Result<Vec<GraphGroup>, ServerFnError> {
    Ok(crate::store::list_groups().await)
}

#[server]
pub async fn create_group(name: String) -> Result<GraphGroup, ServerFnError> {
    let group = GraphGroup {
        id: uuid::Uuid::new_v4().simple().to_string(),
        name: if name.trim().is_empty() { "新建分组".into() } else { name },
        created_at: now_ms(),
    };
    crate::store::save_group(&group).await;
    Ok(group)
}

#[server]
pub async fn rename_group(id: String, name: String) -> Result<(), ServerFnError> {
    let mut groups: Vec<GraphGroup> = crate::store::list_groups().await;
    if let Some(g) = groups.iter_mut().find(|g| g.id == id) {
        g.name = name;
    }
    crate::store::write_groups(&groups).await;
    Ok(())
}

#[server]
pub async fn delete_group(id: String) -> Result<(), ServerFnError> {
    crate::store::delete_group(&id).await;
    Ok(())
}

// ---------- 资产 ----------

#[server]
pub async fn upload_asset(bytes: Vec<u8>, filename: String) -> Result<AssetRef, ServerFnError> {
    if bytes.is_empty() {
        return Err(ServerFnError::new("空文件"));
    }
    let ext = filename.rsplit('.').next().unwrap_or("png").to_lowercase();
    let ext = match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "gif" => {
            if ext == "jpeg" { "jpg".to_string() } else { ext }
        }
        _ => "png".to_string(),
    };
    crate::store::save_asset(&bytes, &ext)
        .await
        .map_err(ServerFnError::new)
}

// ---------- 运行 ----------

#[server]
pub async fn list_runs(limit: usize) -> Result<Vec<Run>, ServerFnError> {
    Ok(crate::store::list_runs(limit).await)
}

/// 落盘批次并异步执行最终请求；立即返回 Running 状态的批次。
#[cfg(feature = "server")]
async fn persist_and_execute(
    provider: Provider,
    profile: ModelProfile,
    resolved: ResolvedRequest,
    run: Run,
) -> Run {
    crate::store::save_run(&run).await;
    let mut run_task = run.clone();
    tokio::spawn(async move {
        let t0 = std::time::Instant::now();
        match crate::adapter::execute(
            &provider,
            crate::adapter::ExecRequest {
                prompt: &resolved.prompt,
                params: &resolved.params,
                images: &resolved.images,
                mask: resolved.mask.as_ref(),
                model_id: &run_task.model_id,
                profile: &profile,
            },
        )
        .await
        {
            Ok(outcome) => {
                run_task.status = RunStatus::Done;
                run_task.images = outcome.images;
                run_task.usage = outcome.usage;
            }
            Err((status, msg)) => {
                run_task.status = status;
                run_task.error = Some(msg);
            }
        }
        run_task.duration_ms = Some(t0.elapsed().as_millis() as u64);
        crate::store::save_run(&run_task).await;
    });
    run
}

/// 生图节点装配好的请求。Run 不依赖模板 —— 这就是发给 API 的全部内容。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NodeRunRequest {
    pub provider_id: String,
    pub model_id: String,
    pub prompt: String,
    #[serde(default)]
    pub params: ParamMap,
    #[serde(default)]
    pub images: Vec<AssetRef>,
    #[serde(default)]
    pub mask: Option<AssetRef>,
}

/// 从节点图上的生图节点发起生成：归档最终请求（此后图的修改不影响该批次），
/// 立即返回 Running 的批次。
#[server]
pub async fn start_node_run(
    graph_id: String,
    node_id: String,
    req: NodeRunRequest,
) -> Result<Run, ServerFnError> {
    let cfg = crate::store::load_config().await;
    let provider = cfg
        .providers
        .iter()
        .find(|p| p.id == req.provider_id)
        .cloned()
        .ok_or_else(|| ServerFnError::new("Provider 不存在"))?;
    let profile = crate::profiles::merged(&req.model_id, provider.overrides.get(&req.model_id));

    let mode = if req.images.is_empty() {
        Mode::Gen
    } else {
        Mode::Edit
    };
    let resolved = ResolvedRequest {
        prompt: req.prompt,
        params: req.params,
        images: req.images,
        mask: req.mask,
        mode,
    };
    if resolved.prompt.trim().is_empty() {
        return Err(ServerFnError::new("Prompt 为空"));
    }
    crate::model::validate_request(
        &profile,
        &req.model_id,
        &resolved.params,
        resolved.images.len(),
    )
    .map_err(ServerFnError::new)?;

    let run = Run {
        id: uuid::Uuid::new_v4().simple().to_string(),
        recipe_id: String::new(),
        input_id: None,
        recipe_version: 0,
        input_version: 0,
        provider_id: provider.id.clone(),
        model_id: req.model_id.clone(),
        mode: resolved.mode,
        request: None,
        graph_id: Some(graph_id),
        node_id: Some(node_id),
        resolved: Some(resolved.clone()),
        rerun_of: None,
        status: RunStatus::Running,
        error: None,
        images: vec![],
        usage: None,
        created_at: now_ms(),
        duration_ms: None,
        prompt: String::new(),
        params: Default::default(),
        ref_count: 0,
    };
    Ok(persist_and_execute(provider, profile, resolved, run).await)
}

/// 通用重放：以归档的最终请求原样执行（模型/参数/图片/mask 全部取自归档）。
#[cfg(feature = "server")]
#[allow(clippy::too_many_arguments)]
async fn replay_resolved(
    resolved: ResolvedRequest,
    provider_id: String,
    model_id: String,
    graph_id: Option<String>,
    node_id: Option<String>,
    recipe_id: String,
    recipe_version: u32,
    input_id: Option<String>,
    rerun_of: Option<String>,
) -> Result<Run, ServerFnError> {
    let cfg = crate::store::load_config().await;
    let provider = cfg
        .providers
        .iter()
        .find(|p| p.id == provider_id)
        .cloned()
        .ok_or_else(|| ServerFnError::new("Provider 不存在（批次归档里的供应商已被删除）"))?;
    let profile = crate::profiles::merged(&model_id, provider.overrides.get(&model_id));
    let run = Run {
        id: uuid::Uuid::new_v4().simple().to_string(),
        recipe_id,
        input_id,
        recipe_version,
        input_version: 0,
        provider_id: provider.id.clone(),
        model_id,
        mode: resolved.mode,
        request: None,
        graph_id,
        node_id,
        resolved: Some(resolved.clone()),
        rerun_of,
        status: RunStatus::Running,
        error: None,
        images: vec![],
        usage: None,
        created_at: now_ms(),
        duration_ms: None,
        prompt: String::new(),
        params: Default::default(),
        ref_count: 0,
    };
    Ok(persist_and_execute(provider, profile, resolved, run).await)
}

/// 原样重跑：按批次归档的最终请求重放；配方时代批次重放其快照里的请求。
#[server]
pub async fn rerun_run(run_id: String) -> Result<Run, ServerFnError> {
    let old = crate::store::get_run(&run_id)
        .await
        .ok_or_else(|| ServerFnError::new("批次不存在"))?;
    if let Some(request) = old.request.clone() {
        let resolved = request.resolved.clone();
        let provider_id = request.template.provider_id.clone();
        let model_id = request.template.model_id.clone();
        let recipe_id = old.recipe_id.clone();
        let input_id = old.input_id.clone();
        let rerun_of = Some(old.id.clone());
        return replay_resolved(
            resolved,
            provider_id,
            model_id,
            None,
            None,
            recipe_id,
            request.template.version,
            input_id,
            rerun_of,
        )
        .await;
    }
    if let Some(resolved) = old.resolved.clone() {
        let provider_id = old.provider_id.clone();
        let model_id = old.model_id.clone();
        let graph_id = old.graph_id.clone();
        let node_id = old.node_id.clone();
        let rerun_of = Some(old.id.clone());
        return replay_resolved(
            resolved,
            provider_id,
            model_id,
            graph_id,
            node_id,
            String::new(),
            0,
            None,
            rerun_of,
        )
        .await;
    }
    Err(ServerFnError::new(
        "该批次创建于快照机制之前，没有可重放的请求归档",
    ))
}

#[server]
pub async fn delete_run(run_id: String) -> Result<(), ServerFnError> {
    crate::store::delete_run(&run_id).await;
    Ok(())
}
