# 前端代码组织

`main.ts` 初始化后端注册表并挂载 Svelte 应用。应用加载本地资源，同时连接设备保存的
Server；图会话与生图执行不依赖全局活动后端。

| 目录 | 职责与入口 |
| --- | --- |
| `backends/` | `connections.ts` 校验 / 保存连接配置、发现稳定 Server 身份；`registry.svelte.ts` 管理连接状态、独立存储实例与 Provider 执行器；`types.ts` 定义后端、图定位与 Provider 能力 |
| `app/` | 应用装配、顶栏、后端管理 / 本地 Provider 设置 / OPFS 备份 |
| `workspace/` | `store.ts` 存储接口，`httpStore.ts` 与 `opfs/store.ts` 实现；`Sidebar.svelte` 挂载后端根，`BackendGraphs.svelte` 管理每棵图树 |
| `canvas/` | Rete 编辑器、会话 / 保存队列、Graph/View 转换与执行；`session.svelte.ts` 将会话固定到 `{ backendId, id }` |
| `generation/` | `generator.ts` 执行契约、`direct.ts` 浏览器执行、`remote.ts` 服务器执行；`localConfig.ts` 管理 Local Provider；`profiles.ts` 参数协议档案，`params.ts` 从当前模型档案派生 UI 控件 |
| `images/` | 图片引用、拖拽载荷、导入队列、字节嗅探与内容指纹；持久拖拽带 `backendId`，临时产物只带会话 URL |
| `library/` | `LibraryDock.svelte` 汇总各后端目录根并浏览所选来源；`transfer.ts` 显式跨后端复制文件 / 目录 |
| `ui/` | 通用控件、主题、确认框、消息与图片预览 |

`settings.ts` 定义设备 localStorage 键和读取方法，保持零依赖；主题、后端注册表和
图会话共同使用它，避免模块求值循环。启动直接装配当前工作区，不执行历史迁移。

依赖规则：

- `WorkspaceStore` 只负责图、View、分组、库与图内参考图。调用方拿到明确后端实例，
  保存、上传与删除前捕获所属后端；Local Provider 配置由 `generation/localConfig.ts` 管理。
- 每个 Provider 有独立 `ImageGenerator`。执行前固定其来源与实例，Graph 存储位置不影响
  执行路由。Remote Provider 只发现模型档案，前端不读取 Server 凭证配置。
- Provider 节点持久保存 `providerBackendId`、`provider`、`modelId`。服务端稳定身份跨客户端
  保持一致；设备只保存挂载地址和显示名。
- 结构文档不保存运行状态、产物、data/blob 图片 URL。参考图复制到目标图，展示 URL 运行时解析。
- 导入器每批任务固定上传目标与原节点，切图后不能提交异步结果；执行也检查会话 epoch，
  即使切走再回到同一个图也不接收旧结果。
- 参数 UI 从所选 Provider 模型档案派生。连线结构变更通过 `structure.svelte.ts` 通知控件；
  Svelte runes 模块使用 `.svelte.ts`。
- `ui/` 不依赖业务模块；不保留旧工作区切换、全局 API 包装和旧路径兼容入口。

`window.__astelierRuntime` 仅供浏览器测试包装存储失败和注入替身生图器。正常应用不注入。
运行方式见根目录 [README](../README.md)。验证使用根目录的 `cargo test`，以及 `web/`
下的 `bun run test`、`bun run check` 和 `bun run test:browser`。浏览器测试需要运行
Vite，并通过 `PLAYWRIGHT_MODULE`、`CHROMIUM_PATH` 指定 Playwright 模块和 Chromium；
`ASTELIER_TEST_URL` 可覆盖默认开发地址，`ASTELIER_SERVER_BIN` 可指定 Rust 服务二进制。
