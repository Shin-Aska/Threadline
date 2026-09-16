import { useCallback, useEffect, useState } from "react";
import type { Account, ProviderFailure, UnifiedFeedPage, UnifiedPost } from "../types";
import { getUnifiedTimeline } from "../services/unified";
import { mergePosts } from "../services/unified/aggregate";

export function useUnifiedTimeline(accounts: readonly Account[]) {
  const [posts, setPosts] = useState<UnifiedPost[]>([]); const [pages, setPages] = useState<Record<string, UnifiedFeedPage>>({}); const [failures, setFailures] = useState<readonly ProviderFailure[]>([]); const [loading, setLoading] = useState(true); const [revision, setRevision] = useState(0);
  useEffect(() => { const controller = new AbortController(); setLoading(true); void getUnifiedTimeline(accounts, {}, controller.signal).then(result => { setPosts(result.posts); setPages(result.pages); setFailures(result.failures); }).finally(() => { if (!controller.signal.aborted) setLoading(false); }); return () => controller.abort(); }, [accounts, revision]);
  const more = useCallback(async () => { const cursors = Object.fromEntries(Object.entries(pages).filter(([, page]) => page.cursor).map(([id, page]) => [id, page.cursor])); if (!Object.keys(cursors).length) return; setLoading(true); const result = await getUnifiedTimeline(accounts.filter(account => account.id in cursors), cursors); const next = { ...pages, ...Object.fromEntries(Object.entries(result.pages).map(([id, page]) => [id, { ...page, posts: mergePosts([pages[id], page]) }])) }; setPages(next); setPosts(mergePosts(Object.values(next))); setFailures(result.failures); setLoading(false); }, [accounts, pages]);
  return { posts, failures, loading, hasMore: Object.values(pages).some(page => page.cursor), retry: () => setRevision(value => value + 1), more };
}
