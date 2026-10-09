# Astelier

基于 Svelte 5 和 Rete.js 的节点式图像生成工作台，支持提示词、参考图、模型选择和图片库。
可以独立运行在浏览器中，也可以连接 Rust 服务保存数据和调用模型。

## 开发

需要 Bun；运行服务端还需要 Rust。

```sh
cd web
bun install
bun run dev
```

在设置中配置 Provider，然后连接节点并点击 Run。图会自动保存到浏览器，生成结果可拖入图片库收藏。

## 本地服务

```sh
just build-web
just serve
```

访问 `http://127.0.0.1:8230`。服务端数据保存在 `data/`，Provider 配置放在 `data/config.json`。
可通过 `ASTELIER_ADDR`、`ASTELIER_DATA_DIR` 和 `ASTELIER_CORS_ORIGINS` 配置地址、数据目录和跨域来源。

## 部署

前端构建产物为 `web/dist`，可以部署到静态托管平台。

[GitHub Pages workflow](.github/workflows/pages.yml) 在推送到 `main` 时部署，也支持手动触发。
仓库的 Pages 发布来源需设为 **GitHub Actions**。

代码组织见 [web/README.md](web/README.md)。
