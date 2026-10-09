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

顶栏「导出图」或侧栏图的操作菜单可导出 `.astelier` 图包。它是 ZIP 文件，内容为该图目录的
`graph.json`、`view.json` 和 `store/`（全部图内参考图）。可通过「导入图」选择文件，或直接拖入应用；
拖到侧栏存储分区或文件夹时会导入对应位置，其余区域使用当前图所属存储。每次导入都会创建新图，
保留节点 ID、连线、布局与参考图，不覆盖原图。
拖拽时会高亮目标存储或文件夹，并显示导入位置的预览块。breadcrumb 左侧按钮可以收起／展开整个侧栏，
状态会保存在当前浏览器；「管理」入口在侧栏展开时位于侧栏标题，收起后位于顶栏。

图包不包含 Provider 配置、密钥、全局图片库或临时生成结果；分享给其他设备后，可在 Model 节点重新选择
当地可用的 Provider 和模型。整个浏览器工作区的备份仍在设置的「导入 / 导出」中。

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
