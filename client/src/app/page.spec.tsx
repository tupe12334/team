import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen } from "@testing-library/react";

// ---------------------------------------------------------------------------
// Setup: reset module registry before each test so the module-level
// `daemonAddr` expression is re-evaluated with the current env vars.
// ---------------------------------------------------------------------------

beforeEach(() => {
  vi.resetModules();
});

afterEach(() => {
  vi.unstubAllEnvs();
});

// Re-register doMock + dynamic-import each time so fresh module evaluation
// picks up whatever process.env state was set before the import.
async function importHome() {
  vi.doMock("@/components/DaemonPanel", () => ({ default: () => null }));
  vi.doMock("@/components/ConfigPanel", () => ({ default: () => null }));
  vi.doMock("@/components/QueuePanel", () => ({ default: () => null }));
  vi.doMock("@/components/WorkersPanel", () => ({ default: () => null }));
  return (await import("./page")).default;
}

// ---------------------------------------------------------------------------
// daemonAddr derivation — three branches of the `??` / ternary expression:
//   DAEMON_ADDR set        → daemonAddr = DAEMON_ADDR value  (branch 1)
//   only DAEMON_PORT set   → daemonAddr = `[::1]:${PORT}`    (branch 2)
//   neither set            → daemonAddr = null               (branch 3)
// ---------------------------------------------------------------------------

describe("Home – daemonAddr footer", () => {
  it("shows the raw address when DAEMON_ADDR is set (DAEMON_ADDR branch)", async () => {
    vi.stubEnv("DAEMON_ADDR", "daemon:50051");
    const Home = await importHome();
    render(<Home />);
    expect(screen.getByText("gRPC → daemon:50051")).toBeTruthy();
  });

  it("constructs IPv6 loopback address from DAEMON_PORT when DAEMON_ADDR is absent (DAEMON_PORT branch)", async () => {
    // DAEMON_ADDR is not set; daemonAddr = `[::1]:${DAEMON_PORT}`
    delete process.env.DAEMON_ADDR;
    vi.stubEnv("DAEMON_PORT", "50051");
    const Home = await importHome();
    render(<Home />);
    expect(screen.getByText("gRPC → [::1]:50051")).toBeTruthy();
  });

  it("shows 'gRPC port not configured' when neither env var is set (null branch)", async () => {
    delete process.env.DAEMON_ADDR;
    delete process.env.DAEMON_PORT;
    const Home = await importHome();
    render(<Home />);
    expect(screen.getByText("gRPC port not configured")).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// Static structure: header brand always rendered regardless of env
// ---------------------------------------------------------------------------

describe("Home – static structure", () => {
  it("renders the header brand title", async () => {
    const Home = await importHome();
    render(<Home />);
    expect(screen.getByText("Daemon Control")).toBeTruthy();
  });

  it("renders the project identifier in the header", async () => {
    const Home = await importHome();
    render(<Home />);
    expect(screen.getByText("team / daemon")).toBeTruthy();
  });
});
