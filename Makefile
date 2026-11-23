# Portão único: verde antes de dizer pronto. Precisa do pandoc no PATH, do
# Node e, no Linux, do webkit2gtk-4.1 (para compilar o app).
.PHONY: check interface
check: interface
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace

# A interface vem antes do cargo: tipos, testes e o pacote da página.
interface:
	cd app && npm ci --ignore-scripts --no-audit --no-fund
	cd app && npx tsc --noEmit
	cd app && npm test
	cd app && npx vite build
