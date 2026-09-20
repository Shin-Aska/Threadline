import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { socialApi } from "../services/desktop/social";
import { mergeSocialPosts } from "../services/social/presentation";
import type { Account } from "../types";
import type { FeedPage, FollowedSource, ProfileFeedKind } from "../types/social";

export type SocialFeedSource =
  | { readonly kind: "HOME" }
  | { readonly kind: "OWN"; readonly feedKind: ProfileFeedKind }
  | { readonly kind: "PROFILE"; readonly profileId: string; readonly feedKind: ProfileFeedKind }
  | { readonly kind: "TAG"; readonly tag: string }
  | { readonly kind: "SOURCE"; readonly source: FollowedSource };

export interface SocialFeedFailure {
  readonly accountId: string;
  readonly message: string;
}
type FeedLoadResult = { readonly kind: "PAGE"; readonly accountId: string; readonly page: FeedPage } | { readonly kind: "ERROR"; readonly accountId: string; readonly error: string };

const message = (cause: unknown): string => cause instanceof Error ? cause.message : String(cause);

function load(accountId: string, source: SocialFeedSource, cursor: string | null, refresh = false): Promise<FeedPage> {
  switch (source.kind) {
    case "HOME": return socialApi.home(accountId, cursor, { refresh });
    case "OWN": return socialApi.own(accountId, source.feedKind, cursor, { refresh });
    case "PROFILE": return socialApi.profileFeed(accountId, source.profileId, source.feedKind, cursor, { refresh });
    case "TAG": return socialApi.tagFeed(accountId, source.tag, cursor, { refresh });
    case "SOURCE": return socialApi.sourceFeed(accountId, source.source, cursor, { refresh });
  }
}

/** Rejects superseded pages and permits one feed request at a time per mounted view. */
export function useSocialFeed(accounts: readonly Account[], source: SocialFeedSource) {
  const [pages, setPages] = useState<Readonly<Record<string, FeedPage>>>({});
  const [failures, setFailures] = useState<readonly SocialFeedFailure[]>([]);
  const [loading, setLoading] = useState(true);
  const revision = useRef(0);
  const inFlight = useRef(false);
  const accountsRef = useRef(accounts);
  const sourceRef = useRef(source);
  accountsRef.current = accounts;
  sourceRef.current = source;
  const sourceKey = JSON.stringify(source);
  const accountKey = accounts.map(account => account.id).join("\u001f");
  const refresh = useCallback(async (force = true) => {
    if (inFlight.current) return;
    const request = ++revision.current;
    inFlight.current = true;
    setLoading(true);
    const settled: readonly FeedLoadResult[] = await Promise.all(accountsRef.current.map(async account => {
      try { return { kind: "PAGE", accountId: account.id, page: await load(account.id, sourceRef.current, null, force) }; }
      catch (cause) { return { kind: "ERROR", accountId: account.id, error: message(cause) }; }
    }));
    if (request !== revision.current) return;
    const next: Record<string, FeedPage> = {};
    for (const result of settled) switch (result.kind) { case "PAGE": next[result.accountId] = result.page; break; case "ERROR": break; }
    setPages(next);
    setFailures(settled.flatMap(result => result.kind === "ERROR" ? [{ accountId: result.accountId, message: result.error }] : []));
    setLoading(false);
    inFlight.current = false;
  }, []);
  useEffect(() => { void accountKey; void sourceKey; void refresh(false); return () => { revision.current += 1; inFlight.current = false; }; }, [accountKey, refresh, sourceKey]);
  const more = useCallback(async () => {
    if (inFlight.current) return;
    const targets = accountsRef.current.filter(account => pages[account.id]?.cursor);
    if (targets.length === 0) return;
    const request = ++revision.current;
    inFlight.current = true;
    setLoading(true);
    const settled: readonly FeedLoadResult[] = await Promise.all(targets.map(async account => {
      try { return { kind: "PAGE", accountId: account.id, page: await load(account.id, sourceRef.current, pages[account.id]?.cursor ?? null) }; }
      catch (cause) { return { kind: "ERROR", accountId: account.id, error: message(cause) }; }
    }));
    if (request !== revision.current) return;
    setPages(current => {
      const next = { ...current };
      for (const result of settled) switch (result.kind) { case "PAGE": next[result.accountId] = { cursor: result.page.cursor, posts: [...(current[result.accountId]?.posts ?? []), ...result.page.posts] }; break; case "ERROR": break; }
      return next;
    });
    setFailures(settled.flatMap(result => result.kind === "ERROR" ? [{ accountId: result.accountId, message: result.error }] : []));
    setLoading(false);
    inFlight.current = false;
  }, [pages]);
  const items = useMemo(() => mergeSocialPosts(Object.entries(pages).map(([accountId, page]) => ({ accountId, posts: page.posts }))), [pages]);
  return { items, failures, loading, hasMore: Object.values(pages).some(page => page.cursor !== null), refresh, more };
}
