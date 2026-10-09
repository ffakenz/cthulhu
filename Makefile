.DEFAULT_GOAL := help

CARGO := cargo

.PHONY: help fmt fmt-check lint build test deps ci run clean

help: ## List the targets
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-10s\033[0m %s\n", $$1, $$2}'

fmt: ## Format every crate
	$(CARGO) fmt --all

fmt-check: ## Fail when a file is not formatted
	$(CARGO) fmt --all -- --check

lint: ## Run clippy on every target; any warning fails
	$(CARGO) clippy --workspace --all-targets --all-features --locked -- -D warnings

build: ## Build the workspace
	$(CARGO) build --workspace --locked

test: ## Run every test
	$(CARGO) test --workspace --locked

deps: ## Fail when Cargo.lock resolves a package from outside the workspace
	@$(CARGO) metadata --format-version 1 --locked --offline > /dev/null
	@if grep -n '^source = ' Cargo.lock; then echo 'Cargo.lock resolves the packages above from outside the workspace'; exit 1; fi
	@members=$$(( $$($(CARGO) metadata --format-version 1 --no-deps --locked --offline | grep -o '"manifest_path"' | wc -l) )); \
	locked=$$(( $$(grep -c '^\[\[package\]\]' Cargo.lock) )); \
	if [ "$$members" -ne "$$locked" ]; then echo "Cargo.lock holds $$locked packages for $$members workspace members"; exit 1; fi

ci: deps fmt-check lint build test ## The commit gate

run: ## Run the cthulhu binary
	$(CARGO) run --locked --bin cthulhu

clean: ## Remove build output
	$(CARGO) clean
