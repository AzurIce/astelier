default: serve

# 服务端：REST API + 资产 + 托管 web/dist
serve:
	cargo run --release

# 前端开发：vite dev（/api、/asset 代理到 127.0.0.1:8230，另起 serve）
dev-web:
	cd web && bun run dev

# 前端构建：产物 web/dist 由服务端托管
build-web:
	cd web && bun install && bun run build

check:
	cargo check

clean:
	rm -rf target web/dist web/node_modules
