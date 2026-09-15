import { useEffect, useRef, useState } from "react";
import { desktopApi } from "../services/desktop";
import type { HashtagSuggestion, WorkspaceState } from "../types";

export interface ActiveHashtag { readonly start: number; readonly end: number; readonly query: string }
export function activeHashtag(text: string, start: number, end: number): ActiveHashtag | null {
  if (start !== end) return null;
  const before = text.slice(0, start);
  const match = /(?:^|\s)[#＃]([\p{L}\p{M}\p{N}_]*)$/u.exec(before);
  const query = match?.[1];
  if (query === undefined || [...query].length > 64) return null;
  const tail = /^[\p{L}\p{M}\p{N}_]*/u.exec(text.slice(start))?.[0] ?? "";
  return { start: start - query.length - 1, end: start + tail.length, query };
}
export type HashtagResult = { readonly status: "ready"; readonly suggestions: readonly HashtagSuggestion[] }
  | { readonly status: "error"; readonly error: string };
export function useHashtags(query: string | null, accountIds: readonly string[], workspace: WorkspaceState) {
  const [results, setResults] = useState<{ readonly key: string; readonly items: Readonly<Record<string, HashtagResult>> }>({ key: "", items: {} });
  const retryRequest = useRef<(accountId: string) => void>(() => {});
  const cache = useRef(new Map<string, { readonly at: number; readonly suggestions: readonly HashtagSuggestion[] }>());
  const key = JSON.stringify([query, accountIds, workspace.connectedAccountIds]);
  useEffect(() => {
    if (query === null || workspace.mode === "BROWSER") return;
    let active = true;
    const request = (accountId: string, force = false) => {
        const save = (result: HashtagResult) => { if (active) setResults(previous => ({ key, items: { ...(previous.key === key ? previous.items : {}), [accountId]: result } })); };
        if (!workspace.connectedAccountIds.includes(accountId)) { save({ status: "error", error: "Reconnect this account for hashtag suggestions." }); return; }
        const cacheKey = JSON.stringify([accountId, query]);
        const cached = cache.current.get(cacheKey);
        if (!force && cached && Date.now() - cached.at < 60000) { save({ status: "ready", suggestions: cached.suggestions }); return; }
        void desktopApi.social.hashtags(accountId, query).then(suggestions => {
          if (!active) return;
          if (cache.current.size >= 30) { const oldest = cache.current.keys().next().value; if (oldest !== undefined) cache.current.delete(oldest); }
          cache.current.set(cacheKey, { at: Date.now(), suggestions });
          save({ status: "ready", suggestions });
        }).catch((cause: unknown) => save({ status: "error", error: cause instanceof Error ? cause.message : String(cause) }));
    };
    retryRequest.current = accountId => {
      if (!active || !accountIds.includes(accountId)) return;
      setResults(previous => {
        if (previous.key !== key) return previous;
        const items = { ...previous.items };
        delete items[accountId];
        return { key, items };
      });
      request(accountId, true);
    };
    const timer = setTimeout(() => { for (const accountId of accountIds) request(accountId); }, 600);
    return () => { active = false; clearTimeout(timer); };
  }, [key, query, accountIds, workspace]);
  return { results: results.key === key ? results.items : {}, retry: (accountId: string) => retryRequest.current(accountId) };
}
