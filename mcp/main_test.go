package main

import (
	"testing"
)

func TestDaemonAddr_DaemonAddrEnvSet(t *testing.T) {
	t.Setenv("DAEMON_ADDR", "192.168.1.100:9090")
	t.Setenv("DAEMON_PORT", "")
	got := daemonAddr()
	if got != "192.168.1.100:9090" {
		t.Fatalf("expected DAEMON_ADDR value, got %q", got)
	}
}

func TestDaemonAddr_DaemonPortEnvSet(t *testing.T) {
	t.Setenv("DAEMON_ADDR", "")
	t.Setenv("DAEMON_PORT", "12345")
	got := daemonAddr()
	want := "[::1]:12345"
	if got != want {
		t.Fatalf("expected %q, got %q", want, got)
	}
}

func TestDaemonAddr_DefaultsTo50051(t *testing.T) {
	t.Setenv("DAEMON_ADDR", "")
	t.Setenv("DAEMON_PORT", "")
	got := daemonAddr()
	want := "[::1]:50051"
	if got != want {
		t.Fatalf("expected %q, got %q", want, got)
	}
}
