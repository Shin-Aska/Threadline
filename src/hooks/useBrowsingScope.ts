import { useEffect, useMemo, useState } from "react";
import type { Account, Provider } from "../types";

export function useBrowsingScope(view: string, accounts: readonly Account[]) {
  const key = `threadline:browsing-scope:${view}`;
  const available = useMemo(() => new Set(accounts.map(account => account.id)), [accounts]);
  const [selected, setSelected] = useState<string[]>(() => {
    try { const saved = JSON.parse(localStorage.getItem(key) ?? "[]") as string[]; return saved.filter(id => available.has(id)); } catch { return []; }
  });
  const effective = useMemo(() => selected.length ? selected.filter(id => available.has(id)) : accounts.map(account => account.id), [selected, available, accounts]);
  useEffect(() => { localStorage.setItem(key, JSON.stringify(effective)); }, [key, effective]);
  const toggle = (id: string) => setSelected(current => { const base = current.length ? current : accounts.map(account => account.id); return base.includes(id) ? base.filter(value => value !== id) : [...base, id]; });
  const selectProvider = (provider: Provider | "ALL") => setSelected(accounts.filter(account => provider === "ALL" || account.provider === provider).map(account => account.id));
  return { selected: effective, toggle, selectProvider };
}
