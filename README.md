# Atelier · 生成式艺术工作台

基于 Svelte 5 + Rete.js 2 的节点图创作工具。Model / Prompt / Image / Generate /
Preview 节点在浏览器中完成参数编辑、输入装配与拓扑执行。

**工作区可选，前端零必需后端：**

- **本地工作区**：数据存浏览器 OPFS，生图由浏览器直连 provider
  （OpenAI Images 兼容协议）。`web/dist` 部署到任意静态 HTTPS 站点即可使用。
- **远端工作区**：顶栏填入 server URL（本仓库 Rust/axum 服务或任何兼容
  实现），数据与生图代理都在服务端，密钥留在服务端进程。

两种模式共享同一套前端；`WorkspaceStore` / `ImageGenerator` 两个抽象边界
下的实现可独立替换。未来 SaaS（账号 + 数据同步）将在远端工作区基础上
演进：服务端加鉴权与按用户存储，前端零改动接入。

Graph 保存节点、参数和连线，View 保存布局和视口（自动保存，失败显示
「保存失败」并阻塞切图）。节点的 busy、error 与生成输出是 Svelte 响应式
会话状态，不建运行档案、不自动归档；生成结果作为临时图片留在会话中供
下游与拖拽，拖入库才显式保存。

## 架构

```text
web/（bun + Vite + Svelte 5 + Rete.js 2）
  src/app/           应用装配、顶栏（工作区切换 / 设置）、Provider 设置
  src/canvas/        Rete 编辑器、节点、执行、当前图会话与保存队列
  src/workspace/     WorkspaceStore 接口、工作区选择、持久文档类型与侧栏
    opfs/            本地实现（fs 原语 / 路径校验 / 对象 URL / zip 导入导出）
    httpStore.ts     远端实现（/api/* 参数化 baseUrl）
  src/generation/    ImageGenerator 接口、直连实现、HTTP 代理实现、
                     模型档案与协议纯逻辑（profiles / protocol）、参数表
  src/images/        图片引用、拖拽协议、导入队列、内容指纹、字节嗅探
  src/library/       图片库面板、库 API 与路径处理
  src/ui/            通用控件、主题、确认框、消息与预览
        │ main.ts 按 localStorage 选择的工作区装配
        ├─ 本地：createOpfsStore + createDirectGenerator（浏览器直连）
        └─ 远端：createHttpStore(baseUrl) + createHttpGenerator(baseUrl)
                │ fetch {base}/api/*（服务端需允许跨域）
                ▼
src/（cargo，axum · 远端工作区服务端）
  api.rs       图、分组、图片库、配置与直接生成接口
  adapter.rs   generations JSON / edits multipart，返回全部图片 data URL
  profiles.rs  模型档案与 override 合并
  store.rs     JSON、图内参考图和库文件存储
  main.rs      API、图片路由、CORS 层与 web/dist 托管
```

前端目录与依赖规则见 [web/README.md](web/README.md)。

生成响应中的每张图片都保留在 Generate 节点中，可单独预览、拖入 Image
节点或图片库；下游生成接收全部图片，Preview 展示第一张。执行中切图或
删除原节点时，晚返回的结果不会写入新图中的节点。

### 本地工作区数据（OPFS，站点私有）

```text
atelier/
├── config.json              Provider 配置（API key 明文，设置里可清除）
├── groups.json              图分组
├── graphs/{uuid}/graph.json 节点与参数（id 稳定，重命名不改身份）
├── graphs/{uuid}/view.json  位置与视口
├── graphs/{uuid}/store/     图内参考图
└── stores/                  用户显式收藏的图片库
```

图身份是稳定 UUID（同步友好）；图片引用保存文件名/路径，展示 URL
（blob:）一律运行时解析。跨标签页写经 Web Locks 串行化；启动申请
`navigator.storage.persist()`，设置里可查看配额。**清除站点数据会删除
本地工作区**——换设备 / 备份用设置里的「导入 / 导出 zip」（图按
updated_at last-writer-wins 合并；Provider 密钥不随 zip 转移）。

### 远端服务端数据（ATELIER_DATA_DIR 可覆盖）

```text
data/
├── config.json              Provider 配置（支持 env:VAR 凭证引用）
├── groups.json              图分组
├── graphs/{gid}/…           图文档与图内参考图（id = 目录名）
├── stores/                  图片库
└── assets/                  历史遗留（仍可作生成输入）
```

## API（远端工作区协议）

| 端点 | 说明 |
|---|---|
| `GET/PUT /api/config` | Provider 配置 |
| `GET/POST /api/graphs`、`GET/PUT/DELETE /api/graphs/{id}` | 图管理，新图包含最小生成管线 |
| `GET/PUT /api/graphs/{id}/view` | 布局与视口 |
| `PUT /api/graphs/{id}/title`、`PUT /api/graphs/{id}/group` | 图重命名与移动 |
| `GET/POST /api/groups`、`PATCH/DELETE /api/groups/{id}`、`PATCH /api/groups/{id}/parent` | 图分组管理 |
| `GET/POST /api/stores`、`POST /api/stores/dirs`、`PATCH/DELETE /api/stores/{path}` | 图片库 |
| `GET/POST /api/graphs/{id}/store`、`DELETE /api/graphs/{id}/store/{name}` | 图内参考图 |
| `POST /api/generate` | 直接等待上游，返回 `{ imageUrls: string[], usage? }`，图片为 data URL |
| `GET /gstore/{gid}/{name}`、`GET /store/{path}`、`GET /asset/{name}` | 已保存图片读取 |

`POST /api/generate` 接收 `{ model, prompt, params?, imageUrls? }`。`model`
支持 `provider:model`；参考图支持已保存图片路径和临时 data URL。

**CORS**：服务端默认放开全部来源（单用户本地工具、无鉴权，CORS 不构成
额外暴露），供纯静态部署的前端跨源选用；暴露公网时用
`ATELIER_CORS_ORIGINS="https://a,https://b"` 收紧。

## 开发与验证

```sh
nix develop
just dev-web     # Vite 开发服务（本地工作区，无需任何后端）
just serve       # 可选：cargo run --release 启动远端工作区服务端
just build-web   # 构建前端（纯静态产物）
cargo test       # 服务端回归（15 项）
cd web
bun run test     # 前端单测（45 项：路径/嗅探/档案/协议/导入/保存/传输）
bun run check
bun run build
```

浏览器集成检查使用真实 Svelte/Rete：本地模式走真实 OPFS（生图器为替身、
保存故障注入），远端模式拦截绝对 URL mock 服务；不调用真实 provider：

```sh
# 在 web/ 下运行；模块路径可指向仓库外安装的 playwright-core
PLAYWRIGHT_MODULE=/path/to/playwright-core/index.mjs \
CHROMIUM_PATH=chromium \
ATELIER_TEST_URL=http://127.0.0.1:5173 \
bun run test:browser
```

实际 provider 的跨域探测见 [CORS spike](spikes/cors/README.md)；浏览器直连
的协议纯逻辑与错误提取已由单测和免费网关探测（无效密钥 401 可读）覆盖。

历史静态实验见 [poke-image-studio](poke-image-studio/)。迁移前的 Run
模型与运行历史已删除；磁盘上旧的 `data/runs/` 等目录不再读取。
