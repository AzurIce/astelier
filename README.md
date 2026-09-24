# Atelier · 生成式艺术工作台

**rust server + React/tldraw 前端** 的生成式图像创作工具。画布基于
[tldraw image-pipeline](https://tldraw.dev/starter-kits/image-pipeline)
模板（节点式图片流水线：类型化端口、DAG 执行引擎在浏览器端），后端 axum
提供 REST API 并托管前端产物。前身是 dioxus fullstack 实现（配方/输入时代
的完整说明见 git 历史）与更早的静态实验 [poke-image-studio](poke-image-studio/)。

核心思路不变：**以模型元数据（`ModelProfile` params schema）驱动参数与请求
转换**，API 调用全部在服务端（无 CORS、密钥不出本机、结果可持久化）；每次
生成落一条 Run 档案（归档最终请求，此后画布怎么改都不影响该批次，可原样重放）。
规划中：ComfyUI 以 **workflow 级 provider** 接入（整段 workflow 当一个节点调
用，不做节点级透传）。

## 架构

```
web/（bun + Vite + React 19 + tldraw 5）
  节点图前端：Generate / LoadImage / Preview… 节点，端口类型检查、DAG 执行
        │ fetch /api/*（dev 时 vite proxy → 127.0.0.1:8230）
        ▼
src/（cargo，axum）
  api.rs        REST handlers（config / graphs / groups / assets / runs / generate）
  adapter.rs    协议适配（OpenAI Images 兼容：文生图 generations，带图 edits）
  profiles.rs   内置模型档案 + override 合并（schema 驱动，接新模型不改代码）
  store.rs      data/ 落盘（JSON + 资产文件，tmp+rename 原子写）
  main.rs       路由 + 托管 web/dist（SPA fallback 到 index.html）
```

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

## 路线

- [ ] 前端接回：模型选择（Model 节点 ← /api/providers/*/profiles）、批次 Feed、设置页
- [ ] 图持久化：tldraw 画布 ↔ /api/graphs（目前画布存浏览器 IndexedDB）
- [ ] mask 节点（画刷/反相/撤销）
- [ ] 更多 adapter：seeddream / nano banana（结构化差异转换）
- [ ] ComfyUI provider（workflow 级）
- [ ] 从 `/v1/models` 拉取模型列表
