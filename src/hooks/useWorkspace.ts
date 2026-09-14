import { useCallback, useEffect, useRef, useState } from "react";
import { desktopApi } from "../services/desktop";
import { useComposerStore } from "../stores/composer";
import type { WorkspaceState } from "../types";

export function useWorkspace() {
  const [workspace, setWorkspace] = useState<WorkspaceState | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const request = useRef(0);
  const initialized = useRef(false);
  const refresh = useCallback(async (preferredId?: string) => {
    const current = ++request.current;
    setRefreshing(true); setError(null);
    try {
      const next = await desktopApi.workspace.load();
      if (current !== request.current) return false;
      const availableIds = next.accounts.map(account => account.id);
      const defaults = next.mode === "LIVE" || next.mode === "DISCONNECTED" ? next.connectedAccountIds : availableIds;
      const selected = initialized.current ? useComposerStore.getState().selected.filter(id => availableIds.includes(id)) : [...defaults];
      if (preferredId && availableIds.includes(preferredId) && !selected.includes(preferredId)) selected.push(preferredId);
      useComposerStore.getState().setSelected(selected);
      initialized.current = true; setWorkspace(next);
      return true;
    } catch (cause) {
      if (current === request.current) setError(cause instanceof Error ? cause.message : String(cause));
      return false;
    } finally { if (current === request.current) setRefreshing(false); }
  }, []);
  useEffect(() => {
    void refresh();
    return () => { request.current += 1; };
  }, [refresh]);
  return { workspace, error, refreshing, refresh };
}
