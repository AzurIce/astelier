# 前端代码组织

`src/main.ts` 应用主题并挂载 `app/App.svelte`。源码按功能职责组织，同一功能的组件、状态与操作放在一起；单元测试与被测模块相邻，浏览器集成检查放在 `tests/`。

| 目录 | 职责 | 主要入口 |
| --- | --- | --- |
| `app/` | 装配侧栏、画布、图片库与通用浮层，协调启动和执行 | `App.svelte`、`Topbar.svelte` |
| `canvas/` | Rete 编辑器、当前画布会话、节点执行与保存 | `editor.ts`、`session.svelte.ts`、`execute.ts` |
| `workspace/` | Graph/View 持久文档、图与分组管理、图内参考图存储 | `types.ts`、`api.ts`、`imageFiles.ts`、`Sidebar.svelte` |
| `generation/` | Provider 配置、生成 HTTP 请求与参数 schema | `config.svelte.ts`、`api.ts`、`params.ts` |
| `images/` | 图片引用类型、拖拽载荷、导入队列与内容指纹 | `types.ts`、`drag.ts`、`import.ts` |
| `library/` | 用户收藏的图片库面板、存储请求与路径操作 | `LibraryDock.svelte`、`api.ts`、`paths.ts` |
| `ui/` | 通用控件、图标、主题、确认框、消息和图片预览 | `theme/`、`confirm/`、`toast/`、`lightbox/` |

`canvas/nodes/model.svelte.ts` 只负责节点身份、端口和响应式字段。参数保存与恢复放在 `nodes/serialization.svelte.ts`，Graph/View 与 Rete 的转换放在 `document.ts`。Svelte runes 所在模块使用 `.svelte.ts`。

`canvas/editor.ts` 负责初始化和连接 Rete 插件。视口控制放在 `viewport.ts`，指针与连线选择放在 `interactions.ts`；画布组件直接依赖这些模块，避免反向依赖初始化模块。`dom/` 集中处理控件事件与 Rete 原生事件的接合。

`canvas/runtime.ts` 保存编辑器实例和画布回调。Provider 配置属于 `generation/config.svelte.ts`；图内图片上传与回收属于 `workspace/imageFiles.ts`。会话模块只协调当前图的加载、保存和切换。

依赖规则：

- `workspace/types.ts` 是纯数据契约，不引入 Svelte、Rete 或 HTTP。
- `images/` 不依赖画布节点或会话；导入器通过参数接收上传与节点更新操作。
- `ui/` 不依赖业务模块。图片库组件放在 `library/`，画布组件放在 `canvas/`。
- HTTP 请求按功能放到对应的 API 模块；从具体模块直接导入，不设聚合转发入口或旧路径兼容文件。

开发、构建和测试命令见根目录 [README](../README.md#开发与验证)。浏览器检查使用真实 Svelte/Rete 与模拟 API，不发起真实生图请求。
