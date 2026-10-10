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

# The crates the decision records admit, besides the workspace's own packages.
ADMITTED := arrayvec autocfg csv csv-core itoa memchr num-traits proc-macro2 quote rust_decimal \
    ryu serde serde_core serde_derive syn unicode-ident

deps: ## Fail when the build graph holds a crate no decision record admits
	@graph=$$($(CARGO) tree --workspace --locked --edges normal,build,dev --target all --all-features --prefix none --format '{p}') || exit 1; \
	members=$$($(CARGO) tree --workspace --locked --depth 0 --prefix none --format '{p}') || exit 1; \
	allowed=" $$(printf '%s\n' "$$members" | awk 'NF {printf "%s ", $$1}')$(ADMITTED) "; \
	outside=$$(printf '%s\n' "$$graph" | awk 'NF {print $$1}' | sort -u | while read -r crate; do \
		case "$$allowed" in (*" $$crate "*) ;; (*) echo "$$crate" ;; esac; \
	done); \
	if [ -n "$$outside" ]; then printf 'The build graph holds crates no decision record admits:\n%s\n' "$$outside"; exit 1; fi

ci: deps fmt-check lint build test ## The commit gate

run: ## Run the cthulhu binary
	$(CARGO) run --locked --bin cthulhu

clean: ## Remove build output
	$(CARGO) clean
