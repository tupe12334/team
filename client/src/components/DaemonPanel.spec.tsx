import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import type { DaemonInfo } from "@/components/useDaemonPanel";

vi.mock("@/components/useDaemonPanel");
import { useDaemonPanel } from "@/components/useDaemonPanel";
const mockHook = vi.mocked(useDaemonPanel);

type Panel = ReturnType<typeof useDaemonPanel>;

function makePanelState(overrides: Partial<Panel> = {}): Panel {
  return {
    info: null,
    error: null,
    loading: false,
    reloading: false,
    shuttingDown: false,
    handleReload: vi.fn(),
    handleShutdown: vi.fn(),
    ...overrides,
  };
}

function makeInfo(overrides: Partial<DaemonInfo> = {}): DaemonInfo {
  return { version: "1.0.0", uptimeSeconds: 0, configPath: "/etc/d.toml", workersCount: 4, ...overrides };
}

const { default: DaemonPanel } = await import("./DaemonPanel");

beforeEach(() => { vi.clearAllMocks(); });

// ---------------------------------------------------------------------------
// formatUptime branch coverage
// Each branch combination tests a distinct arm of the `if (h)` / `if (m)` guards.
// ---------------------------------------------------------------------------

describe("DaemonPanel – formatUptime", () => {
  it("formats seconds only when uptime < 60 s (h=0, m=0 arms skipped)", () => {
    mockHook.mockReturnValue(makePanelState({ info: makeInfo({ uptimeSeconds: 45 }) }));
    render(<DaemonPanel />);
    expect(screen.getByText("45s")).toBeTruthy();
  });

  it("formats minutes + seconds when uptime < 1 h (h=0 arm skipped, m>0 arm taken)", () => {
    // 90 s → 1 m 30 s
    mockHook.mockReturnValue(makePanelState({ info: makeInfo({ uptimeSeconds: 90 }) }));
    render(<DaemonPanel />);
    expect(screen.getByText("1m 30s")).toBeTruthy();
  });

  it("formats hours + seconds when minutes component is 0 (h>0 arm taken, m=0 arm skipped)", () => {
    // 3630 s → 1 h 0 m 30 s → "1h 30s" (m=0 → not pushed)
    mockHook.mockReturnValue(makePanelState({ info: makeInfo({ uptimeSeconds: 3630 }) }));
    render(<DaemonPanel />);
    expect(screen.getByText("1h 30s")).toBeTruthy();
  });

  it("formats hours + minutes + seconds when all components are non-zero (both arms taken)", () => {
    // 3725 s → 1 h 2 m 5 s
    mockHook.mockReturnValue(makePanelState({ info: makeInfo({ uptimeSeconds: 3725 }) }));
    render(<DaemonPanel />);
    expect(screen.getByText("1h 2m 5s")).toBeTruthy();
  });

  it("formats 0 s when uptimeSeconds is 0 (raw || 0 identity path)", () => {
    // s = 0 || 0 = 0 → h=0, m=0, sec=0 → "0s"
    mockHook.mockReturnValue(makePanelState({ info: makeInfo({ uptimeSeconds: 0 }) }));
    render(<DaemonPanel />);
    expect(screen.getByText("0s")).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// configPath fallback
// ---------------------------------------------------------------------------

describe("DaemonPanel – configPath", () => {
  it("renders configPath when it is non-empty", () => {
    mockHook.mockReturnValue(makePanelState({ info: makeInfo({ configPath: "/home/user/.config/team/config.toml" }) }));
    render(<DaemonPanel />);
    expect(screen.getByText("/home/user/.config/team/config.toml")).toBeTruthy();
  });

  it("renders em-dash when configPath is empty (falsy fallback arm)", () => {
    // `info.configPath || "—"` — the falsy arm when configPath === ""
    mockHook.mockReturnValue(makePanelState({ info: makeInfo({ configPath: "" }) }));
    render(<DaemonPanel />);
    expect(screen.getByText("—")).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// Button label states
// ---------------------------------------------------------------------------

describe("DaemonPanel – button label states", () => {
  it("shows 'Reload Config' when not reloading", () => {
    mockHook.mockReturnValue(makePanelState({ info: makeInfo(), reloading: false }));
    render(<DaemonPanel />);
    expect(screen.getByText("Reload Config")).toBeTruthy();
  });

  it("shows 'Reloading…' when reloading is true", () => {
    mockHook.mockReturnValue(makePanelState({ info: makeInfo(), reloading: true }));
    render(<DaemonPanel />);
    expect(screen.getByText("Reloading…")).toBeTruthy();
  });

  it("shows 'Shutdown' when not shutting down", () => {
    mockHook.mockReturnValue(makePanelState({ info: makeInfo(), shuttingDown: false }));
    render(<DaemonPanel />);
    expect(screen.getByText("Shutdown")).toBeTruthy();
  });

  it("shows 'Shutting down…' when shuttingDown is true", () => {
    mockHook.mockReturnValue(makePanelState({ info: makeInfo(), shuttingDown: true }));
    render(<DaemonPanel />);
    expect(screen.getByText("Shutting down…")).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// Button disabled states: disabled={reloading} and disabled={shuttingDown}
// ---------------------------------------------------------------------------

describe("DaemonPanel – button disabled states", () => {
  it("disables the Reload Config button when reloading is true", () => {
    // The label tests (above) only assert text — they never check btn.disabled.
    // This test exercises the `disabled={reloading}` prop when it is truthy.
    mockHook.mockReturnValue(makePanelState({ info: makeInfo(), reloading: true }));
    const { container } = render(<DaemonPanel />);
    const btn = container.querySelector<HTMLButtonElement>(".daemon-panel__reload-btn");
    expect(btn?.disabled).toBe(true);
  });

  it("enables the Reload Config button when reloading is false", () => {
    // Exercises the falsy arm of `disabled={reloading}`.
    mockHook.mockReturnValue(makePanelState({ info: makeInfo(), reloading: false }));
    const { container } = render(<DaemonPanel />);
    const btn = container.querySelector<HTMLButtonElement>(".daemon-panel__reload-btn");
    expect(btn?.disabled).toBe(false);
  });

  it("disables the Shutdown button when shuttingDown is true", () => {
    // Exercises the `disabled={shuttingDown}` prop when truthy.
    mockHook.mockReturnValue(makePanelState({ info: makeInfo(), shuttingDown: true }));
    const { container } = render(<DaemonPanel />);
    const btn = container.querySelector<HTMLButtonElement>(".daemon-panel__shutdown-btn");
    expect(btn?.disabled).toBe(true);
  });

  it("enables the Shutdown button when shuttingDown is false", () => {
    // Exercises the falsy arm of `disabled={shuttingDown}`.
    mockHook.mockReturnValue(makePanelState({ info: makeInfo(), shuttingDown: false }));
    const { container } = render(<DaemonPanel />);
    const btn = container.querySelector<HTMLButtonElement>(".daemon-panel__shutdown-btn");
    expect(btn?.disabled).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Conditional rendering: loading / error / info sections
// ---------------------------------------------------------------------------

describe("DaemonPanel – conditional rendering", () => {
  it("renders the loading indicator when loading is true", () => {
    mockHook.mockReturnValue(makePanelState({ loading: true }));
    const { container } = render(<DaemonPanel />);
    expect(container.querySelector(".daemon-panel__loading")).toBeTruthy();
  });

  it("renders the error message when error is set", () => {
    mockHook.mockReturnValue(makePanelState({ error: "connection refused" }));
    render(<DaemonPanel />);
    expect(screen.getByText("connection refused")).toBeTruthy();
  });

  it("renders the daemon info body when info is non-null", () => {
    mockHook.mockReturnValue(makePanelState({ info: makeInfo({ version: "2.3.4" }) }));
    render(<DaemonPanel />);
    expect(screen.getByText("v2.3.4")).toBeTruthy();
  });

  it("does not render the info body when info is null", () => {
    mockHook.mockReturnValue(makePanelState({ info: null }));
    const { container } = render(<DaemonPanel />);
    expect(container.querySelector(".daemon-panel__body")).toBeNull();
  });
});
