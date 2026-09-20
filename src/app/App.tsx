import { useEffect, useState } from "react";
import { AccountsPanel } from "../components/AccountsPanel";
import { AppShell, type Page } from "../components/AppShell";
import { DiscoverView } from "../components/DiscoverView";
import { FollowingView } from "../components/FollowingView";
import { MyProfilesView } from "../components/MyProfilesView";
import { NotificationsView } from "../components/NotificationsView";
import { PublishingWorkspace } from "../components/PublishingWorkspace";
import { SocialDetailView } from "../components/details/SocialDetailView";
import { TimelineView } from "../components/TimelineView";
import { Notice } from "../components/ui";
import { useWorkspace } from "../hooks/useWorkspace";
import { useWorkspaceNavigation } from "../hooks/useWorkspaceNavigation";
import { invalidateSocialReadCache } from "../services/desktop/social";

const titles: Readonly<Record<Page, string>> = { notifications: "Notifications", myprofiles: "My profiles", timeline: "Timeline", discover: "Discover", following: "Following", composer: "Composer", accounts: "Accounts & Sync" };

export function App() {
  const navigation = useWorkspaceNavigation();
  const [visited, setVisited] = useState<ReadonlySet<Page>>(() => new Set(["timeline"]));
  const { workspace, error, refreshing, refresh } = useWorkspace();
  const needsSetup = workspace?.accounts.length === 0;
  useEffect(() => { setVisited(current => new Set(current).add(navigation.page)); }, [navigation.page]);
  useEffect(() => { document.title = `${needsSetup || !workspace ? "Account setup" : navigation.detail ? navigation.detail.kind === "POST" ? "Conversation" : navigation.detail.kind === "TAG" ? `#${navigation.detail.id}` : navigation.detail.kind === "SOURCE" ? navigation.detail.source.title : "Profile" : titles[navigation.page]} · Threadline`; }, [navigation.page, navigation.detail, needsSetup, workspace]);
  const workspaceError = error && <div className="page workspace-notice"><Notice error><span>{error}</span><button className="button" disabled={refreshing} onClick={() => void refresh()}>{refreshing ? "Refreshing…" : "Retry workspace"}</button></Notice></div>;
  if (!workspace || needsSetup) return <main className="setup-shell"><header className="setup-brand"><img src={new URL("../assets/threadline-logo.svg", import.meta.url).href} width={32} height={32} alt="" /><strong>Threadline</strong></header>{workspaceError}{!workspace && !error && <Notice>Loading your workspace…</Notice>}{workspace && <AccountsPanel setup workspace={workspace} refreshing={refreshing || error !== null} onConnected={async account => { invalidateSocialReadCache(account.id); if (await refresh(account.id)) navigation.navigate("timeline"); }} onRemoved={async () => { invalidateSocialReadCache(); await refresh(); }} />}</main>;
  const openPost = (accountId: string, id: string) => navigation.openDetail({ kind: "POST", accountId, id, origin: titles[navigation.page] });
  const openProfile = (accountId: string, id: string) => navigation.openDetail({ kind: "PROFILE", accountId, id, origin: titles[navigation.page] });
  const openTag = (accountId: string, id: string) => navigation.openDetail({ kind: "TAG", accountId, id, origin: titles[navigation.page] });
  const compose = () => { navigation.navigate("composer"); requestAnimationFrame(() => document.getElementById("post-text")?.focus()); };
  const pageProps = { workspace, onPost: openPost, onProfile: openProfile, onTag: openTag };
  return <AppShell connectedAccountIds={workspace.connectedAccountIds} accounts={workspace.accounts} page={navigation.page} profileAccountId={navigation.profileAccountId} mode={workspace.mode} onNavigate={navigation.navigate} onProfileAccount={navigation.selectProfileAccount} onCompose={compose}>
    {workspaceError}
    {navigation.detail && <SocialDetailView target={navigation.detail} workspace={workspace} onBack={navigation.back} onPost={openPost} onProfile={openProfile} onTag={openTag} />}
    {visited.has("notifications") && <div hidden={navigation.detail !== null || navigation.page !== "notifications"}><NotificationsView workspace={workspace} active={navigation.detail === null && navigation.page === "notifications"} onPost={openPost} onProfile={openProfile} /></div>}
    {visited.has("myprofiles") && <div hidden={navigation.detail !== null || navigation.page !== "myprofiles"}><MyProfilesView {...pageProps} accountId={navigation.profileAccountId} onAccount={navigation.selectProfileAccount} onAccounts={() => navigation.navigate("accounts")} /></div>}
    {visited.has("timeline") && <div hidden={navigation.detail !== null || navigation.page !== "timeline"}><TimelineView {...pageProps} /></div>}
    {visited.has("discover") && <div hidden={navigation.detail !== null || navigation.page !== "discover"}><DiscoverView {...pageProps} /></div>}
    {visited.has("following") && <div hidden={navigation.detail !== null || navigation.page !== "following"}><FollowingView {...pageProps} onSource={(accountId, source) => navigation.openDetail({ kind: "SOURCE", accountId, source, origin: "Following" })} /></div>}
    {visited.has("composer") && <div hidden={navigation.detail !== null || navigation.page !== "composer"}><PublishingWorkspace workspace={workspace} refreshing={refreshing || error !== null} onAccounts={() => navigation.navigate("accounts")} /></div>}
    {visited.has("accounts") && <div hidden={navigation.detail !== null || navigation.page !== "accounts"}><AccountsPanel workspace={workspace} refreshing={refreshing || error !== null} onConnected={async account => { invalidateSocialReadCache(account.id); await refresh(account.id); }} onRemoved={async () => { invalidateSocialReadCache(); await refresh(); }} /></div>}
  </AppShell>;
}
