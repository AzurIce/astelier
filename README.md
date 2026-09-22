# Atelier · 生成式艺术工作台

dioxus-fullstack 实现的生成式图像创作工具。前身是纯静态实验 [poke-image-studio](poke-image-studio/)，
本项目的核心思路与其一致：**以模型元数据（params schema）驱动参数 UI 与请求转换**，
并把 API 调用移到服务端（无 CORS、密钥不出本机、结果可持久化）。

## 界面布局

```
┌──────────────────────────────────────────────────────────┐
│ 顶栏   Atelier          [Provider ▾]  [☾]  [⚙]           │
├──────────┬───────────────────────────────────────────────┤
│ 输入列表  │   批次画廊（每次生成 = 一张批次卡片）          │
│ (持久化)  │   · 图片网格 / 生成中 shimmer / 错误与重跑     │
│          │   · 放大（灯箱）· 用作参考 · 下载               │
│          ├───────────────────────────────────────────────┤
│          │   底部 Composer：参考图 · Prompt · 参数 · 生成  │
└──────────┴───────────────────────────────────────────────┘
```

- **左侧**：持久化的「输入」——一段 Prompt + 参数快照 + 参考图，可反复修改、反复生成。
- **中间**：批次（Run）画廊，按时间倒序；生成中有 shimmer 占位，完成后可放大/下载/「用作参考」。
- **底部**：模型选择、参考图/Mask、Prompt、以及由模型元数据动态生成的参数区。
  快捷键 ⌘/Ctrl + Enter 生成。
- 亮 / 暗双主题（右上角切换），未手动切换时跟随系统；Provider 与模型配置在「设置」弹窗管理。

## 元数据驱动的统一参数表示

不同 API / 模型接受的参数各不相同，但 UI 需要统一表达。做法：

1. **`ModelProfile`**（`src/profiles.rs`）描述一个模型的能力：
   `params: Vec<ParamDef>`（参数名 / API 字段名 / 控件类型 / 取值范围 / 所属模式）、
   `size_rule`（自定义尺寸校验规则）、透明底 / mask / 流式 / 参考图上限等能力开关。
2. **UI 参数区**由 `params` 动态渲染（`src/ui/params.rs`）：
   select → 分段控件（≤5 段）或下拉；number → 步进器；
   size → 预设比例 + 自定义 W×H（按 SizeRule 实时校验，如 16 整除 · 比例 1:3–3:1 · 上限 3840×2160）。
   首个「默认」段 = Unset = 不随请求发送。
3. **持久化的输入**存统一表示 `ParamMap`（统一键 → ParamValue），与具体协议无关。
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
├── config.json    # providers、active provider、模型元数据覆盖
├── inputs.json    # 持久化输入
├── runs.json      # 生成批次（状态 / 用量 / 错误）
└── assets/        # 参考图、mask、生成结果（uuid 命名，经 /asset/{name} 路由伺服）
```

首次启动自动写入默认 Provider（OpenAI 官方 / Poke API）与默认模型列表。

## 开发环境（Nix flake）

```bash
direnv allow                      # 或手动 nix develop（仓库根即项目根）
```

dev shell 提供：rust stable（wasm32 target、rust-src、clippy、rust-analyzer）、
dioxus-cli、binaryen、lld、以及与本项目 wasm-bindgen 0.2.126 严格对齐的 wasm-bindgen-cli。

`tools/bin/dx` 是 [官方 v0.7.10 预编译二进制](https://github.com/DioxusLabs/dioxus/releases)——
nixpkgs 的 dioxus-cli 落后于 crates.io 版本，而 dx 与 dioxus 子 crate 版本必须一致
（否则 devserver 注入的解释器 JS 与客户端协议错位，水合时崩溃）。
Cargo.toml 中 facade 用 `=0.7.10` 锁死，子 crate 精确版本由 Cargo.lock 保证。

```bash
just serve        # nix develop -c tools/bin/dx serve --addr 127.0.0.1 --port 8230 --open false
just check        # server + wasm 双端 cargo check
```

> 已知环境约束：subsecond 热补丁（devtools）在本环境会把 devserver 的
> 「must rebuild」提示层留在页面上拦截点击，故 facade 关闭了 `devtools` 默认特性；
> dx 的文件监视仍会触发重建与整页刷新。

## 架构

```
src/
├── main.rs        # 入口：wasm → dioxus::launch；server → axum Router + SSR
├── model.rs       # 共享类型：ParamDef/ParamValue/ModelProfile/Input/Run + 校验
├── profiles.rs    # 内置模型档案 + provider override 深合并 + 能力徽章推导
├── api.rs         #[server] 函数（配置/输入/批次/上传/生成）
├── store.rs     S # data/ JSON 持久化 + 资产存取 + /asset 路由
├── adapter.rs   S # 统一表示 → OpenAI Images 协议；执行请求；解析结果
├── app.rs         # AppState（信号集合）、加载/轮询编排
└── ui/            # topbar / sidebar / feed / composer / params / settings / lightbox
                   # widgets(Modal/Dropdown/Segmented/Stepper/Toast) / icons / theme
S = #[cfg(feature = "server")] 独占
```

生成流程：Composer「生成」→ 前端 `validate_request` 预校验 → `start_run` server fn
（落库 Running + tokio spawn 异步执行）→ adapter 发请求 → 结果落盘 → Run 置 Done/Error
→ 客户端 1.2s 轮询 `list_runs` 刷新画廊。

## 路线

- [ ] 批次流式（partial_images 渐进预览，server → 客户端 SSE/流式 server fn）
- [ ] mask 画板（画刷/反相/撤销，替代当前的 PNG 上传）
- [ ] 更多 adapter：seedream / nano banana（结构化差异转换）
- [ ] 从 `/v1/models` 拉取模型列表
- [ ] 画布：基于 dioxus-flow 的节点式编排，把「输入 → 生成 → 编辑」连成图
