import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import type { WorkerStatusData, WorkerInfo } from "@/components/useWorkersPanel";

vi.mock("@/components/useWorkersPanel");
import { useWorkersPanel } from "@/components/useWorkersPanel";
const mockHook = vi.mocked(useWorkersPanel);

type Panel = ReturnType<typeof useWorkersPanel>;

function makePanelState(overrides: Partial<Panel> = {}): Panel {
  return { data: null, error: null, loading: false, ...overrides };
}

function makeData(overrides: Partial<WorkerStatusData> = {}): WorkerStatusData {
  return { total: 1, busy: 0, idle: 1, workers: [], ...overrides };
}

function makeWorker(overrides: Partial<WorkerInfo> = {}): WorkerInfo {
  return {
    workerId: "worker-abc123",
    status: "IDLE",
    currentTaskId: "",
    currentAgent: "",
    taskStartedAt: null,
    ...overrides,
  };
}

const { default: WorkersPanel } = await import("./WorkersPanel");

beforeEach(() => { vi.clearAllMocks(); });

// ---------------------------------------------------------------------------
// Conditional rendering: loading / error / data sections
// ---------------------------------------------------------------------------

describe("WorkersPanel – conditional rendering", () => {
  it("renders the loading indicator when loading is true", () => {
    mockHook.mockReturnValue(makePanelState({ loading: true }));
    const { container } = render(<WorkersPanel />);
    expect(container.querySelector(".workers-panel__loading")).toBeTruthy();
  });

  it("renders the error message when error is set", () => {
    mockHook.mockReturnValue(makePanelState({ error: "workers unavailable" }));
    render(<WorkersPanel />);
    expect(screen.getByText("workers unavailable")).toBeTruthy();
  });

  it("renders the stats grid when data is non-null", () => {
    mockHook.mockReturnValue(makePanelState({ data: makeData({ total: 3, busy: 1, idle: 2 }) }));
    const { container } = render(<WorkersPanel />);
    expect(container.querySelector(".workers-panel__stats")).toBeTruthy();
  });

  it("does not render the workers body when data is null", () => {
    mockHook.mockReturnValue(makePanelState({ data: null }));
    const { container } = render(<WorkersPanel />);
    expect(container.querySelector(".workers-panel__body")).toBeNull();
  });

  it("renders the empty-workers placeholder when workers list is empty", () => {
    // data.workers.length === 0 → `No workers registered`
    mockHook.mockReturnValue(makePanelState({ data: makeData({ workers: [] }) }));
    const { container } = render(<WorkersPanel />);
    expect(container.querySelector(".workers-panel__empty")).toBeTruthy();
  });

  it("renders a worker list item when workers list is non-empty", () => {
    mockHook.mockReturnValue(makePanelState({ data: makeData({ workers: [makeWorker()] }) }));
    const { container } = render(<WorkersPanel />);
    expect(container.querySelector(".worker-item")).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// Worker status badge: IDLE vs BUSY styling
// ---------------------------------------------------------------------------

describe("WorkersPanel – worker status badge", () => {
  it("renders IDLE badge for an idle worker (IDLE arm of status ternary)", () => {
    mockHook.mockReturnValue(makePanelState({
      data: makeData({ workers: [makeWorker({ status: "IDLE" })] }),
    }));
    render(<WorkersPanel />);
    expect(screen.getByText("IDLE")).toBeTruthy();
  });

  it("renders BUSY badge for a busy worker (non-IDLE arm of status ternary)", () => {
    mockHook.mockReturnValue(makePanelState({
      data: makeData({ workers: [makeWorker({ status: "BUSY", currentTaskId: "task-xyz" })] }),
    }));
    render(<WorkersPanel />);
    expect(screen.getByText("BUSY")).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// Worker task-info section: visible only for BUSY workers with a currentTaskId
// ---------------------------------------------------------------------------

describe("WorkersPanel – task-info visibility", () => {
  it("renders task info when status is BUSY and currentTaskId is set", () => {
    // `w.status === "BUSY" && w.currentTaskId` → true → task info rendered
    mockHook.mockReturnValue(makePanelState({
      data: makeData({ workers: [makeWorker({ status: "BUSY", currentTaskId: "task-abc" })] }),
    }));
    const { container } = render(<WorkersPanel />);
    expect(container.querySelector(".worker-item__task")).toBeTruthy();
    expect(screen.getByText("task-abc")).toBeTruthy();
  });

  it("does not render task info when status is BUSY but currentTaskId is empty", () => {
    // `w.status === "BUSY" && w.currentTaskId` → false (empty string is falsy)
    mockHook.mockReturnValue(makePanelState({
      data: makeData({ workers: [makeWorker({ status: "BUSY", currentTaskId: "" })] }),
    }));
    const { container } = render(<WorkersPanel />);
    expect(container.querySelector(".worker-item__task")).toBeNull();
  });

  it("does not render task info for an IDLE worker even with a currentTaskId", () => {
    // `w.status === "BUSY"` is false → entire block hidden
    mockHook.mockReturnValue(makePanelState({
      data: makeData({ workers: [makeWorker({ status: "IDLE", currentTaskId: "task-xyz" })] }),
    }));
    const { container } = render(<WorkersPanel />);
    expect(container.querySelector(".worker-item__task")).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Worker agent visibility: shown only when currentAgent is truthy
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Stats grid values: Total / Busy / Idle counts flow through to the DOM
// ---------------------------------------------------------------------------

describe("WorkersPanel – stats grid values", () => {
  it("renders the correct Total, Busy, and Idle counts in the stats grid", () => {
    // The existing stats-grid test only checks that `.workers-panel__stats` exists;
    // it never verifies that data.total/busy/idle actually appear in the rendered output.
    // This test exercises the `{value}` interpolation inside the workers-stat__value spans.
    mockHook.mockReturnValue(makePanelState({ data: makeData({ total: 5, busy: 2, idle: 3 }) }));
    render(<WorkersPanel />);
    expect(screen.getByText("5")).toBeTruthy(); // data.total → workers-stat__value
    expect(screen.getByText("2")).toBeTruthy(); // data.busy
    expect(screen.getByText("3")).toBeTruthy(); // data.idle
  });
});

describe("WorkersPanel – agent visibility", () => {
  it("shows agent name when currentAgent is set on a BUSY worker", () => {
    // `w.currentAgent` → truthy → renders `· agent: <span>{agent}</span>`
    mockHook.mockReturnValue(makePanelState({
      data: makeData({ workers: [makeWorker({ status: "BUSY", currentTaskId: "task-1", currentAgent: "review" })] }),
    }));
    render(<WorkersPanel />);
    expect(screen.getByText("review")).toBeTruthy();
  });

  it("does not show agent section when currentAgent is empty", () => {
    // `w.currentAgent` → falsy ("") → agent fragment not rendered
    mockHook.mockReturnValue(makePanelState({
      data: makeData({ workers: [makeWorker({ status: "BUSY", currentTaskId: "task-1", currentAgent: "" })] }),
    }));
    const { container } = render(<WorkersPanel />);
    expect(container.querySelector(".worker-item__agent")).toBeNull();
  });
});
