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

# Põe o md Novel nos aplicativos do sistema (Linux e macOS): compila o programa
# de verdade, sem depuração, e instala no prefixo padrão (~/.local no Linux,
# ~/Applications no macOS). Rode de novo depois de atualizar o projeto. Não abre
# janela. A conferência vem primeiro para não compilar minutos e recusar no fim.
SISTEMA := $(shell uname -s)
ifeq ($(SISTEMA),Darwin)
PREFIXO ?= $(HOME)/Applications
INSTALADOR := app/macos/instalar.sh
else
PREFIXO ?= $(HOME)/.local
INSTALADOR := app/linux/instalar.sh
endif

instalar:
	sh $(INSTALADOR) conferir "$(PREFIXO)"
	cd app && npm ci --ignore-scripts --no-audit --no-fund
	cd app && cargo tauri build --no-bundle
	sh $(INSTALADOR) instalar "$(PREFIXO)" target/release/md-novel-app

# Apaga apenas o que o `instalar` criou, sem tocar nos dados guardados pelo app.
desinstalar:
	sh $(INSTALADOR) desinstalar "$(PREFIXO)"
