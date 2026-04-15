import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import type { DaemonConfig } from "@/components/useConfigPanel";

vi.mock("@/components/useConfigPanel");
import { useConfigPanel } from "@/components/useConfigPanel";
const mockHook = vi.mocked(useConfigPanel);

type Panel = ReturnType<typeof useConfigPanel>;

function makePanelState(overrides: Partial<Panel> = {}): Panel {
  return {
    draft: null,
    setDraft: vi.fn(),
    error: null,
    loading: false,
    saving: false,
    saved: false,
    isDirty: null,
    handleSave: vi.fn(),
    handleReset: vi.fn(),
    ...overrides,
  };
}

function makeDraft(overrides: Partial<DaemonConfig> = {}): DaemonConfig {
  return { workersCount: 4, logLevel: "info", enabledAgents: [], ...overrides };
}

const { default: ConfigPanel } = await import("./ConfigPanel");

beforeEach(() => { vi.clearAllMocks(); });

// ---------------------------------------------------------------------------
// Conditional rendering: loading / error / draft (ConfigForm) sections
// ---------------------------------------------------------------------------

describe("ConfigPanel – conditional rendering", () => {
  it("renders the loading indicator when loading is true", () => {
    mockHook.mockReturnValue(makePanelState({ loading: true }));
    const { container } = render(<ConfigPanel />);
    expect(container.querySelector(".config-panel__loading")).toBeTruthy();
  });

  it("renders the error message when error is set", () => {
    mockHook.mockReturnValue(makePanelState({ error: "config unreachable" }));
    render(<ConfigPanel />);
    expect(screen.getByText("config unreachable")).toBeTruthy();
  });

  it("renders the config form body when draft is non-null", () => {
    mockHook.mockReturnValue(makePanelState({ draft: makeDraft() }));
    const { container } = render(<ConfigPanel />);
    expect(container.querySelector(".config-panel__body")).toBeTruthy();
  });

  it("does not render the config form when draft is null", () => {
    mockHook.mockReturnValue(makePanelState({ draft: null }));
    const { container } = render(<ConfigPanel />);
    expect(container.querySelector(".config-panel__body")).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Save button label: saving / saved / default — nested ternary arms
// ---------------------------------------------------------------------------

describe("ConfigPanel – save button label", () => {
  it("shows 'Save' when neither saving nor saved (outer-false, inner-false)", () => {
    mockHook.mockReturnValue(makePanelState({ draft: makeDraft(), saving: false, saved: false, isDirty: true }));
    render(<ConfigPanel />);
    expect(screen.getByText("Save")).toBeTruthy();
  });

  it("shows 'Saving…' when saving is true (outer-true arm)", () => {
    mockHook.mockReturnValue(makePanelState({ draft: makeDraft(), saving: true, saved: false, isDirty: true }));
    render(<ConfigPanel />);
    expect(screen.getByText("Saving…")).toBeTruthy();
  });

  it("shows 'Saved' when saving is false and saved is true (outer-false, inner-true arm)", () => {
    mockHook.mockReturnValue(makePanelState({ draft: makeDraft(), saving: false, saved: true, isDirty: true }));
    render(<ConfigPanel />);
    expect(screen.getByText("Saved")).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// Reset button visibility: only rendered when isDirty is truthy
// ---------------------------------------------------------------------------

describe("ConfigPanel – Reset button visibility", () => {
  it("shows the Reset button when isDirty is true", () => {
    mockHook.mockReturnValue(makePanelState({ draft: makeDraft(), isDirty: true }));
    const { container } = render(<ConfigPanel />);
    expect(container.querySelector(".config-panel__reset-btn")).toBeTruthy();
  });

  it("hides the Reset button when isDirty is false", () => {
    mockHook.mockReturnValue(makePanelState({ draft: makeDraft(), isDirty: false }));
    const { container } = render(<ConfigPanel />);
    expect(container.querySelector(".config-panel__reset-btn")).toBeNull();
  });

  it("hides the Reset button when isDirty is null (initial state before config loads)", () => {
    // isDirty is `boolean | null` — null before config and draft are both present
    mockHook.mockReturnValue(makePanelState({ draft: makeDraft(), isDirty: null }));
    const { container } = render(<ConfigPanel />);
    expect(container.querySelector(".config-panel__reset-btn")).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Save button disabled state: disabled when saving OR when not isDirty
// ---------------------------------------------------------------------------

describe("ConfigPanel – save button disabled state", () => {
  it("disables Save when saving is true", () => {
    mockHook.mockReturnValue(makePanelState({ draft: makeDraft(), saving: true, isDirty: true }));
    const { container } = render(<ConfigPanel />);
    const btn = container.querySelector(".config-panel__save-btn") as HTMLButtonElement;
    expect(btn.disabled).toBe(true);
  });

  it("disables Save when isDirty is falsy (no unsaved changes)", () => {
    mockHook.mockReturnValue(makePanelState({ draft: makeDraft(), saving: false, isDirty: false }));
    const { container } = render(<ConfigPanel />);
    const btn = container.querySelector(".config-panel__save-btn") as HTMLButtonElement;
    expect(btn.disabled).toBe(true);
  });

  it("enables Save when isDirty is true and not saving", () => {
    mockHook.mockReturnValue(makePanelState({ draft: makeDraft(), saving: false, isDirty: true }));
    const { container } = render(<ConfigPanel />);
    const btn = container.querySelector(".config-panel__save-btn") as HTMLButtonElement;
    expect(btn.disabled).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// enabledAgents display value
// ---------------------------------------------------------------------------

describe("ConfigPanel – enabledAgents field", () => {
  it("shows comma-separated agents in the enabled-agents input", () => {
    mockHook.mockReturnValue(makePanelState({
      draft: makeDraft({ enabledAgents: ["review", "qa", "ship"] }),
    }));
    const { container } = render(<ConfigPanel />);
    const input = container.querySelector('input[placeholder]') as HTMLInputElement;
    expect(input.value).toBe("review, qa, ship");
  });

  it("shows an empty string when enabledAgents is empty", () => {
    mockHook.mockReturnValue(makePanelState({ draft: makeDraft({ enabledAgents: [] }) }));
    const { container } = render(<ConfigPanel />);
    // Find the agents text input (placeholder="e.g. review, qa, ship")
    const input = container.querySelector('input[placeholder="e.g. review, qa, ship"]') as HTMLInputElement;
    expect(input.value).toBe("");
  });
});
