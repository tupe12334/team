import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
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
    // Use the generic querySelector<T> overload — avoids both `as` casts and `!` assertions.
    const btn = container.querySelector<HTMLButtonElement>(".config-panel__save-btn");
    expect(btn?.disabled).toBe(true);
  });

  it("disables Save when isDirty is falsy (no unsaved changes)", () => {
    mockHook.mockReturnValue(makePanelState({ draft: makeDraft(), saving: false, isDirty: false }));
    const { container } = render(<ConfigPanel />);
    const btn = container.querySelector<HTMLButtonElement>(".config-panel__save-btn");
    expect(btn?.disabled).toBe(true);
  });

  it("enables Save when isDirty is true and not saving", () => {
    mockHook.mockReturnValue(makePanelState({ draft: makeDraft(), saving: false, isDirty: true }));
    const { container } = render(<ConfigPanel />);
    const btn = container.querySelector<HTMLButtonElement>(".config-panel__save-btn");
    expect(btn?.disabled).toBe(false);
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
    render(<ConfigPanel />);
    // getByDisplayValue finds the input by its current value — no cast needed.
    expect(screen.getByDisplayValue("review, qa, ship")).toBeTruthy();
  });

  it("shows an empty string when enabledAgents is empty", () => {
    mockHook.mockReturnValue(makePanelState({ draft: makeDraft({ enabledAgents: [] }) }));
    const { container } = render(<ConfigPanel />);
    // Use generic overload to get HTMLInputElement.value without `as` or `!`.
    const input = container.querySelector<HTMLInputElement>('input[placeholder="e.g. review, qa, ship"]');
    expect(input?.value).toBe("");
  });
});

// ---------------------------------------------------------------------------
// workersCount onChange: parseInt fallback and Math.max clamp branches
// ---------------------------------------------------------------------------

describe("ConfigPanel – workersCount input interaction", () => {
  it("calls setDraft with parsed integer for a valid positive input (parseInt truthy arm)", () => {
    // `Math.max(1, parseInt("8") || 1)` = `Math.max(1, 8)` = 8 — both || and max pass through.
    const setDraft = vi.fn();
    mockHook.mockReturnValue(makePanelState({ draft: makeDraft({ workersCount: 4 }), setDraft }));
    render(<ConfigPanel />);
    // type="number" inputs have ARIA role "spinbutton" — getByRole throws if absent (no ! needed).
    const input = screen.getByRole("spinbutton");
    fireEvent.change(input, { target: { value: "8" } });
    expect(setDraft).toHaveBeenCalledWith(expect.objectContaining({ workersCount: 8 }));
  });

  it("calls setDraft with 1 when input is cleared (parseInt('') → NaN → || 1 fallback)", () => {
    // `Math.max(1, parseInt("") || 1)` = `Math.max(1, NaN || 1)` = `Math.max(1, 1)` = 1
    // Exercises the falsy arm of `parseInt(e.target.value) || 1`.
    const setDraft = vi.fn();
    mockHook.mockReturnValue(makePanelState({ draft: makeDraft({ workersCount: 4 }), setDraft }));
    render(<ConfigPanel />);
    const input = screen.getByRole("spinbutton");
    fireEvent.change(input, { target: { value: "" } });
    expect(setDraft).toHaveBeenCalledWith(expect.objectContaining({ workersCount: 1 }));
  });

  it("calls setDraft with 1 when input is negative (Math.max clamp branch)", () => {
    // `Math.max(1, parseInt("-5") || 1)` = `Math.max(1, -5)` = 1
    // parseInt("-5") = -5 (truthy — the || arm passes through), but Math.max clamps it.
    // This is a distinct branch: || 1 does NOT fire, Math.max(1, ...) does.
    const setDraft = vi.fn();
    mockHook.mockReturnValue(makePanelState({ draft: makeDraft({ workersCount: 4 }), setDraft }));
    render(<ConfigPanel />);
    const input = screen.getByRole("spinbutton");
    fireEvent.change(input, { target: { value: "-5" } });
    expect(setDraft).toHaveBeenCalledWith(expect.objectContaining({ workersCount: 1 }));
  });
});

// ---------------------------------------------------------------------------
// enabledAgents onChange: split/trim/filter(Boolean) branches
// ---------------------------------------------------------------------------

describe("ConfigPanel – enabledAgents input interaction", () => {
  it("calls setDraft with trimmed non-empty strings when agents are comma-separated", () => {
    // split(",") → map(trim) → filter(Boolean): all three items survive — truthy filter arm.
    const setDraft = vi.fn();
    mockHook.mockReturnValue(makePanelState({ draft: makeDraft({ enabledAgents: [] }), setDraft }));
    render(<ConfigPanel />);
    const input = screen.getByPlaceholderText("e.g. review, qa, ship");
    fireEvent.change(input, { target: { value: "review, qa, ship" } });
    expect(setDraft).toHaveBeenCalledWith(
      expect.objectContaining({ enabledAgents: ["review", "qa", "ship"] })
    );
  });

  it("calls setDraft with empty array when field is cleared (filter drops the single empty string)", () => {
    // split(",") → [""]; map(trim) → [""]; filter(Boolean) → [] — falsy filter arm.
    const setDraft = vi.fn();
    mockHook.mockReturnValue(makePanelState({ draft: makeDraft({ enabledAgents: ["review"] }), setDraft }));
    render(<ConfigPanel />);
    const input = screen.getByPlaceholderText("e.g. review, qa, ship");
    fireEvent.change(input, { target: { value: "" } });
    expect(setDraft).toHaveBeenCalledWith(expect.objectContaining({ enabledAgents: [] }));
  });

  it("calls setDraft excluding the trailing empty entry from a trailing comma", () => {
    // "review, qa, " → split → ["review", " qa", " "]; trim → ["review", "qa", ""];
    // filter(Boolean) drops the trailing "" — confirms filter(Boolean) is doing real work.
    const setDraft = vi.fn();
    mockHook.mockReturnValue(makePanelState({ draft: makeDraft({ enabledAgents: [] }), setDraft }));
    render(<ConfigPanel />);
    const input = screen.getByPlaceholderText("e.g. review, qa, ship");
    fireEvent.change(input, { target: { value: "review, qa, " } });
    expect(setDraft).toHaveBeenCalledWith(
      expect.objectContaining({ enabledAgents: ["review", "qa"] })
    );
  });
});
