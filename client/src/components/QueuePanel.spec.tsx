import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
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
// EnqueueForm agent select: agents.map and title resolution
// ---------------------------------------------------------------------------

describe("QueuePanel – EnqueueForm agent select", () => {
  it("renders agent <option> elements from the agents list when non-empty", () => {
    // Exercises agents.map(a => <option>{a.name}</option>) — all other tests use
    // agents:[] so the map body never runs and no agent options are ever rendered.
    mockHook.mockReturnValue(makePanelState({
      agents: [
        { name: "review", description: "Code review" },
        { name: "qa", description: "Quality assurance" },
      ],
    }));
    render(<QueuePanel />);
    expect(screen.getByText("review")).toBeTruthy();
    expect(screen.getByText("qa")).toBeTruthy();
  });

  it("sets the agent select title to the matched description when agent matches (?.description truthy arm)", () => {
    // agents.find((a) => a.name === agent)?.description returns the description string
    // when agent matches an entry.  All other tests have agents:[] so find() always
    // returns undefined and ?.description short-circuits to undefined (falsy arm only).
    mockHook.mockReturnValue(makePanelState({
      agents: [{ name: "review", description: "Code review" }],
      agent: "review",
    }));
    const { container } = render(<QueuePanel />);
    const select = container.querySelector<HTMLSelectElement>(".queue-panel__agent");
    expect(select?.title).toBe("Code review");
  });

  it("renders the priority input with an empty value when priority is 0 (falsy || arm)", () => {
    // `value={priority || ""}` — when priority=0 the input must show "" so the
    // placeholder is visible and the field appears empty.  All tests default priority
    // to 0 but none explicitly assert the rendered value is "".
    mockHook.mockReturnValue(makePanelState({ priority: 0 }));
    const { container } = render(<QueuePanel />);
    const input = container.querySelector<HTMLInputElement>('input[placeholder="priority"]');
    expect(input?.value).toBe("");
  });

  it("renders the priority input with the numeric value when priority is non-zero (truthy || arm)", () => {
    // `value={priority || ""}` — when priority=5 the expression evaluates to 5
    // (truthy), so the input shows the number.  Only the falsy (priority=0 → "")
    // arm is covered by the default panel state in other tests.
    mockHook.mockReturnValue(makePanelState({ priority: 5 }));
    const { container } = render(<QueuePanel />);
    const input = container.querySelector<HTMLInputElement>('input[placeholder="priority"]');
    expect(input?.value).toBe("5");
  });
});

// ---------------------------------------------------------------------------
// Priority input onChange: Math.trunc / Number() / || 0 / Math.max(0) branches
// ---------------------------------------------------------------------------

describe("QueuePanel – priority input interaction", () => {
  it("calls setPriority with the parsed integer for a valid positive input (happy path)", () => {
    // Math.max(0, Math.trunc(Number("5")) || 0) = Math.max(0, 5) = 5
    const setPriority = vi.fn();
    mockHook.mockReturnValue(makePanelState({ setPriority }));
    render(<QueuePanel />);
    fireEvent.change(screen.getByPlaceholderText("priority"), { target: { value: "5" } });
    expect(setPriority).toHaveBeenCalledWith(5);
  });

  it("calls setPriority with 0 when input is cleared (empty string → NaN → || 0 fallback)", () => {
    // Number("") = 0; Math.trunc(0) = 0; 0 || 0 = 0 (falsy arm taken)
    // Start with priority:5 so the input is "5" and clearing it is a real change.
    const setPriority = vi.fn();
    mockHook.mockReturnValue(makePanelState({ priority: 5, setPriority }));
    render(<QueuePanel />);
    fireEvent.change(screen.getByPlaceholderText("priority"), { target: { value: "" } });
    expect(setPriority).toHaveBeenCalledWith(0);
  });

  it("calls setPriority with 0 when input is negative (Math.max clamp branch)", () => {
    // Math.trunc(Number("-3")) = -3; -3 || 0 = -3 (truthy, || passes through);
    // Math.max(0, -3) = 0 — the || arm does NOT fire, Math.max clamps it.
    const setPriority = vi.fn();
    mockHook.mockReturnValue(makePanelState({ setPriority }));
    render(<QueuePanel />);
    fireEvent.change(screen.getByPlaceholderText("priority"), { target: { value: "-3" } });
    expect(setPriority).toHaveBeenCalledWith(0);
  });

  it("calls setPriority with the truncated integer for a decimal input (Math.trunc branch)", () => {
    // Number("5.7") = 5.7; Math.trunc(5.7) = 5; 5 || 0 = 5; Math.max(0, 5) = 5
    const setPriority = vi.fn();
    mockHook.mockReturnValue(makePanelState({ setPriority }));
    render(<QueuePanel />);
    fireEvent.change(screen.getByPlaceholderText("priority"), { target: { value: "5.7" } });
    expect(setPriority).toHaveBeenCalledWith(5);
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

// ---------------------------------------------------------------------------
// Handler wiring: onClick / onChange connections to hook callbacks
// ---------------------------------------------------------------------------

describe("QueuePanel – handler wiring", () => {
  it("calls handleDelete with the task id when the delete button is clicked", () => {
    // onClick={() => { void handleDelete(task.id); }} — id must be forwarded correctly.
    // All other delete tests only check the label/disabled state; none verify the handler fires.
    const handleDelete = vi.fn().mockResolvedValue(undefined);
    mockHook.mockReturnValue(makePanelState({
      tasks: [makeTask({ id: "t1" })],
      handleDelete,
    }));
    render(<QueuePanel />);
    fireEvent.click(screen.getByText("✕"));
    expect(handleDelete).toHaveBeenCalledWith("t1");
  });

  it("calls setProvider when the provider select changes", () => {
    // onChange={(e) => { setProvider(e.target.value as typeof provider); }}
    // No existing test fires a change event on the provider select.
    const setProvider = vi.fn();
    mockHook.mockReturnValue(makePanelState({ provider: "GITHUB", setProvider }));
    render(<QueuePanel />);
    // getAllByRole("combobox") returns [providerSelect, agentSelect] in DOM order.
    fireEvent.change(screen.getAllByRole("combobox")[0], { target: { value: "LINK" } });
    expect(setProvider).toHaveBeenCalledWith("LINK");
  });

  it("calls setAgent when the agent select changes", () => {
    // onChange={(e) => { setAgent(e.target.value); }}
    // No existing test fires a change event on the agent select.
    const setAgent = vi.fn();
    mockHook.mockReturnValue(makePanelState({
      agents: [{ name: "review", description: "Code review" }],
      setAgent,
    }));
    render(<QueuePanel />);
    // getAllByRole("combobox") returns [providerSelect, agentSelect] in DOM order.
    fireEvent.change(screen.getAllByRole("combobox")[1], { target: { value: "review" } });
    expect(setAgent).toHaveBeenCalledWith("review");
  });

  it("calls setOrg when the org input changes (GITHUB provider)", () => {
    // onChange={(e) => { setOrg(e.target.value); }} — no test previously fired on this input.
    const setOrg = vi.fn();
    mockHook.mockReturnValue(makePanelState({ provider: "GITHUB", setOrg }));
    render(<QueuePanel />);
    fireEvent.change(screen.getByPlaceholderText("org"), { target: { value: "acme" } });
    expect(setOrg).toHaveBeenCalledWith("acme");
  });

  it("calls setRepo when the repo input changes (GITHUB provider)", () => {
    // onChange={(e) => { setRepo(e.target.value); }}
    const setRepo = vi.fn();
    mockHook.mockReturnValue(makePanelState({ provider: "GITHUB", setRepo }));
    render(<QueuePanel />);
    fireEvent.change(screen.getByPlaceholderText("repo"), { target: { value: "my-app" } });
    expect(setRepo).toHaveBeenCalledWith("my-app");
  });

  it("calls setNumber when the issue number input changes (GITHUB provider)", () => {
    // onChange={(e) => { setNumber(e.target.value); }}
    const setNumber = vi.fn();
    mockHook.mockReturnValue(makePanelState({ provider: "GITHUB", setNumber }));
    render(<QueuePanel />);
    fireEvent.change(screen.getByPlaceholderText("#"), { target: { value: "42" } });
    expect(setNumber).toHaveBeenCalledWith("42");
  });

  it("calls setUrl when the URL input changes (LINK provider)", () => {
    // onChange={(e) => { setUrl(e.target.value); }} — only rendered when provider === "LINK".
    const setUrl = vi.fn();
    mockHook.mockReturnValue(makePanelState({ provider: "LINK", setUrl }));
    render(<QueuePanel />);
    fireEvent.change(screen.getByPlaceholderText("url"), { target: { value: "https://github.com/acme/app/issues/1" } });
    expect(setUrl).toHaveBeenCalledWith("https://github.com/acme/app/issues/1");
  });
});
