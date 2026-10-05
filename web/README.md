# 前端代码组织

`src/main.ts` 按 localStorage 的工作区选择装配 `WorkspaceStore` 与
`ImageGenerator`（本地 OPFS + 直连，或远端 HTTP + 服务端代理），再挂载
`app/App.svelte`。`window.__atelierRuntime` 是浏览器集成测试的注入缝。

| 目录 | 职责 | 主要入口 |
| --- | --- | --- |
| `app/` | 装配侧栏、画布、图片库与通用浮层；顶栏（工作区切换）、Provider 设置 | `App.svelte`、`Topbar.svelte`、`SettingsDialog.svelte` |
| `canvas/` | Rete 编辑器、当前画布会话、节点执行与保存 | `editor.ts`、`session.svelte.ts`、`execute.ts` |
| `workspace/` | `WorkspaceStore` 抽象、工作区选择、图/分组 API、图内参考图与侧栏；`opfs/` 为本地实现（fs 原语、路径校验、对象 URL 缓存、zip 导入导出），`httpStore.ts` 为远端实现 | `store.ts`、`selection.svelte.ts`、`api.ts`、`opfs/store.ts` |
| `generation/` | `ImageGenerator` 抽象（`generator.ts`）、浏览器直连（`direct.ts`）、HTTP 代理（`api.ts`）、模型档案与协议纯逻辑（`profiles.ts` / `protocol.ts`）、UI 参数表（`params.ts`，由档案派生）与 Provider 配置状态 | `direct.ts`、`profiles.ts` |
| `images/` | 图片引用类型、拖拽载荷、导入队列、内容指纹、字节嗅探（格式/宽高） | `types.ts`、`import.ts`、`sniff.ts` |
| `library/` | 用户收藏的图片库面板、库 API（委托 store）与路径操作 | `LibraryDock.svelte`、`api.ts`、`paths.ts` |
| `ui/` | 通用控件、图标、主题、确认框、消息和图片预览 | `theme/`、`confirm/`、`toast/`、`lightbox/` |

`canvas/nodes/model.svelte.ts` 只负责节点身份、端口和响应式字段。参数保存与恢复放在 `nodes/serialization.svelte.ts`，Graph/View 与 Rete 的转换放在 `document.ts`。Svelte runes 所在模块使用 `.svelte.ts`。

`canvas/editor.ts` 负责初始化和连接 Rete 插件。视口控制放在 `viewport.ts`，指针与连线选择放在 `interactions.ts`；画布组件直接依赖这些模块，避免反向依赖初始化模块。`dom/` 集中处理控件事件与 Rete 原生事件的接合。

依赖规则：

- `workspace/store.ts` 是存储抽象边界：组件与画布不感知 OPFS / HTTP 实现与磁盘路径；图片展示 URL 由 store 解析（本地为缓存 blob: URL）。
- `generation/generator.ts` 是生图抽象边界：执行引擎不感知直连 / 代理；参数定义单一来源于 `generation/profiles.ts`，`params.ts` 仅做 UI 派生。
- `workspace/types.ts` 是纯数据契约，不引入 Svelte、Rete 或 HTTP。
- `images/` 不依赖画布节点或会话；导入器通过参数接收上传与节点更新操作。`sniff.ts` 的格式/宽高嗅探被存储与生图协议共用。
- `ui/` 不依赖业务模块。图片库组件放在 `library/`，画布组件放在 `canvas/`。
- HTTP 请求按功能放到对应模块；从具体模块直接导入，不设聚合转发入口或旧路径兼容文件。

开发、构建和测试命令见根目录 [README](../README.md#开发与验证)。浏览器检查使用真实 Svelte/Rete 与真实 OPFS（本地模式）或 mock 远端服务（远端模式），不发起真实生图请求。
