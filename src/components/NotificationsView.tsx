import { Bell, Heart, MessageCircle, Repeat2, Users } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { socialApi } from "../services/desktop/social";
import type { WorkspaceState } from "../types";
import type { NotificationItem, NotificationPage } from "../types/social";
import { Notice, ProviderIcon } from "./ui";
import { ViewHeader } from "./unified/ViewHeader";

type NotificationFilter = "ALL" | "MENTIONS" | "INTERACTIONS";
interface NotificationProps {
  readonly workspace: WorkspaceState;
  readonly active: boolean;
  readonly onPost: (accountId: string, postId: string) => void;
  readonly onProfile: (accountId: string, profileId: string) => void;
}

const POLL_INTERVAL_MS = 30_000;
const MAX_POLL_INTERVAL_MS = 5 * 60_000;

const icon = (kind: NotificationItem["kind"]) => {
  switch (kind) {
    case "MENTION": case "REPLY": case "QUOTE": return <MessageCircle />;
    case "LIKE": return <Heart />;
    case "REPOST": return <Repeat2 />;
    case "FOLLOW": return <Users />;
    case "OTHER": return <Bell />;
  }
};
const mergeNotifications = (existing: readonly NotificationItem[], incoming: readonly NotificationItem[]): readonly NotificationItem[] => {
  const merged = new Map(existing.map(item => [item.id, item]));
  for (const item of incoming) {
    const prior = merged.get(item.id);
    merged.set(item.id, prior?.unread === false ? { ...item, unread: false } : item);
  }
  return [...merged.values()];
};

export function NotificationsView({ workspace, active, onPost, onProfile }: NotificationProps) {
  const [pages, setPages] = useState<Readonly<Record<string, NotificationPage>>>({});
  const [filter, setFilter] = useState<NotificationFilter>("ALL");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const inFlight = useRef(false);
  const pendingRefresh = useRef<{ readonly quiet: boolean; readonly force: boolean } | null>(null);
  const activeRef = useRef(active);
  const visibleRef = useRef(document.visibilityState === "visible");
  const loadedRef = useRef(false);
  const lastActiveRef = useRef(false);
  const failureCount = useRef(0);
  const timer = useRef<number | null>(null);
  const refreshRef = useRef<(quiet?: boolean, force?: boolean) => Promise<void>>(async () => undefined);
  const clearTimer = useCallback(() => {
    if (timer.current !== null) window.clearTimeout(timer.current);
    timer.current = null;
  }, []);
  // A retained hidden page must not keep a notification poll timer alive.
  const schedule = useCallback(() => {
    clearTimer();
    if (!activeRef.current || !visibleRef.current) return;
    const delay = Math.min(POLL_INTERVAL_MS * 2 ** failureCount.current, MAX_POLL_INTERVAL_MS);
    timer.current = window.setTimeout(() => void refreshRef.current(true, true), delay);
  }, [clearTimer]);
  const refresh = useCallback(async (quiet = false, force = false) => {
    if (inFlight.current) { pendingRefresh.current = { quiet, force }; return; }
    inFlight.current = true;
    clearTimer();
    if (!quiet) setLoading(true);
    setError(null);
    const results = await Promise.all(workspace.accounts.map(async account => {
      try { return { accountId: account.id, page: await socialApi.notifications(account.id, null, { refresh: force }), error: null }; }
      catch (cause) { return { accountId: account.id, page: null, error: cause instanceof Error ? cause.message : String(cause) }; }
    }));
    setPages(currentPages => Object.fromEntries(results.map(result => { const previous = currentPages[result.accountId]; return [result.accountId, result.page ? { cursor: previous ? previous.cursor : result.page.cursor, notifications: mergeNotifications(previous?.notifications ?? [], result.page.notifications) } : previous ?? { notifications: [], cursor: null }]; })));
    const failures = results.filter(result => result.error).map(result => `${workspace.accounts.find(account => account.id === result.accountId)?.displayName ?? result.accountId}: ${result.error}`);
    failureCount.current = failures.length > 0 ? failureCount.current + 1 : 0;
    setError(failures.length ? failures.join(" · ") : null);
    setLoading(false);
    inFlight.current = false;
    const pending = pendingRefresh.current;
    pendingRefresh.current = null;
    if (pending && activeRef.current && visibleRef.current) await refreshRef.current(pending.quiet, pending.force);
    else schedule();
  }, [clearTimer, schedule, workspace.accounts]);
  refreshRef.current = refresh;
  useEffect(() => {
    activeRef.current = active;
    if (!active) { lastActiveRef.current = false; clearTimer(); return; }
    const resumed = !lastActiveRef.current;
    lastActiveRef.current = true;
    if (visibleRef.current && resumed) {
      const force = loadedRef.current;
      loadedRef.current = true;
      void refreshRef.current(force, force);
    }
    return clearTimer;
  }, [active, clearTimer, workspace.accounts]);
  useEffect(() => {
    const visibilityChanged = () => {
      const visible = document.visibilityState === "visible";
      if (visible === visibleRef.current) return;
      visibleRef.current = visible;
      if (!visible) { clearTimer(); return; }
      if (!activeRef.current) return;
      const force = loadedRef.current;
      loadedRef.current = true;
      void refreshRef.current(force, force);
    };
    document.addEventListener("visibilitychange", visibilityChanged);
    return () => { document.removeEventListener("visibilitychange", visibilityChanged); clearTimer(); };
  }, [clearTimer]);
  const notifications = useMemo(() => Object.entries(pages).flatMap(([accountId, page]) => page.notifications.map(item => ({ accountId, item }))).sort((left, right) => Date.parse(right.item.createdAt) - Date.parse(left.item.createdAt)), [pages]);
  const visible = notifications.filter(({ item }) => filter === "ALL" || filter === "MENTIONS" && ["MENTION", "REPLY", "QUOTE"].includes(item.kind) || filter === "INTERACTIONS" && ["LIKE", "REPOST", "FOLLOW"].includes(item.kind));
  const markRead = async () => {
    setError(null);
    try {
      await Promise.all(Object.entries(pages).map(([accountId, page]) => socialApi.markNotificationsRead(accountId, page.notifications.filter(item => item.unread).map(item => item.id))));
      setPages(current => Object.fromEntries(Object.entries(current).map(([accountId, page]) => [accountId, { ...page, notifications: page.notifications.map(item => ({ ...item, unread: false })) }])));
    } catch (cause) { setError(cause instanceof Error ? cause.message : String(cause)); }
  };
  const more = async () => {
    if (inFlight.current) return;
    const targets = workspace.accounts.filter(account => pages[account.id]?.cursor);
    if (targets.length === 0) return;
    inFlight.current = true;
    clearTimer();
    setLoading(true); setError(null);
    const results = await Promise.all(targets.map(async account => {
      try { return { accountId: account.id, page: await socialApi.notifications(account.id, pages[account.id]?.cursor ?? null), error: null }; }
      catch (cause) { return { accountId: account.id, page: null, error: cause instanceof Error ? cause.message : String(cause) }; }
    }));
    setPages(current => {
      const next = { ...current };
      for (const result of results) if (result.page) next[result.accountId] = { cursor: result.page.cursor, notifications: mergeNotifications(current[result.accountId]?.notifications ?? [], result.page.notifications) };
      return next;
    });
    const failures = results.filter(result => result.error).map(result => `${workspace.accounts.find(account => account.id === result.accountId)?.displayName ?? result.accountId}: ${result.error}`);
    setError(failures.length ? failures.join(" · ") : null);
    setLoading(false);
    inFlight.current = false;
    schedule();
  };
  return <div className="unified-page"><ViewHeader icon={<Bell />} title="Notifications" subtitle="Activity for your connected accounts." controls={<button className="button" disabled={!notifications.some(item => item.item.unread)} onClick={() => void markRead()}>Mark all read</button>} /><div className="unified-layout"><section className="panel notification-panel"><div className="tabs">{(["ALL", "MENTIONS", "INTERACTIONS"] as const).map(value => <button className={`button ${filter === value ? "selected" : ""}`} key={value} onClick={() => setFilter(value)}>{value === "ALL" ? "All" : value === "MENTIONS" ? "Mentions" : "Interactions"}</button>)}</div>{error && <Notice error>{error}</Notice>}{loading && notifications.length === 0 && <div className="post-skeletons" aria-label="Loading notifications"><i /><i /></div>}{visible.map(({ accountId, item }) => { const account = workspace.accounts.find(account => account.id === accountId); return <button className={`notification-row click-row ${item.unread ? "unread" : ""}`} key={`${accountId}:${item.id}`} onClick={() => item.post ? onPost(accountId, item.post.remoteId) : onProfile(accountId, item.actor.id)}>{icon(item.kind)}<div><strong>{item.actor.displayName} · {item.kind.toLocaleLowerCase()}</strong>{item.post && <p>{item.post.text}</p>}<small className="muted">Via {account?.displayName ?? accountId} · {account && <><ProviderIcon provider={account.provider} /> {account.provider === "BLUESKY" ? "Bluesky" : "Mastodon"} · </>}<time dateTime={item.createdAt}>{new Date(item.createdAt).toLocaleString()}</time></small></div>{item.unread && <span className="unread-dot" aria-label="Unread" />}</button>; })}{!loading && visible.length === 0 && <div className="empty-state"><Bell /><h3>No notifications in this view</h3><p>New provider activity will appear here.</p></div>}{Object.values(pages).some(page => page.cursor) && <button className="button load-more" disabled={loading} onClick={() => void more()}>{loading ? "Loading…" : "Load more"}</button>}</section><aside className="details-column"><section className="panel"><h2>Notification scope</h2><p className="aside-copy section-space">Activity is grouped from all connected accounts. Opening an item keeps the receiving account as the action identity.</p>{workspace.accounts.map(account => <div className="actor-row" key={account.id}><ProviderIcon provider={account.provider} /><span><strong>{account.displayName}</strong><small>@{account.handle}</small></span></div>)}</section></aside></div></div>;
}
