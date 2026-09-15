import { useEffect, useState } from "react";
import { AccountsPanel } from "../components/AccountsPanel";
import { AppShell } from "../components/AppShell";
import type { Page } from "../components/AppShell";
import { ComposerPanel } from "../components/ComposerPanel";
import { Notice } from "../components/ui";
import { useWorkspace } from "../hooks/useWorkspace";

export function App() {
  const [page, setPage] = useState<Page>("composer");
  const { workspace, error, refreshing, refresh } = useWorkspace();
  const needsSetup = workspace?.accounts.length === 0;
  useEffect(() => { document.title = `${needsSetup || !workspace ? "Account setup" : page === "composer" ? "Composer" : "Accounts & Sync"} · Threadline`; }, [page, needsSetup, workspace]);
  const workspaceError = error && <div className="page workspace-notice"><Notice error><span>{error}</span><button className="button" disabled={refreshing} onClick={() => void refresh()}>{refreshing ? "Refreshing…" : "Retry workspace"}</button></Notice></div>;
  if (!workspace || needsSetup) return <main className="setup-shell">
    <header className="setup-brand"><img src={new URL("../assets/threadline-logo.svg", import.meta.url).href} width={32} height={32} alt="" /><strong>Threadline</strong></header>
    {workspaceError}
    {!workspace && !error && <Notice>Loading your workspace…</Notice>}
    {workspace && <AccountsPanel setup workspace={workspace} refreshing={refreshing || error !== null} onConnected={async account => { if (await refresh(account.id)) setPage("composer"); }} onRemoved={async () => { await refresh(); }} />}
  </main>;
  const compose = () => {
    setPage("composer");
    requestAnimationFrame(() => document.getElementById("post-text")?.focus());
  };
  return <AppShell connectedAccountIds={workspace.connectedAccountIds} refreshing={refreshing || error !== null} accounts={workspace.accounts} page={page} mode={workspace.mode} onNavigate={setPage} onCompose={compose}>
    {workspaceError}
    <div hidden={page !== "composer"}><ComposerPanel workspace={workspace} refreshing={refreshing || error !== null} onAccounts={() => setPage("accounts")} /></div>
    {page === "accounts" && <AccountsPanel workspace={workspace} refreshing={refreshing || error !== null} onConnected={async account => { await refresh(account.id); }} onRemoved={async () => { await refresh(); }} />}
  </AppShell>;
}
