# Atelier · 生成式艺术工作台

基于 Svelte 5 + Rete.js 2 的节点图创作工具。Model / Prompt / Image / Generate /
Preview 节点在浏览器中完成参数编辑、输入装配与拓扑执行；当前 Rust/axum 服务
提供文件存储、生图协议适配和静态页面托管。

Graph 保存节点、参数和连线，View 保存布局和视口。节点的 busy、error 和生成
输出通过 Svelte 响应式字段更新，只保留在当前会话中。生成接口直接返回临时图片，
不建立 Run 档案、不自动归档参考图或生成结果；用户把满意的图片拖入库时才收藏。

目前继续自动保存图文档和布局，保存失败显示“保存失败”，切图等待待保存及进行中
的写入完成。临时生成图片可供下游节点使用，或拖入 Image 节点继续编辑；这些临时
引用不随图文档保存，刷新或重新打开图后不恢复。

OPFS 与浏览器直连 provider 的迁移计划见 [todo.md](todo.md)。当前仍需 Rust 服务，
本次改动没有提前迁移存储或 API key 配置。历史静态实验见
[poke-image-studio](poke-image-studio/)。

## 架构

```text
web/（bun + Vite + Svelte 5 + Rete.js 2）
  src/nodes/classes.svelte.ts  节点身份、响应式业务字段与会话运行状态
  src/editor.ts               Rete 插件、连线约束、视口与统一 socket 布局观察
  src/exec.ts                 使用输入快照执行图，结果回写原节点
  src/graphDoc.ts             Graph/View 的显式序列化与恢复
  src/graphStore.svelte.ts     共享当前图 ID/标题/保存状态，切图与改名
  src/saveQueue.ts             合并修改、串行保存、失败保留与 flush 等待
  src/components/LibraryDock.svelte  图片库与显式收藏
        │ fetch /api/*（开发时 Vite proxy → 127.0.0.1:8230）
        ▼
src/（cargo，axum）
  api.rs       图、分组、图片库、配置与直接生成接口
  adapter.rs   generations JSON / edits multipart，返回全部图片 data URL
  profiles.rs  模型档案与 override 合并
  store.rs     JSON、图内参考图和库文件存储
  main.rs      API、图片路由和 web/dist 托管
```

生成响应中的每张图片都保留在 Generate 节点中，可单独预览、拖入 Image 节点或
图片库；下游生成接收全部图片，Preview 展示第一张。执行中切图或删除原节点时，
晚返回的结果不会写入新图中的节点。

数据目录可用 `ATELIER_DATA_DIR` 覆盖：

```text
data/
├── config.json              Provider 配置（支持 env:VAR 凭证引用）
├── groups.json              图分组
├── graphs/{gid}/graph.json  节点与参数
├── graphs/{gid}/view.json   位置与视口
├── graphs/{gid}/store/      已导入参考图
├── stores/                  用户显式收藏的图片库
└── assets/                  显式资产上传及已有文件
```

生成过程不写入 `runs/` 或 `assets/`。已有历史文件不再用于运行记录或结果恢复；
本次改动没有删除用户磁盘上的旧文件。显式资产上传与已有 `/asset/` 文件读取仍保留。

## API

| 端点 | 说明 |
|---|---|
| `GET/PUT /api/config` | Provider 配置 |
| `GET /api/providers/{id}/profiles` | 模型档案 |
| `PUT /api/providers/{id}/models/{model}/override` | 模型档案覆盖 |
| `GET/POST /api/graphs`、`GET/PUT/DELETE /api/graphs/{id}` | 图管理，新图包含最小生成管线 |
| `GET/PUT /api/graphs/{id}/view` | 布局与视口 |
| `PUT /api/graphs/{id}/title`、`PUT /api/graphs/{id}/group` | 图重命名与移动 |
| `GET/POST /api/groups`、`PATCH/DELETE /api/groups/{id}`、`PATCH /api/groups/{id}/parent` | 图分组管理 |
| `GET/POST /api/stores`、`POST /api/stores/dirs`、`PATCH/DELETE /api/stores/{path}` | 图片库 |
| `GET/POST /api/graphs/{id}/store`、`DELETE /api/graphs/{id}/store/{name}` | 图内参考图 |
| `POST /api/assets?filename=…` | 显式资产上传 |
| `POST /api/generate` | 直接等待上游，返回 `{ imageUrls: string[], usage? }`，图片为 data URL |
| `GET /asset/{name}`、`GET /gstore/{gid}/{name}`、`GET /store/{path}` | 已保存图片读取 |

`/api/generate` 接收 `{ model, prompt, params?, imageUrls? }`。`model` 支持
`provider:model`；参考图支持已保存图片路径和临时 data URL。参考图读取与结果
解码在请求内完成，不为历史重放复制文件。`/api/runs` 和重跑接口已移除。

## 开发与验证

```sh
nix develop
just serve       # cargo run --release → http://127.0.0.1:8230
just dev-web     # Vite 开发服务
just build-web   # 构建前端
cargo test       # 参数、直接生成、存储错误与 HTTP 回归
cd web
bun run test     # 图片导入与保存竞态回归
bun run check
bun run build
```

端口用 `ATLIER_ADDR` 覆盖。前端依赖由 `web/bun.lock` 管理。

浏览器集成检查使用真实 Svelte/Rete 与模拟应用 API，覆盖响应式字段、临时图片、
显式收藏、切图保存和晚返回结果，不调用真实 provider。先启动 Vite，再使用已安装
的 Chromium 与 Playwright/Playwright Core 运行：

```sh
# 在 web/ 下运行；模块路径可指向仓库外安装的 playwright-core
PLAYWRIGHT_MODULE=/path/to/playwright-core/index.mjs \
CHROMIUM_PATH=chromium \
ATELIER_TEST_URL=http://127.0.0.1:5173 \
bun run test:browser
```

实际 provider 的跨域探测单独见 [CORS spike](spikes/cors/README.md)。
