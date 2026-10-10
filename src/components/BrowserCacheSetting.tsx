import { useCallback, useEffect, useRef, useState } from "react";
import { desktopApi } from "../services/desktop";

type ClearState = { kind: "ready" | "clearing" | "cleared" } | { kind: "failed"; message: string };
type SizeState = { kind: "loading" } | { kind: "unavailable" } | { kind: "measured"; bytes: number };

function formatBytes(bytes: number): string {
  const units = ["B", "KiB", "MiB", "GiB", "TiB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${new Intl.NumberFormat(undefined, { maximumFractionDigits: 1 }).format(value)} ${units[unit]}`;
}

export function BrowserCacheSetting() {
  const [state, setState] = useState<ClearState>({ kind: "ready" });
  const [size, setSize] = useState<SizeState>({ kind: "loading" });
  const pending = useRef(false);
  const sizeRequest = useRef(0);
  const desktop = "__TAURI_INTERNALS__" in window;

  const refreshSize = useCallback(async () => {
    const request = ++sizeRequest.current;
    setSize({ kind: "loading" });
    try {
      const bytes = await desktopApi.storage.browserCacheSize();
      if (request !== sizeRequest.current) return;
      setSize(bytes === null ? { kind: "unavailable" } : { kind: "measured", bytes });
    } catch {
      if (request === sizeRequest.current) setSize({ kind: "unavailable" });
    }
  }, []);

  useEffect(() => {
    if (desktop) void refreshSize();
    return () => { sizeRequest.current += 1; };
  }, [desktop, refreshSize]);

  const clearCache = async () => {
    if (!desktop || pending.current) return;
    pending.current = true;
    setState({ kind: "clearing" });
    try {
      await desktopApi.storage.clearBrowserCache();
      setState({ kind: "cleared" });
      void refreshSize();
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : typeof cause === "string" ? cause : "Could not clear the browser cache. Try again.";
      setState({ kind: "failed", message });
    } finally {
      pending.current = false;
    }
  };

  return <section className="panel settings-panel" aria-labelledby="settings-browser-heading">
    <div className="settings-panel-heading"><h2 id="settings-browser-heading">Browser cache</h2><p>The embedded browser can store images and other web resources on disk.</p></div>
    {desktop && <p className="settings-help" aria-live="polite">
      {size.kind === "loading" ? "Calculating disk cache size…" : size.kind === "unavailable" ? "Disk cache size unavailable." : `Disk cache used: ${formatBytes(size.bytes)} (approx.)`}
    </p>}
    <button className="button" type="button" disabled={!desktop || state.kind === "clearing"} onClick={() => { void clearCache(); }}>
      {state.kind === "clearing" ? "Clearing browser cache…" : "Clear browser cache"}
    </button>
    <p className="settings-help">Accounts, drafts, publishing history and preferences are kept.</p>
    {!desktop && <p className="settings-help">Available in the desktop app.</p>}
    {state.kind === "clearing" && <p className="settings-help" role="status">Clearing browser cache…</p>}
    {state.kind === "cleared" && <p className="settings-help" role="status">Browser cache cleared.</p>}
    {state.kind === "failed" && <p className="inline-error" role="alert">{state.message}</p>}
  </section>;
}
