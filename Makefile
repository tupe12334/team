GOBIN := $(shell go env GOPATH)/bin
PATH  := $(GOBIN):$(PATH)

-include .env
export

.PHONY: generate
generate: ## Regenerate all code from proto files
	export PATH="$$PATH:$(CURDIR)/node_modules/.bin" && buf generate
	# Workaround for protoc-gen-cobra bug: &Empty{} → &emptypb.Empty{}
	# https://github.com/NathanBaulch/protoc-gen-cobra/issues (unqualified Empty for google.protobuf.Empty)
	sed -i '' 's/v := &Empty{}/v := \&emptypb.Empty{}/g' cli/gen/agents.cobra.pb.go cli/gen/daemon.cobra.pb.go

.PHONY: build
build: ## Build the CLI binary, daemon, MCP server, client, and TUI
	cd cli && GOWORK=off go build -o ../bin/team .
	cd mcp && go mod tidy && go build -o ../bin/mcp-server .
	cd daemon && cargo build
	cd tui && cargo build
	cd client && pnpm build

.PHONY: install
install: ## Install all binaries: CLI and MCP server to GOBIN, daemon and TUI to ~/.cargo/bin
	cd cli && GOWORK=off go install .
	cd mcp && go mod tidy && go install .
	cd daemon && cargo install --path .
	cd tui && cargo install --path .

.PHONY: mcp
mcp: ## Build the MCP server binary
	cd mcp && go mod tidy && go build -o ../bin/mcp-server .

.PHONY: tui
tui: ## Run the TUI
	cd tui && cargo run

.PHONY: run
run: ## Run the daemon and client concurrently
	cd daemon && cargo run & DAEMON_ADDR="[::1]:$(DAEMON_PORT)" pnpm --prefix client dev; kill %1

.PHONY: test
test: ## Run all tests (TypeScript typecheck + client Vitest + daemon Rust unit tests)
	cd client && pnpm exec tsc --noEmit
	cd client && pnpm test
	cd daemon && cargo test

.PHONY: lint
lint: ## Run all linters (Go vet, Rust clippy, Next.js ESLint)
	cd cli && GOWORK=off go vet $(shell cd cli && GOWORK=off go list ./... | grep -v /gen)
	cd mcp && go vet ./...
	cd daemon && cargo clippy -- -D warnings
	cd tui && cargo clippy -- -D warnings
	cd client && pnpm lint
