default: serve

# 服务端：REST API + 资产 + 托管 web/dist
serve:
	cargo run --release

# 纯静态前端开发：无需后端，可选连接任意 Rust 服务
dev-web:
	cd web && bun run dev

# 前端构建：web/dist 可部署到静态站点，也可由 Rust 服务托管
build-web:
	cd web && bun install && bun run build

check:
	cargo check

clean:
	rm -rf target web/dist web/node_modules
