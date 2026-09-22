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
        created_at: now,
        updated_at: now,
    };
    crate::store::save_recipe_input(&recipe_id, &input).await;
    Ok(input)
}

#[server]
pub async fn update_input(input: RecipeInput) -> Result<RecipeInput, ServerFnError> {
    let mut input = input;
    input.updated_at = now_ms();
    crate::store::save_recipe_input(&input.recipe_id, &input).await;
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

/// 用配方 + 一份输入发起生成。输入会被落库（新建或更新），
/// 模板渲染、校验、执行全部在服务端完成；立即返回 Running 状态的批次。
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

    recipe
        .validate_params(&profile)
        .map_err(ServerFnError::new)?;

    // 落库输入（新建或更新）
    if input.id.is_empty() {
        input.id = uuid::Uuid::new_v4().simple().to_string();
        input.created_at = now_ms();
    }
    input.recipe_id = recipe_id.clone();
    input.updated_at = now_ms();
    crate::store::save_recipe_input(&recipe_id, &input).await;

    // 渲染模板
    let rendered = crate::model::render_recipe(
        &recipe.prompt_template,
        &input.variables,
        &input.images,
        recipe.refs.len(),
    );
    if !rendered.is_complete() {
        let mut problems = vec![];
        if !rendered.missing_vars.is_empty() {
            problems.push(format!("未填变量：{}", rendered.missing_vars.join("、")));
        }
        if !rendered.missing_imgs.is_empty() {
            problems.push(format!("未绑定图片槽：{}", rendered.missing_imgs.join("、")));
        }
        return Err(ServerFnError::new(problems.join("；")));
    }
    if rendered.prompt.trim().is_empty() {
        return Err(ServerFnError::new("Prompt 模板为空"));
    }

    // 合并图片：固定参考图在前，槽位图按模板出现顺序在后
    let images: Vec<AssetRef> = recipe
        .refs
        .iter()
        .chain(rendered.slot_images.iter())
        .cloned()
        .collect();
    if images.len() > profile.max_refs as usize {
        return Err(ServerFnError::new(format!(
            "图片总数超过上限 {} 张",
            profile.max_refs
        )));
    }
    let mode = if images.is_empty() { Mode::Gen } else { Mode::Edit };

    let run = Run {
        id: uuid::Uuid::new_v4().simple().to_string(),
        recipe_id: recipe.id.clone(),
        input_id: Some(input.id.clone()),
        recipe_version: recipe.version,
        provider_id: provider.id.clone(),
        model_id: recipe.model_id.clone(),
        mode,
        prompt: rendered.prompt.clone(),
        params: recipe.params.clone(),
        ref_count: images.len(),
        status: RunStatus::Running,
        error: None,
        images: vec![],
        usage: None,
        created_at: now_ms(),
        duration_ms: None,
    };
    crate::store::save_run(&run).await;

    let mut run_task = run.clone();
    tokio::spawn(async move {
        let t0 = std::time::Instant::now();
        match crate::adapter::execute(
            &provider,
            crate::adapter::ExecRequest {
                prompt: &rendered.prompt,
                params: &recipe.params,
                images: &images,
                mask: recipe.mask.as_ref(),
                model_id: &recipe.model_id,
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

    Ok(run)
}

#[server]
pub async fn rerun_run(run_id: String) -> Result<Run, ServerFnError> {
    let old = crate::store::get_run(&run_id)
        .await
        .ok_or_else(|| ServerFnError::new("批次不存在"))?;
    // 原样重跑：以批次快照关联的输入再执行一次
    // （若配方已修改，重跑使用的仍是输入的当前值；输入会先被渲染校验）
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
