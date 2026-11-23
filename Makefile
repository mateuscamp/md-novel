# Portão único: verde antes de dizer pronto. Precisa do pandoc no PATH.
.PHONY: check
check:
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace
