import { useLayoutEffect, useState } from "react";
import type { RefObject } from "react";

const metrics = ["fontFamily", "fontSize", "fontWeight", "fontStyle", "fontVariant", "letterSpacing", "lineHeight", "textIndent", "textTransform", "tabSize", "paddingTop", "paddingRight", "paddingBottom", "paddingLeft", "borderTopWidth", "borderRightWidth", "borderBottomWidth", "borderLeftWidth", "boxSizing"] as const;
export function useHashtagAnchor(editor: RefObject<HTMLTextAreaElement | null>, popup: RefObject<HTMLDivElement | null>, text: string, index: number) {
  const [position, setPosition] = useState({ left: 8, top: 8, width: 360, maxHeight: 400, visible: false });
  useLayoutEffect(() => {
    const input = editor.current;
    const panel = popup.current;
    if (!input || !panel) return;
    const update = () => {
      const rect = input.getBoundingClientRect();
      const style = getComputedStyle(input);
      const mirror = document.createElement("div");
      for (const property of metrics) mirror.style[property] = style[property];
      Object.assign(mirror.style, { position: "fixed", visibility: "hidden", pointerEvents: "none", whiteSpace: "pre-wrap", overflowWrap: "break-word", borderStyle: "solid", left: rect.left + "px", top: rect.top + "px", width: input.clientWidth + parseFloat(style.borderLeftWidth) + parseFloat(style.borderRightWidth) + "px" });
      mirror.textContent = text.slice(0, index);
      const marker = document.createElement("span");
      marker.textContent = text.slice(index) || "\u200b";
      mirror.append(marker);
      document.body.append(mirror);
      const point = marker.getClientRects()[0] ?? marker.getBoundingClientRect();
      const x = point.left - input.scrollLeft;
      const y = point.top - input.scrollTop;
      mirror.remove();
      const viewport = window.visualViewport;
      const leftEdge = (viewport?.offsetLeft ?? 0) + 8;
      const topEdge = (viewport?.offsetTop ?? 0) + 8;
      const rightEdge = leftEdge + (viewport?.width ?? innerWidth) - 16;
      const bottomEdge = topEdge + (viewport?.height ?? innerHeight) - 16;
      const width = Math.min(360, rightEdge - leftEdge);
      const lineHeight = parseFloat(style.lineHeight);
      const below = bottomEdge - y - lineHeight - 6;
      const above = y - topEdge - 6;
      const openAbove = below < Math.min(300, panel.scrollHeight) && above > below;
      const maxHeight = Math.min(400, Math.max(0, openAbove ? above : below));
      const height = Math.min(panel.scrollHeight, maxHeight);
      setPosition({ left: Math.max(leftEdge, Math.min(x, rightEdge - width)), top: openAbove ? y - height - 6 : y + lineHeight + 6, width, maxHeight, visible: y >= Math.max(rect.top, topEdge) && y < Math.min(rect.bottom, bottomEdge) && maxHeight >= 100 });
    };
    update();
    const observer = new ResizeObserver(update);
    observer.observe(input); observer.observe(panel);
    window.addEventListener("scroll", update, true);
    window.addEventListener("resize", update);
    window.visualViewport?.addEventListener("resize", update);
    window.visualViewport?.addEventListener("scroll", update);
    return () => {
      observer.disconnect();
      window.removeEventListener("scroll", update, true);
      window.removeEventListener("resize", update);
      window.visualViewport?.removeEventListener("resize", update);
      window.visualViewport?.removeEventListener("scroll", update);
    };
  }, [editor, popup, text, index]);
  return position;
}
