# Atelier · 生成式艺术工作台

**rust server + Svelte/Rete.js 前端** 的生成式图像创作工具。画布基于
[Rete.js](https://retejs.org)（MIT，全插件免商用限制）+ Svelte 5 自建节点系统
（Model / Prompt / Image / Generate / Preview，端口类型检查、拓扑执行在浏览器端），
后端 axum 提供 REST API 并托管前端产物。前身依次是 dioxus fullstack（配方/输入
时代）与 React + tldraw image-pipeline（因 tldraw 5.x 许可改为生产付费而弃用），
完整说明见 git 历史与更早的静态实验 [poke-image-studio](poke-image-studio/)。

核心思路不变：**以模型元数据（`ModelProfile` params schema）驱动参数与请求
转换**，API 调用全部在服务端（无 CORS、密钥不出本机、结果可持久化）；每次
生成落一条 Run 档案（归档最终请求，此后画布怎么改都不影响该批次，可原样重放）。
规划中：ComfyUI 以 **workflow 级 provider** 接入（整段 workflow 当一个节点调
用，不做节点级透传）。

## 架构

```
web/（bun + Vite + Svelte 5 + Rete.js 2）
  src/editor.ts     Area/Svelte/Connection 插件组装 + 连线类型约束 + 视口控制（zoom/fit）
  src/nodes/        节点类（ClassicPreset 派生）+ Svelte 节点组件 + 自绘连线（ConnectionLine）
  src/exec.ts       全图拓扑求值：Model/Prompt/Image → Generate → Preview
  src/graphStore.ts 画布持久化：结构/表现文档分别节流保存到服务端
  src/design/       设计令牌：三套皮肤（工作台/晨雾/素白）× light/dark，运行时注入 --ui-*
  src/components/   UI 基础件：Button/IconButton/Popover/Toast/Dialog/Lightbox
  src/chrome/       外壳：Topbar（图名/皮肤/明暗/Run）、SkinSwitcher、ViewportControls
        │ fetch /api/*（dev 时 vite proxy → 127.0.0.1:8230）
        ▼
src/（cargo，axum）
  api.rs        REST handlers（config / graphs / groups / assets / runs / generate）
  adapter.rs    协议适配（OpenAI Images 兼容：文生图 generations，带图 edits）
  profiles.rs   内置模型档案 + override 合并（schema 驱动，接新模型不改代码）
  store.rs      data/ 落盘（JSON + 资产文件，tmp+rename 原子写）
  main.rs       路由 + 托管 web/dist（SPA fallback 到 index.html）
```

注：Rete.js 仅 `rete-structures` / `rete-scopes-plugin` 两个包为 CC-BY-NC-SA
（禁商用），本项目不使用这两个包，其余全部 MIT。

数据目录（`ATELIER_DATA_DIR` 可覆盖，多实例隔离用）：

```
data/
├── config.json            Provider 配置（api_key 支持 env:VAR 引用，不落密钥）
├── groups.json            节点图分组
├── assets/{id}.{ext}      全部图片资产（只增不删，引用永远有效）
├── graphs/{gid}/graph.json
└── runs/{run_id}.json     批次档案：最终请求归档 + 状态 + 结果图 + 用量
```

## API

| 端点 | 说明 |
|---|---|
| `GET/PUT /api/config` | Provider 配置 |
| `GET /api/providers/{id}/profiles` | 模型档案（params schema，内置 + override 合并） |
| `GET/POST /api/graphs`、`GET/PUT/DELETE /api/graphs/{id}` | 节点图 CRUD（POST 种入「生图→显示」闭环） |
| `GET/POST /api/groups`、`PATCH/DELETE /api/groups/{id}` | 分组 |
| `POST /api/assets?filename=…` | 上传图片（原始字节体） |
| `GET/POST /api/runs`、`GET/DELETE /api/runs/{id}`、`POST /api/runs/{id}/rerun` | 批次：起跑 / 轮询 / 原样重放 |
| `POST /api/generate` | 节点图前端的同步封装：落 Run → 轮询完成 → `{imageUrl, seed}`；`model` 支持 `provider:model`，参考图收 `/asset/…` 与 data URL |
| `GET /asset/{name}` | 资产文件（immutable 缓存） |

## 开发

```sh
nix develop     # rust stable + bun
just serve      # cargo run --release → http://127.0.0.1:8230（托管 web/dist）
just dev-web    # cd web && bun run dev（vite，/api、/asset 已代理）
just build-web  # cd web && bun install && bun run build
just check      # cargo check
```

端口用 `ATLIER_ADDR` 覆盖。前端依赖 bun 管理（`web/bun.lock`）。

Image 节点支持多选上传、从图片库多选拖入，以及画布图片之间的拖入。
点击图片查看大图；拖动左上角序号调整参考图的输出顺序，聚焦序号后也可用
方向键、Home / End 排序或 Delete 移除。连续添加会排队，重复内容自动跳过，
单张导入失败不会隐藏或丢失其他图片，并可在节点内重试。参考图数量上限由
所连接的模型校验。运行 `cd web && bun run test` 可验证导入与拖拽载荷逻辑。

## 路线

- [ ] Generate 参数区接 profiles schema（steps/cfg/seed 之外的模型参数动态渲染）
- [ ] 图持久化：localStorage → /api/graphs（服务端已有 CRUD）
- [ ] 批次 Feed（/api/runs 历史、重放、重跑）与设置页
- [ ] 节点补充：Negative Prompt、多图 ref（走 /api/runs）、Upscale 等
- [ ] mask 节点（画刷/反相/撤销）
- [ ] 更多 adapter：seeddream / nano banana（结构化差异转换）
- [ ] ComfyUI provider（workflow 级）
- [ ] 从 `/v1/models` 拉取模型列表
