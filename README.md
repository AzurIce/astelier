# Astelier · 生成式艺术工作台

Svelte 5 + Rete.js 2 节点图创作工具。Model / Prompt / Image / Generate / Preview
节点在浏览器中完成参数编辑、参考图装配与拓扑执行。前端可以独立部署为静态 HTTPS 站点。

## 多后端与共享资源

默认挂载本地 OPFS，可在「添加 / 管理」中添加多个 Rust Server。左侧图树与底部
图片库按后端分根展示，各自可折叠。打开图不刷新页面，其他后端的资源继续可用。

- 图身份为 `{ backendId, id }`；图片路径在所属后端内有效。同名图、图片和 Provider
  不会相互覆盖。OPFS 和服务器新图都使用稳定 UUID，重命名只改标题。
- Server 的身份保存在 `data/backend.json`，通过 `GET /api/backend` 发现；不同客户端
  连接同一个服务会获得相同 ID，使图中 Remote Provider 引用能跨客户端解析。
  同一服务通过多个地址接入时，只挂载一次。连接列表与显示名存在设备 localStorage。
- 每个后端独立加载和报告错误，可以刷新 / 重连。已连接服务断线保留来源和 Provider
  选择；移除连接不删除服务器数据。移除当前图所属连接前会排空保存并打开本地图。
- 图片可以跨后端拖入任意图，复制进目标图内存储；库之间拖拽同源时移动，跨源时复制
  （包括目录）。生成结果可拖入任意后端的图片库收藏。
- 挂载不执行自动同步。账号、权限与云端数据隔离尚未实现；未来云后端使用同一套资源
  展示、Provider 发现和请求接口。

## Local / Remote Provider

Provider 与图的存储位置独立：本地图可以使用服务器 Provider，服务器图也可以使用
浏览器 Provider。Model 节点按来源分组选择，并保存 `providerBackendId`、`provider`、
`modelId`。新图明确保存模型来源，执行不会推断或回落到其他后端。

| 类型 | 配置 / 密钥位置 | 上游执行位置 |
| --- | --- | --- |
| Local Provider | 设置界面，本地 OPFS `config.json` | 浏览器直连 |
| Remote Provider | Server `config.json` | 对应服务器 |

`GET /api/providers` 只返回 Provider ID、名称、模型与参数档案；不返回凭证或上游
地址。参数控件由所选 Provider 的模型档案派生。服务器执行使用
`POST /api/providers/{provider_id}/generate`，不依赖图所属后端，也不会回落到其他
Provider。Server 配置通过文件管理。

浏览器在提交时固定图会话、节点、Provider 执行器、参数与参考图快照。参考图解析成
实际图片数据，远端请求携带 data URL，不要求服务器读取浏览器 blob URL 或其他
服务器的图片地址。晚返回结果只能写入原会话原节点。

Local Provider 使用 OpenAI Images 兼容协议：文生图 JSON → `/images/generations`，
带参考图 multipart → `/images/edits`。结果支持 base64 和图片 URL，保留全部图片。
浏览器直连及 URL 型结果下载需要上游允许跨域；完整真实付费链路仍待手动验收。

## 保存语义与 OPFS

Graph 保存节点、参数、连线；View 保存布局和视口。自动保存失败显示「保存失败」并
阻止切图。busy、error 和生成输出只属于会话；拖入图片库才持久保存产物，不建立 Run
档案，也不在保存图文档时自动归档生成图片。

```text
astelier/                      浏览器站点私有 OPFS
├── config.json                Local Provider 配置与明文 key
├── groups.json
├── graphs/{uuid}/
│   ├── graph.json
│   ├── view.json
│   └── store/                  图内参考图
└── stores/                    显式收藏的图片库
```

持久文档不保存展示用 blob URL。图片 URL 在运行时解析并缓存，删除 / 覆盖 / 移动时
释放。本地读改写经 Web Locks 串行化；启动申请持久存储，设置显示使用量和配额。
清除站点数据会删除本地工作区。

设置中的 zip 导出包含图、View、分组、图内参考图、库图片及空目录；**不包含 Local
Provider 配置与密钥**。导入图按 `updated_at` 合并，分组按 ID 并集，库按路径覆盖。
图内参考图恢复和完整 OPFS 往返已由真实浏览器测试覆盖。

## GitHub Pages 部署

[Pages workflow](.github/workflows/pages.yml) 在推送到 `main` 时部署，也可以在 Actions
页面手动触发。先在仓库 **Settings → Pages → Build and deployment → Source** 选择
**GitHub Actions**。仓库套餐需要支持当前仓库可见性的 Pages 托管。

Workflow 使用 Node 24 与 Bun 1.4.2，按锁文件安装依赖，运行类型检查和单元测试，
再构建、上传 `web/dist` 并部署。资源基础路径由 Pages 配置提供，适用于仓库子路径、
用户主页和自定义域名；本地开发与 Rust 静态托管默认仍使用 `/`。

部署产物只包含前端，不包含 `data/`、Provider 配置或 Rust 服务。页面使用自己的
OPFS 工作区，也可以在后端管理中连接已有 Rust Server。Pages 使用 HTTPS，公网
Server 也应提供 HTTPS 并允许站点来源的 CORS；可用 `ASTELIER_CORS_ORIGINS` 配置。
本机 / 局域网服务的访问还取决于浏览器的本地网络权限。

## Server 配置

```text
data/                         ASTELIER_DATA_DIR 可覆盖
├── backend.json               自动创建的稳定后端身份
├── config.json                Server Provider 配置
├── groups.json
├── graphs/{uuid}/…
├── stores/
└── assets/                    已有上传资产
```

`config.json` 示例：

```json
{
  "active_provider": "team",
  "providers": [
    {
      "id": "team",
      "name": "团队 Provider",
      "base_url": "https://provider.example.com/v1",
      "api_key": "TEAM_IMAGE_KEY_2",
      "models": ["gpt-image-2"],
      "overrides": {}
    }
  ]
}
```

`api_key` 可直接填环境变量名：`TEAM_IMAGE_KEY_2`，在每次请求时从服务端进程环境读取。
标识符形式（字母 / 下划线开头，随后字母、数字或下划线）按变量名处理；未设置或为空
会返回明确错误。字面密钥如 `sk-…` 也支持；已有 `env:VAR` 显式引用仍可读取。解析后的
密钥不写回配置，不通过发现接口返回。

```sh
export TEAM_IMAGE_KEY_2='your-key'
ASTELIER_DATA_DIR=/path/to/data ASTELIER_ADDR=127.0.0.1:8230 just serve
```

服务默认允许跨域；可通过 `ASTELIER_CORS_ORIGINS="https://a,https://b"` 限定来源。
当前 Rust 服务面向单用户，未加入账号鉴权。

## Server API

| 端点 | 说明 |
| --- | --- |
| `GET /api/backend` | 稳定实例身份与能力声明 |
| `GET /api/providers` | 不含凭证的 Provider / 模型档案列表 |
| `POST /api/providers/{provider_id}/generate` | 指定 Provider 执行，返回 `{ imageUrls, usage? }` |
| `GET/POST /api/graphs`、`GET/PUT/DELETE /api/graphs/{id}` | 图管理，新图包含最小管线 |
| `GET/PUT /api/graphs/{id}/view` | 布局与视口 |
| `PUT /api/graphs/{id}/title`、`PUT /api/graphs/{id}/group` | 重命名与分组移动 |
| `GET/POST /api/groups`、`PATCH/DELETE /api/groups/{id}`、`PATCH /api/groups/{id}/parent` | 分组管理 |
| `GET/POST /api/stores`、`POST /api/stores/dirs`、`PATCH/DELETE /api/stores/{path}` | 图片库 |
| `GET/POST /api/graphs/{id}/store`、`DELETE /api/graphs/{id}/store/{name}` | 图内参考图 |
| `GET /gstore/{gid}/{name}`、`GET /store/{path}`、`GET /asset/{name}` | 图片读取 |

生成负载为 `{ model, prompt, params?, imageUrls? }`，model 是模型原始 ID，Provider
通过 URL 明确指定。响应图片为会话 data URL。可覆盖的路径图片使用重新验证缓存策略。

## 架构与验证

前端模块与依赖规则见 [web/README.md](web/README.md)。

```sh
nix develop
just dev-web                 # 纯前端，无需 Server
just build-web               # 静态产物 web/dist
just serve                   # 可选 Server
cargo test
cargo build                  # 浏览器测试启动真实 Server 所需
cd web
bun run test
bun run check
bun run build
```

浏览器测试需要运行 Vite 开发服务与 Chromium，允许从仓库外提供 playwright-core：

```sh
PLAYWRIGHT_MODULE=/path/to/playwright-core/index.mjs \
CHROMIUM_PATH=/path/to/chromium \
ASTELIER_TEST_URL=http://127.0.0.1:5173 \
bun run test:browser
```

`state-runtime.mjs` 用真实 OPFS 与替身生图器验证响应式、自动保存、收藏与失败时序。
`backends.mjs` 自动启动两套真实 Rust 服务（独立临时数据目录）及受控生图上游，验证
多来源同名资源、Local / Remote 交叉执行、环境密钥、跨源参考图与库复制、会话隔离、
连接恢复 / 移除 / 离线，以及真实 OPFS zip 往返。可用 `ASTELIER_SERVER_BIN` 指定服务端
二进制；测试不访问外部 provider，不使用用户数据或真实密钥。

## 命名与数据格式

项目名为 **Astelier**（`astelier`）。二进制、crate、浏览器存储命名空间和 zip 导出
根目录均使用 `astelier`；服务环境变量使用 `ASTELIER_*`，图片拖拽类型使用
`application/x-astelier-*`。应用只读取当前格式，不执行旧名称或旧文档的自动迁移。

Model 节点保存明确的后端来源；Image 节点只保存 `images[]` 相对文件引用。图片库
从磁盘枚举图片和读取尺寸，不维护或自动删除 manifest。旧配方与运行历史不属于
当前工作区；需要保留的内容应先整理为图和库图片，原始记录独立备份。
