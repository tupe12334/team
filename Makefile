GOBIN := $(shell go env GOPATH)/bin
PATH  := $(GOBIN):$(PATH)

.PHONY: generate
generate: ## Regenerate all code from proto files
	PATH=$(PATH) buf generate
	# Workaround for protoc-gen-cobra bug: &Empty{} → &emptypb.Empty{}
	# https://github.com/NathanBaulch/protoc-gen-cobra/issues (unqualified Empty for google.protobuf.Empty)
	sed -i '' 's/v := &Empty{}/v := \&emptypb.Empty{}/g' cli/gen/daemon.cobra.pb.go

.PHONY: build
build: ## Build the CLI binary
	cd cli && go build -o ../bin/team .

.PHONY: install
install: ## Install the CLI to GOBIN
	cd cli && go install .

.PHONY: mcp
mcp: ## Build the MCP server binary
	cd mcp && go mod tidy && go build -o ../bin/mcp-server .
