# 前端状态与纯 Web 迁移 TODO

本清单汇总本次讨论的改动方向。全库审查发现仍见 [audit.md](./audit.md)。

## 设计原则

- 保留 Graph + View：Graph 描述节点、参数和连线；View 描述位置和视口。
- 节点内联持有当前运行状态和输出，不建立独立的持久化 Run 实体。
- 生成结果属于会话临时数据。用户拖入图片库时，才持久保存所选择的产物。
- 保存图文档不隐式归档生成图片或运行历史。
- 沿用 Svelte 5 + Rete，先改善响应式接入。
- 工作区可选：本地 OPFS + 浏览器直连，或远端 server URL + 服务端代理生图；两个抽象边界（`WorkspaceStore` / `ImageGenerator`）下的实现可独立替换，是未来 SaaS（账号 + 数据同步）的接入点。

## 1. 明确保存与引用语义

- [x] 图文档和 View 继续自动保存（2026-10-04 起）；本轮保持不变。
- [x] 临时生成图会话内跨节点使用（Image 节点拖入 temp ref 不上传）；保存图时不归档（serialization 过滤 dataUrl）；已由浏览器测试覆盖。
- [x] API key 保存策略：本地工作区明文存 OPFS config.json，设置界面可清除；远端工作区密钥留在服务端（2026-10-05 定案）。

## 2. 将节点与当前图状态接入 Svelte 响应式

- [x] 全部完成（2026-10-04）。主要位置：[节点类](./web/src/canvas/nodes/model.svelte.ts)、[节点编辑](./web/src/canvas/nodes/actions.ts)、[图状态](./web/src/canvas/session.svelte.ts)、[保存队列](./web/src/canvas/saveQueue.ts)、[文档序列化](./web/src/canvas/document.ts)。

## 3. 建立存储与生图两个边界

- [x] `WorkspaceStore`：图、View、分组、库文件、图内参考图、配置与图片 URL 解析，调用方不依赖 `/api/*` 或磁盘路径。[接口](./web/src/workspace/store.ts)、[OPFS 实现](./web/src/workspace/opfs/store.ts)、[远端实现](./web/src/workspace/httpStore.ts)。
- [x] `ImageGenerator`：接收模型、prompt、参数和参考图，返回本次生成的图片。[接口](./web/src/generation/generator.ts)、[直连实现](./web/src/generation/direct.ts)、[代理实现](./web/src/generation/api.ts)。
- [x] 组件、图执行与图片导入全部经接口调用；HTTP 实现保留并接线为「远端工作区」模式。
- [x] 持久引用 = 文件名/相对路径/UUID；展示 URL 运行时解析（本地 blob: 缓存、远端绝对地址），临时 `blob:`/`data:` URL 不写入文档。

2026-10-05 已完成。工作区选择在顶栏（localStorage），切换前 flush 保存。

## 4. 删除 Run 与产物自动归档

- [x] 全部完成（2026-10-04）。执行逻辑见 [execute.ts](./web/src/canvas/execute.ts)；生成直接等待返回，请求内完成解码。

## 5. 用 OPFS 实现本地存储

- [x] OPFS 版 `WorkspaceStore`：Graph/View JSON、分组、图片库、图内参考图与配置；布局沿用 data/ 结构（`atelier/` 命名空间）。
- [x] 建图初始化（种子管线）、图/分组管理、图片目录管理迁到前端；图身份 = 稳定 UUID，重命名不再换 id。
- [x] `/asset/`、`/store/`、`/gstore/` 静态寻址替换为本地 blob: 对象 URL（缓存 + 删除/移动时释放）。
- [x] 导入/导出 zip（fflate）：图按 updated_at last-writer-wins 合并、分组按 id 并集、库按路径覆盖；密钥不随 zip 转移。[transfer.ts](./web/src/workspace/opfs/transfer.ts)。
- [x] `navigator.storage.persist()` 申请 + 设置内配额展示；跨标签页读改写经 Web Locks 串行化；读写失败按实际错误反馈（保存失败阻塞切图，测试覆盖）。

## 6. 将生图协议适配迁到浏览器

- [x] Provider 配置本地读取（base_url / 模型 / 用户密钥），`env:VAR` 解析随服务端路径留在远端模式。
- [x] 模型档案、参数默认值、校验与协议字段映射迁到前端，单一来源 [profiles.ts](./web/src/generation/profiles.ts)；UI 参数表由档案派生（[params.ts](./web/src/generation/params.ts)）。
- [x] 浏览器直连：文生图 JSON → `/images/generations`，参考图 multipart → `/images/edits`（spike 已验证入口）。
- [x] 参考图直接读会话 Blob（blob:/data:），不经后端资产转换。
- [x] base64 与 URL 两种结果统一转 data URL，保留全部图片供选择收藏；URL 型跨域下载失败时报错明确指向原因。
- [x] 生成错误反馈到原节点状态并结束 busy；不创建运行档案。
- [x] 免费网关探测：用应用自己的 direct 实现连真实 Poke，无效密钥的 401 中文错误可读（2026-10-05）——请求组装、跨域与错误提取全链路通过。

### CORS spike 状态

- [x] 真实 Chromium 验证 Poke 跨域（本地 HTTP 与 HTTPS 来源、预检、错误可读）。
- [x] 一次真实生图（300s 后 502，Poke 上游问题，无 CORS 拦截）。
- [x] 自研 direct 实现直连真实网关的错误链路（见上）。
- [ ] 成功生图 → 解码 → 显式收藏 → OPFS → 刷新读取的完整付费链路（需真实密钥，手动验收）。
- [ ] 带真实参考图的 multipart edits 付费请求。
- [ ] 返回 URL 型结果时图片服务器的跨域下载。

证据见 [spike 文档](./spikes/cors/README.md) 与 [原始结果](./spikes/cors/results.json)。

## 7. 收尾与验收

- [x] 仅启动静态前端即可使用本地工作区与直连 provider；运行时不依赖 Rust 服务（远端模式为可选项）。
- [x] vite 代理删除；`just serve` 保留为远端服务端入口（含 CORS 层）。
- [x] 图和 View 按自动保存策略恢复；运行状态和未收藏产物不随刷新恢复（浏览器测试覆盖）。
- [x] 临时产物可预览、跨节点传递与拖入库；收藏后刷新可读；库文件不随删图删除（浏览器测试覆盖）。
- [x] 切图、删节点、请求失败和保存失败时的实际行为全部有测试。
- [x] 对照 [audit.md](./audit.md) 更新条目（2026-10-05，见其顶部处置表）。
- [ ] 真实 provider 完整付费链路验收一次（同 §6 遗留三项）。

## SaaS 演进路径（2026-10-05 记录，未实现）

- 远端工作区 + 账号鉴权 = 服务端最小雏形；`httpStore` 即同步客户端传输底座（同步单元 = graph/view/groups/图内参考图/库文件，逐文件 last-writer-wins + updated_at，zip 导入的合并规则已按此对齐）。
- 文档只存稳定引用，URL 运行时解析——未来解析成远端 URL 即可。
- 库文件以路径为身份、config.json 不参与同步（每设备各自配置）；落地前再定稿。
