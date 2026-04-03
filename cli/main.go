package main

import (
	"os"

	"github.com/spf13/cobra"
	"github.com/tupe12334/team/cli/gen"
)

func main() {
	root := &cobra.Command{
		Use:   "team",
		Short: "Task queue manager for AI agents",
	}

	root.AddCommand(gen.DaemonServiceClientCommand())
	root.AddCommand(gen.QueueServiceClientCommand())
	root.AddCommand(gen.WorkerServiceClientCommand())

	if err := root.Execute(); err != nil {
		os.Exit(1)
	}
}
