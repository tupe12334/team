GOBIN := $(shell go env GOPATH)/bin
PATH  := $(GOBIN):$(PATH)

-include .env
export

.PHONY: generate
generate: ## Regenerate all code from proto files
	PATH=$(PATH) buf generate
	# Workaround for protoc-gen-cobra bug: &Empty{} → &emptypb.Empty{}
	# https://github.com/NathanBaulch/protoc-gen-cobra/issues (unqualified Empty for google.protobuf.Empty)
	sed -i '' 's/v := &Empty{}/v := \&emptypb.Empty{}/g' cli/gen/daemon.cobra.pb.go

.PHONY: build
build: ## Build the CLI binary, daemon, and client
	cd cli && GOWORK=off go build -o ../bin/team .
	cd daemon && cargo build
	cd client && pnpm build

.PHONY: install
install: ## Install the CLI to GOBIN
	cd cli && go install .

.PHONY: mcp
mcp: ## Build the MCP server binary
	cd mcp && go mod tidy && go build -o ../bin/mcp-server .

.PHONY: run
run: ## Run the daemon and client concurrently
	cd daemon && cargo run & DAEMON_ADDR="[::1]:$(DAEMON_PORT)" pnpm --prefix client dev; kill %1

.PHONY: lint
lint: ## Run all linters (Go vet, Rust clippy, Next.js ESLint)
	cd cli && GOWORK=off go vet ./...
	cd daemon && cargo clippy -- -D warnings
	cd client && pnpm lint
