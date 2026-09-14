import { GitBranch } from "lucide-react";
import type { Account } from "../types";
import { providerNames } from "./providers";

export function LimitMonitor({ accounts, count }: { readonly accounts: readonly Account[]; readonly count: number }) {
  const limit = accounts.length ? Math.min(...accounts.map(account => account.capabilities.maxTextLength)) : 0;
  const remaining = limit - count;
  const ratio = limit ? Math.min(count / limit, 1) : 0;
  return <section className="panel limit-monitor" aria-label="Character limit monitor"><div className={`limit-ring ${remaining < 0 ? "over-limit" : ""}`}><svg viewBox="0 0 80 80" aria-hidden="true"><circle className="ring-track" cx="40" cy="40" r="34" /><circle className="ring-value" cx="40" cy="40" r="34" pathLength="100" strokeDasharray={`${ratio * 100} 100`} /></svg><div><strong>{limit ? Math.abs(remaining) : "—"}</strong><span>{remaining < 0 ? "OVER" : "LEFT"}</span></div></div>
    <div className="limit-summary"><span className="eyebrow">SHORTEST DESTINATION LIMIT</span><strong>{count.toLocaleString()} / {limit ? limit.toLocaleString() : "—"} <small>graphemes</small></strong><p>{remaining < 0 ? "Longer than one post. Your policy determines the thread." : "One idea, adapted for every selected destination."}</p></div>
    <div className="destination-meters">{accounts.map(account => <div className={`destination-meter ${account.provider.toLowerCase()}`} key={account.id}><div><span>{account.displayName} <small>· {providerNames[account.provider]}</small></span><span>{count} / {account.capabilities.maxTextLength}</span></div><progress aria-label={`${account.handle} character capacity`} max={account.capabilities.maxTextLength} value={Math.min(count, account.capabilities.maxTextLength)} /></div>)}</div>
    <span className="limit-policy"><GitBranch size={16} />Capability-aware<br />thread planning</span>
  </section>;
}
