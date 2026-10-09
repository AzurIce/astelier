# 多后端与共享资源 TODO

## 已实现（2026-10-09）

- [x] 前端同时挂载 OPFS 与多个 Rust Server；连接、显示名、当前图跨刷新恢复。
- [x] 左侧按后端分组的可折叠图树；底部按后端分根的图片库。
- [x] 后端独立加载、错误反馈、重连；移除连接不删除远端数据。
- [x] Server 持久实例身份与稳定图 UUID；重命名不改变引用。
- [x] 图定位、会话与异步保存明确绑定后端；跨后端同名资源隔离。
- [x] Local / Remote Provider 同时发现、按来源选择，可用于任意后端上的图。
- [x] 模型节点保存明确 Provider 来源；参数 UI 派生自对应模型档案。
- [x] Remote Provider 发现不返回凭证；通过 Provider 专属端点执行，删除旧配置与通用生成入口。
- [x] Server key 支持直接环境变量名，包含数字与小写；空 / 缺失变量报错，密钥仅服务端解析。
- [x] 参考图跨后端复制到目标图内存储；持久 OPFS blob 图片正确保存，临时图片保持会话语义。
- [x] 跨库文件 / 目录同源移动、跨源复制；结果可收藏到任意挂载来源。
- [x] Graph + View 自动保存；生成结果不自动归档，刷新不恢复 busy/error/未收藏产物。
- [x] 执行时固定 Provider、参数与参考图快照；晚返回不能影响新会话，即使回到同一个图。
- [x] OPFS zip 不包含 Provider 配置 / 密钥；图内参考图与空目录导入恢复。
- [x] 真实 OPFS 与两套真实 Rust Server 的受控浏览器集成测试；不调用付费 provider。
- [x] 设计思想同步到 Vault 的 Atelier 项目；更新 README、前端文档与开发注释。

实现入口：[后端注册表](web/src/backends/registry.svelte.ts)、[图会话](web/src/canvas/session.svelte.ts)、
[Local 执行](web/src/generation/direct.ts)、[Remote 执行](web/src/generation/remote.ts)、
[Server API](src/api.rs)、[OPFS 存储](web/src/workspace/opfs/store.ts)。

## 尚待真实上游验收

- [ ] 真实 provider 成功生图 → 解码 → 显式收藏 → OPFS → 刷新读取。
- [ ] 携带真实参考图的 multipart edits 付费请求。
- [ ] 返回 URL 型结果时图片服务器的跨域下载。

受控上游覆盖协议、图片传输与实际存储流程；不代表上述真实服务已验收。

## 后续设计范围

- [ ] 登录后挂载云后端：账号鉴权、访问权限与用户数据隔离。
- [ ] 自动同步（独立于挂载与显式复制），冲突语义另行确定。

历史审查见 [audit.md](audit.md)；其初审证据不应视为当前实现。
