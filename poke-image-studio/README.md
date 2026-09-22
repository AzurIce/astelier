# Poke Image Studio

Web 版生图 API 调用工作台（纯静态、零依赖、零构建）。按 **OpenAI Images 兼容协议** 实现完整 gpt-image 参数集，可自定义任意 Provider（OpenAI 官方、Poke API 等 OpenAI 兼容网关）与模型列表。

## 启动

```bash
cd poke-image-studio
python3 -m http.server 8642
# 打开 http://127.0.0.1:8642
```

> 推荐用本地 HTTP 服务打开；直接双击 index.html（file://）时部分浏览器会限制 fetch。

## 功能

- **布局**：Provider 是顶栏下拉框（「管理」编辑当前 Provider 的 Key/URL，「+ Provider」新增）；左侧边栏是**可视化模型列表**——默认只显示 `gpt-image-2` / `gpt-image-2.5-sunburst` / `gpt-image-2.5-flare` 三个型号（旧配置里的默认 11 模型列表会自动迁移精简；dall-e、gpt-image-1 系可随时用底部输入框加回或「拉取」重建，能力档案仍保留），每个模型带能力徽章（任意尺寸 / 透明底 / 流式 / mask 编辑 / xhigh|max / style / url+b64，红 = 不可编辑，橙 = 网关转换模型），点击模型名即选中（作用于当前模式，文生图与编辑分开记忆），点 ▸ 展开**能力树**（尺寸预设与自定义规则、画质档位、数量上限、背景/流式/输出/编辑能力、Prompt 上限等结构化详情）；底部输入框可快速添加自定义模型 ID。
- **自定义尺寸约束可视化**：选择「自定义 WxH…」后，输入框下方实时显示约束规则并逐字符校验（✗ 红色报错原因 / ✓ 绿色可用），规则随模型档案变化（gpt-image-2+：边长被 16 整除、宽高比 1:3–3:1、上限 3840×2160）。
- **Provider 管理**：多 Provider（名称 / Base URL / API Key / 代理前缀），localStorage 持久化，支持导出/导入 JSON 配置、从 `/v1/models` 一键拉取模型列表（自动过滤图像模型）。
- **文生图** `POST /v1/images/generations`（JSON）：
  `model`、`prompt`、`quality`（auto/high/medium/low/xhigh/max/hd/standard）、`size`（预设 + 任意 WxH 自定义，含 16 整除 / 1:3–3:1 / 3840x2160 校验）、`background`（透明底）、`moderation`、`n`（1–10）、`output_format`（png/jpeg/webp）、`output_compression`、`partial_images`、`stream`（SSE 流式渐进预览）、`style`（dall-e-3）、`response_format`（dall-e）、`user`。
  未选择的参数一律不发送，避免网关参数校验失败。
- **图片编辑** `POST /v1/images/edits`：
  - 本地文件模式（multipart/form-data）：最多 16 张参考图（`image[]`）+ **蒙版画板**（涂刷区域导出为透明 = 重绘区，支持反相/撤销/橡皮，或直接上传 mask PNG）+ `input_fidelity`（high/low）。
  - JSON 模式：参考图用公网 URL 或 `fileid:xxx`（映射为 `{file_id}`），mask 用 URL / file_id。
  - 「以最近结果添加」：把上一次生成结果一键变为编辑参考图，形成 生成→编辑 迭代闭环。
- **结果区**：图片画廊、点击放大、下载；显示 usage tokens、revised_prompt；「请求详情」弹窗含等价 cURL（Key 脱敏）、请求 Body、原始响应，方便排错。
- 表单状态自动保存；`⌘/Ctrl + Enter` 快捷提交。

## 模型能力元数据（参数区由元数据生成）

参数区不是写死的：每个模型的能力以 **params schema** 元数据描述，表单、能力树、请求体收集、校验全部由它驱动。接入新模型 = 配一份元数据，例如 nano-banana：

```json
{
  "stream": false,
  "maxPrompt": 32000,
  "editsText": "指令式编辑（多参考图，无 mask）",
  "params": [
    { "key": "image_size", "label": "image_size 分辨率档", "type": "select", "options": ["1K", "2K", "4K"] },
    { "key": "aspect_ratio", "label": "aspect_ratio 比例", "type": "select", "options": ["auto", "1:1", "16:9", "9:16"] },
    { "key": "n", "label": "n 数量", "type": "number", "min": 1, "max": 4 }
  ]
}
```

- param 字段：`key`（请求体字段名）、`type`（select / number / text / size 复合控件）、`label`、`options`、`min` / `max`、`sizeCustom` + `sizeRule`（size 自定义 WxH 及其规则提示）、`modes`（限定 gen / edit）、`requiresStream`（仅流式时发送）。
- 内置档案（gpt-image 全系 / dall-e）也用同一 schema 声明；切换模型时参数区按元数据重新生成，不支持的参数根本不会出现，请求里绝不会带非法字段。
- 侧边栏每个模型行有 **⚙** 按钮：JSON 元数据编辑器（带模板：gpt-image-2.5、网关通用、Gemini/Banana 风格示例），保存后仅覆盖**当前 Provider** 下的该模型，徽章与能力树随之从元数据重新推导。
- 未配置覆盖的未知模型走通用档案（放行全部参数并注明以网关为准）。

## 参数对照（与官方 API Reference 一致）

| 参数 | generations | edits | 说明 |
|---|---|---|---|
| prompt | ✅ | ✅ | 最长 32000 字符（gpt-image） |
| model | ✅ | ✅ | 默认列表含 gpt-image 全系 + dall-e |
| quality | ✅ | ✅ | 2.5 系列支持 xhigh/max |
| size | ✅ | ✅ | gpt-image-2 起支持任意分辨率 |
| background | ✅ | ✅ | transparent 需配 png/webp 输出 |
| output_format / output_compression | ✅ | ✅ | jpeg/webp 时压缩率 0–100 |
| moderation | ✅ | ✅ | low / auto |
| n | ✅ | ✅ | 1–10 |
| stream + partial_images | ✅ | ✅ | 0–3 帧渐进预览（SSE） |
| style / response_format | ✅ | — | 仅 dall-e 系列 |
| images / mask | — | ✅ | JSON: `{image_url}` 或 `{file_id}`；multipart: 文件 |
| input_fidelity | — | ✅ | high 更忠实原图 |

## CORS 说明

浏览器直连要求网关允许跨域。OpenAI 官方 `api.openai.com` 支持浏览器直连；**Poke API 等第三方网关若未放行 CORS，请求会被浏览器拦截**（报 `Failed to fetch`）。解决办法二选一：

1. 在 Provider 的「代理前缀」填一个 CORS 代理（最终 URL = 前缀 + 完整 API URL）；
2. 用本地反向代理（如 nginx / Caddy）把 `/v1` 转发到目标网关，再把 Provider 的 Base URL 指向本地代理。

## 安全

- API Key 只保存在本机 localStorage，请求仅发往你配置的 Provider（及其代理前缀）。
- 「请求详情」中的 cURL 预览已对 Key 脱敏。
- 历史结果只保存在内存中，刷新页面即清空。
