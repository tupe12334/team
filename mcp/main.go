package main

import (
	"fmt"
	"log"
	"os"

	mcpserver "github.com/mark3labs/mcp-go/server"
	"github.com/redpanda-data/protoc-gen-go-mcp/pkg/runtime/mark3labs"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"

	"github.com/tupe12334/team/cli/gen"
	"github.com/tupe12334/team/cli/gen/genmcp"
)

func daemonAddr() string {
	if addr := os.Getenv("DAEMON_ADDR"); addr != "" {
		return addr
	}
	port := os.Getenv("DAEMON_PORT")
	if port == "" {
		port = "50051"
	}
	return fmt.Sprintf("[::1]:%s", port)
}

func main() {
	addr := daemonAddr()

	conn, err := grpc.NewClient(addr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		log.Fatalf("connect: %v", err)
	}
	defer conn.Close()

	rawServer, srv := mark3labs.NewServer("team-daemon", "0.1.0")

	genmcp.ForwardToAgentServiceClient(srv, gen.NewAgentServiceClient(conn))
	genmcp.ForwardToDaemonServiceClient(srv, gen.NewDaemonServiceClient(conn))
	genmcp.ForwardToQueueServiceClient(srv, gen.NewQueueServiceClient(conn))
	genmcp.ForwardToWorkerServiceClient(srv, gen.NewWorkerServiceClient(conn))

	if err := mcpserver.ServeStdio(rawServer); err != nil {
		log.Fatalf("serve: %v", err)
	}
}
