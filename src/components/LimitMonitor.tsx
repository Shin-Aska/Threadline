import type { Account } from "../types";
export function LimitMonitor({ accounts, count }: { readonly accounts: readonly Account[]; readonly count: number }) {
  const limit = accounts.length ? Math.min(...accounts.map(account => account.capabilities.maxTextLength)) : null;
  return <span className={"character-count" + (limit !== null && count > limit ? " over-limit" : "")} aria-label={"Character count: " + count + " graphemes" + (limit !== null ? ", shortest account limit " + limit : "")} title="Graphemes / shortest selected account limit. Your publishing policy determines threading.">{count.toLocaleString()} / {limit?.toLocaleString() ?? "—"}</span>;
}
