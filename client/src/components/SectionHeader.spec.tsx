import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import SectionHeader from "./SectionHeader";

// SectionHeader has two branches on the `right` prop:
//   1. right is provided   → the node is rendered alongside the title
//   2. right is undefined  → nothing extra is rendered

describe("SectionHeader", () => {
  it("renders the title text passed as children", () => {
    render(<SectionHeader>Queue</SectionHeader>);
    expect(screen.getByText("Queue")).toBeTruthy();
  });

  it("renders the right slot when the right prop is provided (truthy arm)", () => {
    // Exercises the `{right}` expression when right is a non-null ReactNode.
    render(
      <SectionHeader right={<span className="test-right-slot" data-testid="right-slot">live · 5s</span>}>
        Workers
      </SectionHeader>
    );
    expect(screen.getByTestId("right-slot")).toBeTruthy();
    expect(screen.getByText("live · 5s")).toBeTruthy();
  });

  it("renders nothing in the right slot when right is undefined (falsy arm)", () => {
    // Exercises the `{right}` expression when right is undefined — the default arm
    // that ConfigPanel and DaemonPanel rely on (they call <SectionHeader> without a right prop).
    const { container } = render(<SectionHeader>Config</SectionHeader>);
    // Only one child inside .section-header: the __left div; no extra sibling.
    const header = container.querySelector(".section-header");
    // section-header__left is always present; a right-slot element must not be.
    expect(header?.querySelector(".section-header__left")).toBeTruthy();
    expect(header?.children.length).toBe(1);
  });
});
