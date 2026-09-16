import { Check, ChevronDown } from "lucide-react";
import { useState } from "react";
import type { Account, Provider } from "../../types";
import { ProviderIcon } from "../ui";

export function SourceControls({ accounts, selected, onToggle, onSelectProvider }: { accounts: readonly Account[]; selected: readonly string[]; onToggle: (id: string) => void; onSelectProvider: (provider: Provider | "ALL") => void }) {
  const [open, setOpen] = useState<Provider | null>(null);
  const count = (provider: Provider) => accounts.filter(account => account.provider === provider && selected.includes(account.id)).length;
  return <div className="source-controls" aria-label="Browsing sources">
    <button className={`source-chip ${selected.length === accounts.length ? "active" : ""}`} onClick={() => onSelectProvider("ALL")}>All sources</button>
    {(["BLUESKY", "MASTODON"] as const).map(provider => <div className="source-popover" key={provider}>
      <button className={`source-chip ${count(provider) && count(provider) === selected.length ? "active" : ""}`} aria-expanded={open === provider} onClick={() => setOpen(open === provider ? null : provider)}><ProviderIcon provider={provider} />{provider === "BLUESKY" ? "Bluesky" : "Mastodon"}<span>{count(provider)}</span><ChevronDown size={13} /></button>
      {open === provider && <div className="source-menu">{accounts.filter(account => account.provider === provider).map(account => <button key={account.id} onClick={() => onToggle(account.id)}><span className="mini-avatar">{account.displayName[0]}</span><span><strong>{account.displayName}</strong><small>{account.handle}</small></span>{selected.includes(account.id) && <Check size={14} />}</button>)}<button onClick={() => { onSelectProvider(provider); setOpen(null); }}>Only {provider === "BLUESKY" ? "Bluesky" : "Mastodon"}</button></div>}
    </div>)}
  </div>;
}
