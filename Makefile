# Portão único: verde antes de dizer pronto. Precisa do pandoc no PATH, do
# Node e, no Linux, do webkit2gtk-4.1 (para compilar o app).
.PHONY: check interface instalar desinstalar
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

# Põe o md Novel no menu de aplicativos (só no Linux): compila o programa de
# verdade, sem depuração, e copia o programa, o ícone e a entrada do menu
# para o prefixo. Rode de novo depois de atualizar o projeto. Não abre janela.
# A conferência vem primeiro para não compilar minutos e recusar no fim.
PREFIXO ?= $(HOME)/.local
instalar:
	sh app/linux/instalar.sh conferir "$(PREFIXO)"
	cd app && npm ci --ignore-scripts --no-audit --no-fund
	cd app && cargo tauri build --no-bundle
	sh app/linux/instalar.sh instalar "$(PREFIXO)" target/release/md-novel-app

# Apaga os três arquivos que o `instalar` criou, e só eles.
desinstalar:
	sh app/linux/instalar.sh desinstalar "$(PREFIXO)"
