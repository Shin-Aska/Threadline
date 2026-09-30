import { useEffect, useState, type RefObject } from "react";

/** Expands the page's rails when their headers scroll above the workspace header. */
export function useAdaptiveRailExpansion(primary: RefObject<HTMLElement | null>, secondary?: RefObject<HTMLElement | null>): boolean {
  const [expanded, setExpanded] = useState(false);

  useEffect(() => {
    const first = primary.current;
    const second = secondary?.current;
    if (!first || (secondary && !second)) return;

    const view = first.closest(".unified-page")?.parentElement;
    const update = () => {
      if (view?.hasAttribute("hidden") || !window.matchMedia("(min-width: 1181px)").matches) {
        setExpanded(false);
        return;
      }
      const headerHeight = Number.parseFloat(window.getComputedStyle(first).getPropertyValue("--workspace-header"));
      const railBottom = Math.max(first.getBoundingClientRect().bottom, second?.getBoundingClientRect().bottom ?? Number.NEGATIVE_INFINITY);
      setExpanded(current => railBottom < headerHeight + (current ? 16 : -8));
    };

    update();
    window.addEventListener("scroll", update, { passive: true });
    window.addEventListener("resize", update);
    const visibilityObserver = new MutationObserver(update);
    if (view) visibilityObserver.observe(view, { attributes: true, attributeFilter: ["hidden"] });
    const sizeObserver = new ResizeObserver(update);
    sizeObserver.observe(first);
    if (second) sizeObserver.observe(second);
    return () => {
      window.removeEventListener("scroll", update);
      window.removeEventListener("resize", update);
      visibilityObserver.disconnect();
      sizeObserver.disconnect();
    };
  }, [primary, secondary]);

  return expanded;
}
