import { ArrowLeftRight, Compass, Feather, Home, Plus, ShieldCheck, Users } from "lucide-react";
import type { ReactNode } from "react";
import type { Account, WorkspaceMode } from "../types";

export type Page = "composer" | "timeline" | "discover" | "following" | "accounts";
interface ShellProps {
  readonly accounts: readonly Account[];
  readonly connectedAccountIds: readonly string[];
  readonly page: Page;
  readonly mode: WorkspaceMode;
  readonly onNavigate: (page: Page) => void;
  readonly onCompose: () => void;
  readonly children: ReactNode;
}
export function AppShell({ accounts, connectedAccountIds, page, mode, onNavigate, onCompose, children }: ShellProps) {
  const connectedCount = accounts.filter(account => connectedAccountIds.includes(account.id)).length;
  const connectionSummary = `${connectedCount} connected ${connectedCount === 1 ? "account" : "accounts"}`;
  return <div className="shell">
    <a className="skip-link" href="#main">Skip to content</a>
    <header className="workspace-brandbar"><img src={new URL("../assets/threadline-logo.svg", import.meta.url).href} width={32} height={32} alt="" /><strong>Threadline</strong></header>
    <aside className="sidebar">
      <nav aria-label="Main navigation">
        <button className={page === "timeline" ? "nav-item active" : "nav-item"} aria-current={page === "timeline" ? "page" : undefined} onClick={() => onNavigate("timeline")} title="Timeline"><Home size={18} /><span>Timeline</span></button>
        <button className={page === "discover" ? "nav-item active" : "nav-item"} aria-current={page === "discover" ? "page" : undefined} onClick={() => onNavigate("discover")} title="Discover"><Compass size={18} /><span>Discover</span></button>
        <button className={page === "following" ? "nav-item active" : "nav-item"} aria-current={page === "following" ? "page" : undefined} onClick={() => onNavigate("following")} title="Following"><Users size={18} /><span>Following</span></button>
        <button className={page === "composer" ? "nav-item active" : "nav-item"} aria-current={page === "composer" ? "page" : undefined} onClick={() => onNavigate("composer")} title="Composer"><Feather size={18} /><span>Composer</span></button>
        <button className={page === "accounts" ? "nav-item active" : "nav-item"} aria-label="Accounts & Sync" aria-current={page === "accounts" ? "page" : undefined} aria-describedby="connected-account-count" onClick={() => onNavigate("accounts")} title={"Accounts & Sync · " + connectionSummary}><ArrowLeftRight size={18} /><span>Accounts & Sync</span><small className="connected-count" aria-hidden="true">{connectedCount}</small></button>
      </nav>
      <p className="sr-only" id="connected-account-count" role="status">{connectionSummary}</p>
      <div className="sidebar-bottom"><div className="local-note"><ShieldCheck size={18} /><div><strong>Your accounts. Your control.</strong><p>Credentials are kept in your system’s secure credential store.</p></div></div><span className="version">THREADLINE / 0.1.0</span></div>
    </aside>
    <div className="app-content">
      {page === "accounts" && <header className="topbar"><span className="muted">{mode === "BROWSER" ? "Browser preview" : "Your connected identities"}</span><button className="button button-blue" onClick={onCompose}><Plus size={16} />Compose</button></header>}
      <main id="main" tabIndex={-1}>{children}</main>
    </div>
  </div>;
}
