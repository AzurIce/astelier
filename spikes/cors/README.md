# 浏览器直连生图 API：CORS spike

针对当前配置的 Poke provider，用真实 Chromium 测试浏览器直连；不调用项目的 Rust 后端。

## 方法

- 测试来源：本地 `http://127.0.0.1:<随机端口>` 和真实 `https://example.com`。
- HTTPS 测试通过 Playwright 在该文档中执行探测函数，origin 保持 `https://example.com`。
- 保留浏览器默认 CORS、TLS 检查；不使用 `route` / `fulfill`、请求代理或关闭安全检查的启动参数。
- 从项目 `data/config.json` 读取 provider；沿用现有环境变量凭证解析，不输出或保存 key。
- 请求使用 `credentials: 'omit'`，通过显式 `Authorization: Bearer …` 鉴权，不依赖 cookie。
- 通过浏览器 CDP 记录真实 OPTIONS/Fetch 状态及 CORS 响应头。

普通探测不生成图片：读取 `/models`，向 `/images/generations` 提交缺少 model/prompt 的 JSON，向 `/images/edits` 提交缺少 prompt/image 的 multipart。

## 已验证

2026-10-04，Chromium 154.0.8037.57，当前 Poke 配置：

| 请求 | 本地 HTTP | 真实 HTTPS | 说明 |
|---|---|---|---|
| 带 Authorization 的 GET `/models` | OPTIONS 204，GET 200 可读 | OPTIONS 204，GET 200 可读 | 当前凭证有效 |
| 带 Authorization 的 JSON POST `/images/generations` | OPTIONS 204，POST 400 可读 | OPTIONS 204，POST 400 可读 | 缺少必填项，未请求生成 |
| 带 Authorization 的 multipart POST `/images/edits` | OPTIONS 204，POST 400 可读 | OPTIONS 204，POST 400 可读 | 缺少参考图，未请求编辑 |

Poke 的预检和实际响应均包含 `Access-Control-Allow-Origin: *`；预检允许 POST、GET 和 Authorization、Content-Type。

这些 400 是请求校验结果，不是跨域失败：浏览器拿到了可读取的 JSON 错误响应。

额外以 OpenAI 官方 API 做未经认证的对照，两个来源都能读取 401 JSON。没有 OpenAI 凭证，不据此声称其成功生图链路已经通过。

另外执行了一次真实生图：从 `https://example.com` 直连 Poke，模型 `gpt-image-2`，`n=1`、`quality=low`、`size=1024x1024`。

- 耗时 300,467 ms，返回 HTTP 502。
- 浏览器能完整读取该 JSON 响应；响应仍包含 `Access-Control-Allow-Origin: *`。
- 返回错误：“上游模型服务返回了无效响应，请稍后重试；若持续发生，请联系管理员。责任方：OpenAI。”这是 Poke 的错误归因，没有独立验证上游故障原因。
- 没有收到结果图片，因此本次没有执行结果图片解码、下载或 OPFS 写入；未重试。

结论：这次真实请求的失败发生在服务响应层面，没有被浏览器 CORS 拦截。成功生图及结果图片链路尚未验证。

完整结果及响应头记录在 [results.json](./results.json)，共 13 项探测。它不包含凭证、图片内容或带签名的结果 URL。

## 复现

### 手工页面

```sh
python3 -m http.server 8765 --bind 127.0.0.1 --directory spikes/cors
```

打开 `http://127.0.0.1:8765`，填写自己的 Base URL 和 key，点击“探测跨域”。页面仅执行上述不生成图片的探测，key 不写入浏览器存储。

也可以将这三个页面文件部署到自己的静态 HTTPS 站点验证实际部署来源。

### 自动化脚本

需要可用的 Chromium 和 Playwright / playwright-core。安装在仓库外即可：

```sh
spike_deps_dir=$(mktemp -d)
npm install --prefix "$spike_deps_dir" playwright-core
PLAYWRIGHT_MODULE="$spike_deps_dir/node_modules/playwright-core/index.mjs" \
CHROMIUM_PATH="$(command -v chromium)" \
node spikes/cors/run.mjs
```

从项目根目录运行。默认读取 `data/config.json`，可用 `ATELIER_DATA_DIR` 指定另一个数据目录。输出覆盖 `spikes/cors/results.json`。

显式添加 `--generate` 才进行一次真实生图：使用第一个有凭证的配置 provider、其第一个模型，`n=1`、`quality=low`、`size=1024x1024`。该调用可能计费，运行前先确认所选择的 provider/model。没有可读取的 JSON 探测响应时跳过。

生成响应若为 base64，在浏览器解码；若为图片 URL，继续用普通 CORS fetch 下载。随后验证图片可解码、写入 OPFS、读回，并删除本次临时文件。

## 结论边界

当前 Poke 的模型列表、JSON 生图入口和 multipart 编辑入口不存在已观察到的跨域阻碍，可以继续开发浏览器直连实现。

multipart 目前只验证请求入口与校验错误，没有执行一次真实图生图。返回 URL 的图片服务器需要独立验证；仅成功接收 base64 不代表所有外部图片 URL 都支持跨域。
