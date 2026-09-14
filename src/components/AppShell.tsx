import { ArrowLeftRight, Check, Command, Feather, Plus, ShieldCheck } from "lucide-react";
import type { ReactNode } from "react";
import type { Account, WorkspaceMode } from "../types";
import { Identity } from "./ui";

export type Page = "composer" | "accounts";
interface ShellProps {
  readonly accounts: readonly Account[];
  readonly page: Page;
  readonly mode: WorkspaceMode;
  readonly onNavigate: (page: Page) => void;
  readonly onCompose: () => void;
  readonly children: ReactNode;
}

const modeLabels: Readonly<Record<WorkspaceMode, string>> = { BROWSER: "Browser preview", LIVE: "Desktop workspace", DISCONNECTED: "Reconnect required" };

export function AppShell({ accounts, page, mode, onNavigate, onCompose, children }: ShellProps) {
  return <div className="shell">
    <a className="skip-link" href="#main">Skip to content</a>
    <aside className="sidebar">
      <div className="brand"><span className="brand-mark"><img src={new URL("../assets/threadline-logo.svg", import.meta.url).href} width={32} height={32} alt="" /></span><strong>Threadline</strong><span className="status-dot" /></div>
      <div className="nodes"><div className="micro-heading"><span>Your identities</span><span>{accounts.length}</span></div>
        {accounts.map(account => <div key={account.id} className={`node ${account.provider.toLowerCase()}`}><span>{account.handle}</span><Check size={12} /></div>)}
        {!accounts.length && <p className="muted">Add an account to get started.</p>}
      </div>
      <nav aria-label="Main navigation">
        <button className={page === "composer" ? "nav-item active" : "nav-item"} aria-current={page === "composer" ? "page" : undefined} onClick={() => onNavigate("composer")} title="Composer"><Feather size={18} /><span>Composer</span><small>Cross</small></button>
        <button className={page === "accounts" ? "nav-item active" : "nav-item"} aria-current={page === "accounts" ? "page" : undefined} onClick={() => onNavigate("accounts")} title="Accounts & Sync"><ArrowLeftRight size={18} /><span>Accounts & Sync</span></button>
      </nav>
      <div className="sidebar-bottom"><div className="local-note"><ShieldCheck size={18} /><div><strong>Your accounts. Your control.</strong><p>Credentials are kept in your system’s secure credential store.</p></div></div>
        {accounts[0] && <div className="profile-strip"><Identity account={accounts[0]} /></div>}
        <span className="version">THREADLINE / 0.1.0</span>
      </div>
    </aside>
    <div className="app-content">
      <header className="topbar"><div className="connection-status"><span className="status-dot" /><span>{modeLabels[mode]}</span><span className="topbar-divider" /><span className="muted">Bluesky <span className="secondary-text">+ Mastodon</span></span></div><button className="button button-blue" onClick={onCompose}><Plus size={16} />Compose</button></header>
      <main id="main" tabIndex={-1}>{children}</main>
      <footer className="app-footer"><span><Command size={12} /> One idea. Every community.</span><span>{mode === "BROWSER" ? "Connect and publish in the desktop app" : "Powered by the native Rust core"}</span></footer>
    </div>
  </div>;
}
