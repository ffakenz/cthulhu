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

# The crates the decision records admit for the program, besides the workspace's own packages.
ADMITTED := arrayvec autocfg csv csv-core itoa memchr num-traits proc-macro2 quote rust_decimal \
    ryu serde serde_core serde_derive syn thiserror thiserror-impl unicode-ident

# The crates the decision records admit for tests only: never in the program's build graph.
ADMITTED_DEV := bitflags cfg-if chacha20 core_detect cpufeatures getrandom libc proptest r-efi \
    rand rand_core rand_xorshift regex-syntax unarray

deps: ## Fail when the build graph holds a crate no decision record admits, or the program a test-only one
	@tree() { $(CARGO) tree --workspace --locked --edges "$$1" --target all --all-features --prefix none --format '{p}'; }; \
	outside() { printf '%s\n' "$$1" | awk 'NF {print $$1}' | sort -u | while read -r crate; do \
		case "$$2" in (*" $$crate "*) ;; (*) echo "$$crate" ;; esac; \
	done; }; \
	graph=$$(tree normal,build,dev) || exit 1; \
	program=$$(tree no-dev) || exit 1; \
	members=$$($(CARGO) tree --workspace --locked --depth 0 --prefix none --format '{p}') || exit 1; \
	members=" $$(printf '%s\n' "$$members" | awk 'NF {printf "%s ", $$1}')"; \
	unadmitted=$$(outside "$$graph" "$${members}$(ADMITTED) $(ADMITTED_DEV) "); \
	if [ -n "$$unadmitted" ]; then printf 'The build graph holds crates no decision record admits:\n%s\n' "$$unadmitted"; exit 1; fi; \
	test_only=$$(outside "$$program" "$${members}$(ADMITTED) "); \
	if [ -n "$$test_only" ]; then printf 'The program build graph holds crates admitted for tests only:\n%s\n' "$$test_only"; exit 1; fi

ci: deps fmt-check lint build test ## The commit gate

run: ## Run the cthulhu binary on INPUT, the table on stdout: make run INPUT=<file>
	@$(CARGO) run --locked --bin cthulhu -- $${INPUT:+"$$INPUT"}

clean: ## Remove build output
	$(CARGO) clean
