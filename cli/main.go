package main

import (
	"fmt"
	"os"

	client "github.com/NathanBaulch/protoc-gen-cobra/client"
	"github.com/spf13/cobra"
	"github.com/tupe12334/team/cli/gen"
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
	client.DefaultConfig.ServerAddr = daemonAddr()

	root := &cobra.Command{
		Use:   "team",
		Short: "Task queue manager for AI agents",
	}

	root.AddCommand(gen.AgentServiceClientCommand())
	root.AddCommand(gen.DaemonServiceClientCommand())
	root.AddCommand(gen.QueueServiceClientCommand())
	root.AddCommand(gen.WorkerServiceClientCommand())

	if err := root.Execute(); err != nil {
		os.Exit(1)
	}
}
