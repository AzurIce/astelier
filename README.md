# Atelier · 生成式艺术工作台

dioxus-fullstack 实现的生成式图像创作工具。前身是纯静态实验 [poke-image-studio](poke-image-studio/)，
本项目的核心思路与其一致：**以模型元数据（params schema）驱动参数 UI 与请求转换**，
并把 API 调用移到服务端（无 CORS、密钥不出本机、结果可持久化）。

## 三层结构：配方 → 输入 → 批次

```
Recipe 配方（模板）        RecipeInput 输入（一次具体化）      Run 批次（自包含快照）
├─ prompt 模板（槽位）  ×N  ├─ 文字槽 / 图片槽的值        →   ├─ TemplateSnapshot 模板快照
├─ 参数（默认值）           ├─ 额外参考图                     ├─ InputSnapshot   输入快照
├─ 固定参考图 / mask        ├─ mask 覆盖（跟随/不用/自定义）   ├─ ResolvedRequest 最终请求
└─ 模型 / provider          └─ 参数覆盖（未覆盖 = 继承）      └─ 状态 / 结果图 / 用量
   version 随保存 +1           version 随保存 +1                 创建后永不改变
```

- **配方（模板）**是可复用的创作定义：prompt 模板（`{文字槽}` / `{img:图片槽}`、`{{` 转义）
  + 默认参数 + 固定参考图 + mask。显式保存，内容变化时 `version +1`。
- **输入**是模板的一次具体化：填变量、绑槽位图，还可以携带仅本次用的
  额外参考图、mask 覆盖、参数覆盖（合并顺序：配方参数 ← 输入覆盖）。
- **批次**在创建那一刻把「当时的模板」「当时的输入」「合并后的最终请求」
  三者完整物化进快照（`RunRequest`）。此后配方 / 输入怎么改，历史批次都不受影响：

  - **原样重跑 = 快照重放**：不回读配方与输入，直接再执行快照里的最终请求；
    输入被删、模板改版、甚至换了模型都能原样重放（新批次标记「重放」）。
  - **查看快照**：弹窗完整回看三段快照，并可「恢复模板为此快照」
    （载入草稿，保存后才 +1 成新版本，绝不静默改历史）或「复制为新输入」。
  - 旧版批次（快照机制之前创建）标记「旧版」，重跑回退为按当前配方 / 输入执行。

## 界面布局

```
┌────────────────────────────────────────────────────────────────┐
│ 顶栏   Atelier   [Provider ▾]  [☾]  [⚙]                         │
├──────────┬──────────────────────────────┬──────────────────────┤
│ 侧栏      │ 模板 ▸（可折叠，v5）          │ 此输入的批次          │
│ 分组      │   prompt 模板·参数·固定图·mask│  ✓ 模板v5 输入v2  [图]│
│ └ 配方    │ 输入「A」 v2                  │  ✓ 模板v5 输入v1  [图]│
│   ├ 输入A │   {subject}[__] {img:x}[chip] │  （独立滚动）         │
│   └ 输入B │   额外参考图 · mask · 参数覆盖 │                      │
│           │   最终请求预览 [保存][生成 ⌘↩] │                      │
└──────────┴──────────────────────────────┴──────────────────────┘
```

- **侧栏**：分组 > 配方 > 输入 树；首项「新建输入」是空白草稿。
- **左列（编辑）**：上方模板编辑带（默认收起，头部常显版本 / 未保存提示）；
  下方输入准备区——槽位表单、额外参考图、mask 三态（跟随配方 / 不使用 / 自定义）、
  参数覆盖（未覆盖灰显继承值，点击即覆盖、可还原），底部实时预览
  最终 prompt 与发送图片序列（编号 + 来源标记 配/输/槽）。
- **右列（批次）**：批次卡片带 `模板v / 输入v` 徽章；配方演进后显示过期提示点；
  操作：原样重跑 / 查看快照 / 复制为新输入 / 删除。
- 响应式：窗口 ≥1100px 双栏各自滚动；窄屏退化为 模板 → 输入 → 批次 纵向堆叠。
- 亮 / 暗双主题（右上角切换），未手动切换时跟随系统；Provider 与模型配置在「设置」弹窗管理。

## 元数据驱动的统一参数表示

不同 API / 模型接受的参数各不相同，但 UI 需要统一表达。做法：

1. **`ModelProfile`**（`src/profiles.rs`）描述一个模型的能力：
   `params: Vec<ParamDef>`（参数名 / API 字段名 / 控件类型 / 取值范围 / 所属模式）、
   `size_rule`（自定义尺寸校验规则）、透明底 / mask / 流式 / 参考图上限等能力开关。
2. **UI 参数区**由 `params` 动态渲染（`src/ui/params.rs`）：
   select → 分段控件（≤5 段）或下拉；number → 步进器；
   size → 预设比例 + 自定义 W×H（按 SizeRule 实时校验，如 16 整除 · 比例 1:3–3:1 · 上限 3840×2160）。
   首个「默认」段 = Unset = 不随请求发送。输入层同一套控件支持「继承 / 覆盖」双态。
3. **持久化的参数**存统一表示 `ParamMap`（统一键 → ParamValue，相邻标签 serde 格式），
   与具体协议无关。
4. **发送时**由 adapter（`src/adapter.rs`）按 `api` 种类转换成具体协议请求：
   - `OpenAiImages`：`POST /images/generations`（JSON）/ `POST /images/edits`（multipart，参考图 image[] + mask），
     响应里的 b64_json / url 统一落盘为本地资产。
   - 未来 seeddream / nano banana 等：各配一份档案（或 Provider 级 JSON 覆盖），
     字段名差异用 `api_key` 表达，结构差异在 adapter 加分支，UI 无需改动。

内置档案：`gpt-image-2`（任意分辨率、透明底预览、流式、mask 编辑）与
`gpt-image-2.5-sunburst / -flare`（画质独占 xhigh / max）。未识别模型走通用档案（以网关为准）。

## 持久化（服务端）

单用户本地工具，直接 JSON 落盘在 `data/`（已 gitignore）：

```
data/
├── config.json            # providers、active provider、模型元数据覆盖
├── groups.json            # 配方分组
├── assets/{id}.{ext}      # 不可变资产池（uuid 命名，经 /asset/{name} 路由伺服，永不 GC）
├── recipes/{rid}/
│   ├── recipe.json        # 配方当前态（模板 / 参数 / 固定图 / mask，version 随保存 +1）
│   ├── README.md          # 配方说明
│   └── inputs/{iid}.json  # 输入当前态（变量 / 槽位图 / 额外图 / mask / 参数覆盖）
└── runs/{run_id}.json     # 批次档案：状态 + 执行时刻的完整快照，可直接 jq 查看
```

- 资产只增不删，因此快照里的 `AssetRef` 引用永远有效。
- 旧版扁平存储（inputs.json / runs.json）首次启动自动迁移到目录结构，旧文件挪入 `data/legacy/`。
- 首次启动自动写入默认 Provider（OpenAI 官方 / Poke API）与默认模型列表。

## 开发环境（Nix flake）

```bash
direnv allow                      # 或手动 nix develop（仓库根即项目根）
```

dev shell 提供：rust stable（wasm32 target、rust-src、clippy、rust-analyzer）、
dioxus-cli、binaryen、lld、以及与本项目 wasm-bindgen 0.2.126 严格对齐的 wasm-bindgen-cli。

dx 与 dioxus 子 crate 版本必须严格一致——版本错位时 devserver 注入的
解释器 JS 与客户端协议不匹配，水合会直接崩溃。flake 里的 nixpkgs 输入
已更新到提供 dioxus-cli 0.7.10 的版本，Cargo.toml 中 facade 用
`=0.7.10` 锁死、子 crate 精确版本由 Cargo.lock 保证，三者对齐。

```bash
just serve        # nix develop -c dx serve --addr 127.0.0.1 --port 8230 --open false
just check        # server + wasm 双端 cargo check
```

> 已知环境约束：subsecond 热补丁（devtools）在本环境会把 devserver 的
> 「must rebuild」提示层留在页面上拦截点击，故 facade 关闭了 `devtools` 默认特性；
> dx 的文件监视仍会触发重建与整页刷新。

## 架构

```
src/
├── main.rs        # 入口：wasm → dioxus::launch；server → axum Router + SSR
├── model.rs       # 共享类型：ParamValue/ParamDef/ModelProfile/Recipe/RecipeInput/Run
│                  #   + merge_request / resolve_request（合并唯一来源）+ validate_request
├── profiles.rs    # 内置模型档案 + provider override 深合并 + 能力徽章推导
├── api.rs         #[server] 函数（配置/配方/输入/批次/上传/生成；start_run 快照化、rerun 重放）
├── store.rs     S # data/ JSON 持久化 + 资产存取 + /asset 路由 + 旧版迁移
├── adapter.rs   S # 统一表示 → OpenAI Images 协议；执行请求；解析结果
├── app.rs         # AppState（信号集合）、加载/轮询编排
└── ui/
    ├── recipe/    # mod(页面骨架+批次带) / template_band(模板编辑) / input_band(输入准备)
    ├── feed.rs    # 全局批次流 + RunCard（快照徽章与动作）
    ├── snapshot.rs# 批次快照弹窗（三段回看 + 重放/恢复模板/复制为新输入）
    └── …          # sidebar / params(含参数覆盖控件) / settings / lightbox
                   # widgets(Modal/Dropdown/Segmented/Stepper/Toast/RefsStrip) / icons / theme
S = #[cfg(feature = "server")] 独占
```

生成流程：输入准备区「生成」→ 前端 `resolve_request` + `validate_request` 预校验 →
`start_run` server fn（落库输入并推进版本 → 解析合并 → 构造 `RunRequest` 快照落盘 →
tokio spawn 异步执行）→ adapter 发请求 → 结果落盘 → Run 置 Done/Error
→ 客户端 1.2s 轮询 `list_runs` 刷新批次流。

## 路线

- [ ] 批次流式（partial_images 渐进预览，server → 客户端 SSE/流式 server fn）
- [ ] mask 画板（画刷/反相/撤销，替代当前的 PNG 上传）
- [ ] 模板版本浏览器（跨批次对比模板快照 / 一键回滚列表）
- [ ] 更多 adapter：seedream / nano banana（结构化差异转换）
- [ ] 从 `/v1/models` 拉取模型列表
- [ ] 画布：基于 dioxus-flow 的节点式编排，把「输入 → 生成 → 编辑」连成图
