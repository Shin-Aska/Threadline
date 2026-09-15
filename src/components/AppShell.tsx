import { ArrowLeftRight, Feather, Plus, ShieldCheck } from "lucide-react";
import { useState } from "react";
import type { ReactNode } from "react";
import type { Account, WorkspaceMode } from "../types";
import { useComposerStore } from "../stores/composer";
import { ProviderIcon } from "./ui";
import { providerNames } from "./providers";

export type Page = "composer" | "accounts";
interface ShellProps {
  readonly accounts: readonly Account[];
  readonly connectedAccountIds: readonly string[];
  readonly refreshing: boolean;
  readonly page: Page;
  readonly mode: WorkspaceMode;
  readonly onNavigate: (page: Page) => void;
  readonly onCompose: () => void;
  readonly children: ReactNode;
}
export function AppShell({ accounts, connectedAccountIds, refreshing, page, mode, onNavigate, onCompose, children }: ShellProps) {
  const { selected, toggle, publishing } = useComposerStore();
  const [showAccounts, setShowAccounts] = useState(false);
  return <div className="shell">
    <a className="skip-link" href="#main">Skip to content</a>
    <aside className="sidebar">
      <div className="brand"><span className="brand-mark"><img src={new URL("../assets/threadline-logo.svg", import.meta.url).href} width={32} height={32} alt="" /></span><strong>Threadline</strong><span className="status-dot" /></div>
      <button className="mobile-identities button" aria-expanded={showAccounts} aria-controls="identity-list" onClick={() => setShowAccounts(value => !value)}>Accounts <span>{selected.length}/{accounts.length}</span></button>
      <div id="identity-list" className={"nodes " + (showAccounts ? "mobile-open" : "")}>
        <div className="micro-heading"><span>Your identities</span><span>{accounts.length}</span></div>
        <div className="identity-list">{accounts.map(account => <label key={account.id} className={"identity-choice " + account.provider.toLowerCase() + (selected.includes(account.id) ? " selected" : "")} title={account.displayName + " · " + account.handle}>
          <span className="avatar" aria-hidden="true">{[...account.displayName][0] ?? "?"}</span><ProviderIcon provider={account.provider} />
          <span className="identity-copy"><strong>{account.displayName}</strong><span>{account.handle}</span>{!connectedAccountIds.includes(account.id) && <small>Reconnect required</small>}</span>
          <input type="checkbox" aria-label={account.displayName + " · " + providerNames[account.provider] + " · " + account.handle} checked={selected.includes(account.id)} disabled={publishing || refreshing} onChange={() => toggle(account.id)} />
        </label>)}</div>
        <button className="button button-link identity-add" onClick={() => onNavigate("accounts")}><Plus size={13} />Link an account</button>
      </div>
      <nav aria-label="Main navigation">
        <button className={page === "composer" ? "nav-item active" : "nav-item"} aria-current={page === "composer" ? "page" : undefined} onClick={() => onNavigate("composer")} title="Composer"><Feather size={18} /><span>Composer</span></button>
        <button className={page === "accounts" ? "nav-item active" : "nav-item"} aria-current={page === "accounts" ? "page" : undefined} onClick={() => onNavigate("accounts")} title="Accounts & Sync"><ArrowLeftRight size={18} /><span>Accounts & Sync</span></button>
      </nav>
      <div className="sidebar-bottom"><div className="local-note"><ShieldCheck size={18} /><div><strong>Your accounts. Your control.</strong><p>Credentials are kept in your system’s secure credential store.</p></div></div><span className="version">THREADLINE / 0.1.0</span></div>
    </aside>
    <div className="app-content">
      {page === "accounts" && <header className="topbar"><span className="muted">{mode === "BROWSER" ? "Browser preview" : "Your connected identities"}</span><button className="button button-blue" onClick={onCompose}><Plus size={16} />Compose</button></header>}
      <main id="main" tabIndex={-1}>{children}</main>
    </div>
  </div>;
}
