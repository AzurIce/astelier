//! server functions —— 客户端调用、服务端实现（dioxus fullstack）。

use crate::model::*;
use dioxus::prelude::*;
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

// ---------- 配方 ----------

#[server]
pub async fn list_recipes() -> Result<Vec<Recipe>, ServerFnError> {
    Ok(crate::store::list_recipes().await)
}

#[server]
pub async fn create_recipe(provider_id: String, model_id: String) -> Result<Recipe, ServerFnError> {
    let now = now_ms();
    let recipe = Recipe {
        id: uuid::Uuid::new_v4().simple().to_string(),
        provider_id,
        model_id,
        prompt_template: String::new(),
        title: Some("未命名".into()),
        group_id: None,
        params: Default::default(),
        refs: vec![],
        mask: None,
        version: 1,
        created_at: now,
        updated_at: now,
    };
    crate::store::save_recipe(&recipe).await;
    Ok(recipe)
}

/// 保存配方：内容性字段（模板/参数/固定图/mask/model）变化时版本 +1
#[server]
pub async fn update_recipe(mut recipe: Recipe) -> Result<Recipe, ServerFnError> {
    let stored = crate::store::get_recipe(&recipe.id).await;
    recipe.updated_at = now_ms();
    if let Some(old) = stored {
        let content_changed = old.prompt_template != recipe.prompt_template
            || old.params != recipe.params
            || old.refs != recipe.refs
            || old.mask != recipe.mask
            || old.model_id != recipe.model_id;
        if content_changed {
            recipe.version = old.version + 1;
        } else {
            recipe.version = old.version;
        }
    }
    crate::store::save_recipe(&recipe).await;
    Ok(recipe)
}

#[server]
pub async fn delete_recipe(id: String) -> Result<(), ServerFnError> {
    crate::store::delete_recipe(&id).await;
    Ok(())
}

#[server]
pub async fn set_recipe_group(
    recipe_id: String,
    group_id: Option<String>,
) -> Result<(), ServerFnError> {
    crate::store::set_recipe_group(&recipe_id, group_id).await;
    Ok(())
}

// ---------- 配方下的输入 ----------

#[server]
pub async fn list_inputs(recipe_id: String) -> Result<Vec<RecipeInput>, ServerFnError> {
    Ok(crate::store::list_recipe_inputs(&recipe_id).await)
}

#[server]
pub async fn create_input(recipe_id: String) -> Result<RecipeInput, ServerFnError> {
    let now = now_ms();
    let input = RecipeInput {
        id: uuid::Uuid::new_v4().simple().to_string(),
        recipe_id: recipe_id.clone(),
        title: None,
        variables: Default::default(),
        images: Default::default(),
        extra_refs: vec![],
        mask_override: None,
        param_overrides: Default::default(),
        version: 1,
        created_at: now,
        updated_at: now,
    };
    crate::store::save_recipe_input(&recipe_id, &input).await;
    Ok(input)
}

/// 保存输入：id 为空时自动创建（v1）；已存在则内容性字段
/// （变量/槽位图/额外图/mask/参数覆盖）变化时版本 +1
#[server]
pub async fn update_input(mut input: RecipeInput) -> Result<RecipeInput, ServerFnError> {
    let stored = if input.id.is_empty() {
        input.id = uuid::Uuid::new_v4().simple().to_string();
        input.created_at = now_ms();
        input.version = 1;
        None
    } else {
        crate::store::get_recipe_input(&input.recipe_id, &input.id).await
    };
    input.updated_at = now_ms();
    if let Some(old) = stored {
        input.created_at = old.created_at;
        input.version = if old.same_content(&input) {
            old.version
        } else {
            old.version + 1
        };
    }
    crate::store::save_recipe_input(&input.recipe_id, &input).await;
    Ok(input)
}

/// 从批次快照复制出一份新输入，便于基于历史微调而不污染原输入。
#[server]
pub async fn new_input_from_run(run_id: String) -> Result<RecipeInput, ServerFnError> {
    let run = crate::store::get_run(&run_id)
        .await
        .ok_or_else(|| ServerFnError::new("批次不存在"))?;
    let request = run
        .request
        .ok_or_else(|| ServerFnError::new("旧版批次没有输入快照，无法复制"))?;
    let now = now_ms();
    let input = RecipeInput {
        id: uuid::Uuid::new_v4().simple().to_string(),
        recipe_id: run.recipe_id.clone(),
        title: Some("来自批次".into()),
        variables: request.input.variables,
        images: request.input.images,
        extra_refs: request.input.extra_refs,
        mask_override: request.input.mask_override,
        param_overrides: request.input.param_overrides,
        version: 1,
        created_at: now,
        updated_at: now,
    };
    crate::store::save_recipe_input(&run.recipe_id, &input).await;
    Ok(input)
}

#[server]
pub async fn delete_input(recipe_id: String, input_id: String) -> Result<(), ServerFnError> {
    crate::store::delete_recipe_input(&recipe_id, &input_id).await;
    Ok(())
}

// ---------- 分组 ----------

#[server]
pub async fn list_groups() -> Result<Vec<InputGroup>, ServerFnError> {
    Ok(crate::store::list_groups().await)
}

#[server]
pub async fn create_group(name: String) -> Result<InputGroup, ServerFnError> {
    let group = InputGroup {
        id: uuid::Uuid::new_v4().simple().to_string(),
        name: if name.trim().is_empty() { "新建分组".into() } else { name },
        created_at: now_ms(),
    };
    crate::store::save_group(&group).await;
    Ok(group)
}

#[server]
pub async fn rename_group(id: String, name: String) -> Result<(), ServerFnError> {
    let mut groups: Vec<InputGroup> = crate::store::list_groups().await;
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

// ---------- README ----------

#[server]
pub async fn get_readme(recipe_id: String) -> Result<String, ServerFnError> {
    Ok(crate::store::read_readme(&recipe_id).await)
}

#[server]
pub async fn save_readme(recipe_id: String, content: String) -> Result<(), ServerFnError> {
    crate::store::write_readme(&recipe_id, &content).await;
    Ok(())
}

/// Markdown → HTML（预览用；渲染在服务端，客户端免 md 解析器）
#[server]
pub async fn render_markdown(md: String) -> Result<String, ServerFnError> {
    use pulldown_cmark::{html, Options, Parser};
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_TASKLISTS);
    let mut html_out = String::new();
    html::push_html(&mut html_out, Parser::new_ext(&md, opts));
    Ok(html_out)
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

/// 用配方 + 一份输入发起生成。输入会被落库（新建或更新），
/// 合并、校验在服务端完成；创建时完整快照当时的模板/输入/最终请求，
/// 此后配方与输入的修改不影响该批次。
#[server]
pub async fn start_run(recipe_id: String, mut input: RecipeInput) -> Result<Run, ServerFnError> {
    let recipe = crate::store::get_recipe(&recipe_id)
        .await
        .ok_or_else(|| ServerFnError::new("配方不存在"))?;
    let cfg = crate::store::load_config().await;
    let provider = cfg
        .providers
        .iter()
        .find(|p| p.id == recipe.provider_id)
        .cloned()
        .ok_or_else(|| ServerFnError::new("Provider 不存在"))?;
    let profile = crate::profiles::merged(
        &recipe.model_id,
        provider.overrides.get(&recipe.model_id),
    );

    // 落库输入（新建或更新，内容变化时版本 +1）
    let stored = if input.id.is_empty() {
        input.id = uuid::Uuid::new_v4().simple().to_string();
        input.created_at = now_ms();
        input.version = 1;
        None
    } else {
        crate::store::get_recipe_input(&recipe_id, &input.id).await
    };
    input.recipe_id = recipe_id.clone();
    input.updated_at = now_ms();
    if let Some(old) = &stored {
        input.created_at = old.created_at;
        input.version = if old.same_content(&input) {
            old.version
        } else {
            old.version + 1
        };
    }
    crate::store::save_recipe_input(&recipe_id, &input).await;

    // 模板 + 输入 → 最终请求；按元数据校验
    let resolved = crate::model::resolve_request(&recipe, &input).map_err(ServerFnError::new)?;
    crate::model::validate_request(&profile, &recipe.model_id, &resolved.params, resolved.images.len())
        .map_err(ServerFnError::new)?;

    let run = Run {
        id: uuid::Uuid::new_v4().simple().to_string(),
        recipe_id: recipe.id.clone(),
        input_id: Some(input.id.clone()),
        recipe_version: recipe.version,
        input_version: input.version,
        provider_id: provider.id.clone(),
        model_id: recipe.model_id.clone(),
        mode: resolved.mode,
        request: Some(crate::model::RunRequest {
            template: crate::model::TemplateSnapshot::capture(&recipe),
            input: crate::model::InputSnapshot::capture(&input),
            resolved: resolved.clone(),
        }),
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

/// 快照重放：按批次快照的最终请求原样再执行一次（模型/参数/图片/mask 全部取自快照），
/// 与配方、输入的当前状态完全无关。
#[cfg(feature = "server")]
async fn replay_request(
    request: crate::model::RunRequest,
    recipe_id: String,
    input_id: Option<String>,
    rerun_of: Option<String>,
) -> Result<Run, ServerFnError> {
    let cfg = crate::store::load_config().await;
    let provider = cfg
        .providers
        .iter()
        .find(|p| p.id == request.template.provider_id)
        .cloned()
        .ok_or_else(|| ServerFnError::new("Provider 不存在（快照里的供应商已被删除）"))?;
    let profile = crate::profiles::merged(
        &request.template.model_id,
        provider.overrides.get(&request.template.model_id),
    );
    let resolved = request.resolved.clone();
    let run = Run {
        id: uuid::Uuid::new_v4().simple().to_string(),
        recipe_id,
        input_id,
        recipe_version: request.template.version,
        input_version: request.input.version,
        provider_id: provider.id.clone(),
        model_id: request.template.model_id.clone(),
        mode: resolved.mode,
        request: Some(request),
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

/// 原样重跑：有快照的批次直接重放快照；旧版批次（无快照）回退为
/// 「当前配方 + 当前输入」重跑。
#[server]
pub async fn rerun_run(run_id: String) -> Result<Run, ServerFnError> {
    let old = crate::store::get_run(&run_id)
        .await
        .ok_or_else(|| ServerFnError::new("批次不存在"))?;
    if let Some(request) = old.request.clone() {
        return replay_request(request, old.recipe_id, old.input_id, Some(old.id)).await;
    }
    // 旧版批次回退路径
    let input_id = old
        .input_id
        .clone()
        .ok_or_else(|| ServerFnError::new("该批次没有关联输入，无法重跑"))?;
    let input = crate::store::get_recipe_input(&old.recipe_id, &input_id)
        .await
        .ok_or_else(|| ServerFnError::new("关联输入已被删除，无法重跑"))?;
    start_run(old.recipe_id, input).await
}

#[server]
pub async fn delete_run(run_id: String) -> Result<(), ServerFnError> {
    crate::store::delete_run(&run_id).await;
    Ok(())
}
