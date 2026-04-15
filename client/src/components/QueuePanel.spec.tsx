import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import type { IssueRef } from "@/gen/queue";
import type { Task, IssueProvider } from "@/components/useQueuePanel";

// Auto-mock the hook so component tests are isolated from network behavior.
vi.mock("@/components/useQueuePanel");
import { useQueuePanel } from "@/components/useQueuePanel";
const mockHook = vi.mocked(useQueuePanel);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

type Panel = ReturnType<typeof useQueuePanel>;

function makePanelState(overrides: Partial<Panel> = {}): Panel {
  return {
    tasks: [] as Task[],
    agents: [],
    error: null,
    loading: false,
    provider: "GITHUB" as IssueProvider,
    setProvider: vi.fn(),
    org: "", setOrg: vi.fn(),
    repo: "", setRepo: vi.fn(),
    number: "", setNumber: vi.fn(),
    url: "", setUrl: vi.fn(),
    agent: "", setAgent: vi.fn(),
    priority: 0, setPriority: vi.fn(),
    submitting: false,
    deletingId: null,
    handleEnqueue: vi.fn(),
    handleDelete: vi.fn(),
    ...overrides,
  };
}

function makeTask(overrides: Partial<Task> = {}): Task {
  return { id: "t1", status: 0, priority: 0, ...overrides };
}

// Lazy import after mock registration so the component uses the mocked hook.
const { default: QueuePanel } = await import("./QueuePanel");

beforeEach(() => { vi.clearAllMocks(); });

// ---------------------------------------------------------------------------
// formatIssueRef branches
// ---------------------------------------------------------------------------

describe("QueuePanel – formatIssueRef", () => {
  it("renders github ref as org/repo#number", () => {
    mockHook.mockReturnValue(makePanelState({
      tasks: [makeTask({
        issueRef: { github: { organization: "acme", repository: "app", number: "42" } } as IssueRef,
      })],
    }));
    render(<QueuePanel />);
    expect(screen.getByText("acme/app#42")).toBeTruthy();
  });

  it("renders centy ref as org/repo#number", () => {
    mockHook.mockReturnValue(makePanelState({
      tasks: [makeTask({
        issueRef: { centy: { organization: "corp", repository: "proj", number: "7" } } as IssueRef,
      })],
    }));
    render(<QueuePanel />);
    expect(screen.getByText("corp/proj#7")).toBeTruthy();
  });

  it("renders jira ref as JIRA:id", () => {
    mockHook.mockReturnValue(makePanelState({
      tasks: [makeTask({
        issueRef: { jira: { id: "PROJ-123" } } as IssueRef,
      })],
    }));
    render(<QueuePanel />);
    expect(screen.getByText("JIRA:PROJ-123")).toBeTruthy();
  });

  it("renders 'unknown' when no provider field is set on the issueRef", () => {
    // Exercises the `return ref.jira ? ... : "unknown"` fallback when all three
    // optional fields (github, centy, jira) are absent — e.g. a future ref type.
    mockHook.mockReturnValue(makePanelState({
      tasks: [makeTask({ issueRef: {} as IssueRef })],
    }));
    render(<QueuePanel />);
    expect(screen.getByText("unknown")).toBeTruthy();
  });

  it("renders a dash element when task.issueRef is undefined", () => {
    // `task.issueRef ? formatIssueRef(task.issueRef) : dash` — the falsy arm.
    mockHook.mockReturnValue(makePanelState({
      tasks: [makeTask({ issueRef: undefined })],
    }));
    const { container } = render(<QueuePanel />);
    expect(container.querySelector(".queue-table__dash")).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// agentOrDash branches
// ---------------------------------------------------------------------------

describe("QueuePanel – agentOrDash", () => {
  it("renders the agent name when task.agent is a non-empty string (truthy arm)", () => {
    mockHook.mockReturnValue(makePanelState({
      tasks: [makeTask({ agent: "review" })],
    }));
    render(<QueuePanel />);
    expect(screen.getByText("review")).toBeTruthy();
  });

  it("renders a dash element when task.agent is undefined (falsy arm)", () => {
    mockHook.mockReturnValue(makePanelState({
      tasks: [makeTask({ agent: undefined })],
    }));
    const { container } = render(<QueuePanel />);
    // At least one dash placeholder must be present (issue-ref cell and agent cell)
    expect(container.querySelectorAll(".queue-table__dash").length).toBeGreaterThanOrEqual(1);
  });
});

// ---------------------------------------------------------------------------
// STATUS map vs FALLBACK_STATUS
// ---------------------------------------------------------------------------

describe("QueuePanel – STATUS badges", () => {
  it("renders QUEUED badge for status 0", () => {
    mockHook.mockReturnValue(makePanelState({ tasks: [makeTask({ status: 0 })] }));
    render(<QueuePanel />);
    expect(screen.getByText("QUEUED")).toBeTruthy();
  });

  it("renders RUNNING badge for status 1", () => {
    mockHook.mockReturnValue(makePanelState({ tasks: [makeTask({ status: 1 })] }));
    render(<QueuePanel />);
    expect(screen.getByText("RUNNING")).toBeTruthy();
  });

  it("renders COMPLETED badge for status 2", () => {
    mockHook.mockReturnValue(makePanelState({ tasks: [makeTask({ status: 2 })] }));
    render(<QueuePanel />);
    expect(screen.getByText("COMPLETED")).toBeTruthy();
  });

  it("renders FAILED badge for status 3", () => {
    mockHook.mockReturnValue(makePanelState({ tasks: [makeTask({ status: 3 })] }));
    render(<QueuePanel />);
    expect(screen.getByText("FAILED")).toBeTruthy();
  });

  it("renders UNKNOWN badge for unrecognized status codes (FALLBACK_STATUS)", () => {
    // task.status=99 is not in the STATUS map → falls through to FALLBACK_STATUS.
    mockHook.mockReturnValue(makePanelState({ tasks: [makeTask({ status: 99 })] }));
    render(<QueuePanel />);
    expect(screen.getByText("UNKNOWN")).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// Conditional rendering: loading / empty / error / task table
// ---------------------------------------------------------------------------

describe("QueuePanel – conditional rendering", () => {
  it("renders the loading indicator when loading is true", () => {
    mockHook.mockReturnValue(makePanelState({ loading: true }));
    const { container } = render(<QueuePanel />);
    expect(container.querySelector(".queue-panel__loading")).toBeTruthy();
  });

  it("renders the empty-queue placeholder when tasks list is empty and not loading", () => {
    mockHook.mockReturnValue(makePanelState({ tasks: [], loading: false }));
    const { container } = render(<QueuePanel />);
    expect(container.querySelector(".queue-panel__empty")).toBeTruthy();
  });

  it("renders the error message when error is set", () => {
    mockHook.mockReturnValue(makePanelState({ error: "daemon unreachable" }));
    render(<QueuePanel />);
    expect(screen.getByText("daemon unreachable")).toBeTruthy();
  });

  it("renders the task table when tasks are present and not loading", () => {
    mockHook.mockReturnValue(makePanelState({ tasks: [makeTask()], loading: false }));
    const { container } = render(<QueuePanel />);
    expect(container.querySelector(".queue-table")).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// EnqueueForm provider branches
// ---------------------------------------------------------------------------

describe("QueuePanel – EnqueueForm provider switch", () => {
  it("renders org/repo/number inputs for GITHUB provider", () => {
    mockHook.mockReturnValue(makePanelState({ provider: "GITHUB" }));
    const { container } = render(<QueuePanel />);
    expect(container.querySelector('input[placeholder="org"]')).toBeTruthy();
    expect(container.querySelector('input[placeholder="repo"]')).toBeTruthy();
    expect(container.querySelector('input[placeholder="#"]')).toBeTruthy();
  });

  it("renders org/repo/number inputs for CENTY provider", () => {
    mockHook.mockReturnValue(makePanelState({ provider: "CENTY" }));
    const { container } = render(<QueuePanel />);
    expect(container.querySelector('input[placeholder="org"]')).toBeTruthy();
  });

  it("renders URL input instead of org/repo/number for LINK provider", () => {
    // Exercises the `else` branch: `provider !== "GITHUB" && provider !== "CENTY"`
    mockHook.mockReturnValue(makePanelState({ provider: "LINK" }));
    const { container } = render(<QueuePanel />);
    expect(container.querySelector('input[type="url"]')).toBeTruthy();
    expect(container.querySelector('input[placeholder="org"]')).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Submit button label: "Adding…" vs "+ Enqueue" — both arms of the ternary
// ---------------------------------------------------------------------------

describe("QueuePanel – submit button label", () => {
  it("shows '+ Enqueue' when submitting is false (default arm)", () => {
    mockHook.mockReturnValue(makePanelState({ submitting: false }));
    render(<QueuePanel />);
    expect(screen.getByText("+ Enqueue")).toBeTruthy();
  });

  it("shows 'Adding…' and is disabled when submitting is true (submitting arm)", () => {
    mockHook.mockReturnValue(makePanelState({ submitting: true }));
    const { container } = render(<QueuePanel />);
    expect(screen.getByText("Adding…")).toBeTruthy();
    const btn = container.querySelector<HTMLButtonElement>(".queue-panel__submit");
    expect(btn?.disabled).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Delete button label: "…" vs "✕" — both arms of the ternary per task row
// ---------------------------------------------------------------------------

describe("QueuePanel – delete button label", () => {
  it("shows '✕' when deletingId does not match the task id (idle arm)", () => {
    mockHook.mockReturnValue(makePanelState({
      tasks: [makeTask({ id: "t1" })],
      deletingId: null,
    }));
    render(<QueuePanel />);
    expect(screen.getByText("✕")).toBeTruthy();
  });

  it("shows '…' and is disabled when deletingId matches the task id (deleting arm)", () => {
    mockHook.mockReturnValue(makePanelState({
      tasks: [makeTask({ id: "t1" })],
      deletingId: "t1",
    }));
    const { container } = render(<QueuePanel />);
    expect(screen.getByText("…")).toBeTruthy();
    const btn = container.querySelector<HTMLButtonElement>(".queue-table__delete");
    expect(btn?.disabled).toBe(true);
  });

  it("shows '✕' (enabled) for unrelated task when another task is being deleted", () => {
    // Two rows: t1 is deleting (deletingId="t1"), t2 is idle — t2's button must not be disabled.
    mockHook.mockReturnValue(makePanelState({
      tasks: [makeTask({ id: "t1" }), makeTask({ id: "t2" })],
      deletingId: "t1",
    }));
    const { container } = render(<QueuePanel />);
    const buttons = container.querySelectorAll<HTMLButtonElement>(".queue-table__delete");
    const deletingBtn = buttons[0]; // t1 → "…", disabled
    const idleBtn = buttons[1];     // t2 → "✕", enabled
    expect(deletingBtn.disabled).toBe(true);
    expect(idleBtn.disabled).toBe(false);
  });
});
