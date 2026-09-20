import { Hash, Layers3, Search, Users } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useBrowsingScope } from "../hooks/useBrowsingScope";
import { useSocialFeed } from "../hooks/useSocialFeed";
import { socialApi } from "../services/desktop/social";
import { mergeSocialPosts, presentSocialPosts, type SocialFeedItem, type SocialOrder } from "../services/social/presentation";
import type { WorkspaceState } from "../types";
import type { FeedPage, FollowedSource } from "../types/social";
import { Notice } from "./ui";
import { SocialFeedToolbar } from "./unified/SocialFeedToolbar";
import { SocialPostCard } from "./unified/SocialPostCard";
import { SourceControls } from "./unified/SourceControls";
import { ViewHeader } from "./unified/ViewHeader";

interface OwnedSource { readonly accountId: string; readonly source: FollowedSource }
type CollectionId = "ALL" | "PERSON" | "TAG" | "LIST_FEED";
const collections: readonly { readonly id: CollectionId; readonly title: string; readonly description: string }[] = [
  { id: "ALL", title: "All Following", description: "Your providers’ following streams, plus followed topics, lists, and custom feeds across selected accounts." },
  { id: "PERSON", title: "People", description: "Your provider’s following stream. Depending on the network, this may include your own posts, reposts, and followed tags; it is not an exhaustive union of every followed author." },
  { id: "TAG", title: "Topics & tags", description: "Followed topics and hashtags across selected accounts." },
  { id: "LIST_FEED", title: "Lists / feeds", description: "Provider lists and custom feeds across selected accounts." },
];
const belongs = (collection: CollectionId, source: FollowedSource): boolean => collection === "ALL" || collection === "LIST_FEED" && (source.sourceType === "LIST" || source.sourceType === "FEED") || source.sourceType === collection;
const needsHome = (collection: CollectionId): boolean => collection === "ALL" || collection === "PERSON";
const needsSourceFeed = (collection: CollectionId, source: FollowedSource): boolean => source.sourceType !== "PERSON" && belongs(collection, source);
const sourcePageKey = ({ accountId, source }: OwnedSource): string => JSON.stringify([accountId, source.id]);
async function mapLimited<Item, Result>(items: readonly Item[], worker: (item: Item) => Promise<Result>): Promise<Result[]> {
  // Source indexes can be large; bound fan-out without changing result order.
  const results: Result[] = new Array(items.length); let next = 0;
  const run = async () => { while (next < items.length) { const index = next++; results[index] = await worker(items[index]!); } };
  await Promise.all(Array.from({ length: Math.min(4, items.length) }, run)); return results;
}
function mergeFeedItems(groups: readonly (readonly SocialFeedItem[])[]): SocialFeedItem[] {
  const merged = new Map<string, SocialFeedItem>();
  for (const group of groups) for (const item of group) {
    const current = merged.get(item.post.canonicalKey);
    if (!current) { merged.set(item.post.canonicalKey, item); continue; }
    const observations = new Map(current.observations.map(observation => [observation.accountId, observation]));
    for (const observation of item.observations) observations.set(observation.accountId, observation);
    merged.set(item.post.canonicalKey, { ...current, observations: [...observations.values()] });
  }
  return [...merged.values()];
}

interface FollowingProps { readonly workspace: WorkspaceState; readonly onSource: (accountId: string, source: FollowedSource) => void; readonly onPost: (accountId: string, postId: string) => void; readonly onProfile: (accountId: string, profileId: string) => void; readonly onTag: (accountId: string, tag: string) => void }
export function FollowingView(props: FollowingProps) {
  const scope = useBrowsingScope("following", props.workspace.accounts);
  const accounts = useMemo(() => props.workspace.accounts.filter(account => scope.selected.includes(account.id)), [props.workspace.accounts, scope.selected]);
  const home = useSocialFeed(accounts, { kind: "HOME" });
  const [sources, setSources] = useState<readonly OwnedSource[]>([]);
  const [sourceCursors, setSourceCursors] = useState<Readonly<Record<string, string | null>>>({});
  const [collectionId, setCollectionId] = useState<CollectionId>("ALL");
  const [pages, setPages] = useState<Readonly<Record<string, FeedPage>>>({});
  const [sourceError, setSourceError] = useState<string | null>(null);
  const [feedError, setFeedError] = useState<string | null>(null);
  const [sourceLoading, setSourceLoading] = useState(true);
  const [feedLoading, setFeedLoading] = useState(true);
  const [query, setQuery] = useState(""); const [order, setOrder] = useState<SocialOrder>("LATEST"); const [mediaOnly, setMediaOnly] = useState(false); const [hideReposts, setHideReposts] = useState(false);
  const sourceRequest = useRef(0);
  const feedRequest = useRef(0);
  const pagesRef = useRef(pages); pagesRef.current = pages;
  const accountKey = accounts.map(account => account.id).join("\u001f");
  const loadSources = async (nextPage: boolean) => {
    if (sourceLoading && nextPage) return;
    const targets = nextPage ? accounts.filter(account => sourceCursors[account.id]) : accounts;
    const current = ++sourceRequest.current; setSourceLoading(true); setSourceError(null);
    const results = await Promise.all(targets.map(async account => { try { return { accountId: account.id, page: await socialApi.following(account.id, nextPage ? sourceCursors[account.id] : null), error: null }; } catch (cause) { return { accountId: account.id, page: null, error: cause instanceof Error ? cause.message : String(cause) }; } }));
    if (current !== sourceRequest.current) return;
    setSources(existing => { const additions = results.flatMap(result => result.page?.sources.map(source => ({ accountId: result.accountId, source })) ?? []); const combined = nextPage ? [...existing, ...additions] : additions; return [...new Map(combined.map(item => [sourcePageKey(item), item])).values()]; });
    setSourceCursors(existing => ({ ...(nextPage ? existing : {}), ...Object.fromEntries(results.flatMap(result => result.page ? [[result.accountId, result.page.cursor]] : [])) }));
    const failures = results.filter(result => result.error).map(result => `${props.workspace.accounts.find(account => account.id === result.accountId)?.displayName ?? result.accountId}: ${result.error}`);
    setSourceError(failures.length ? failures.join(" · ") : null); setSourceLoading(false);
  };
  const loadSourcesRef = useRef(loadSources); loadSourcesRef.current = loadSources;
  useEffect(() => { setSources([]); setPages({}); setSourceCursors({}); void loadSourcesRef.current(false); return () => { sourceRequest.current += 1; }; }, [accountKey]);
  const selected = collections.find(item => item.id === collectionId) ?? collections[0];
  const displayedSources = useMemo(() => sources.filter(item => belongs(collectionId, item.source)), [collectionId, sources]);
  const feedSources = useMemo(() => sources.filter(item => needsSourceFeed(collectionId, item.source)), [collectionId, sources]);
  const sourceKey = feedSources.map(sourcePageKey).join("\u001f");
  const loadFeeds = async (nextPage: boolean) => {
    if (feedLoading && nextPage) return;
    const targets = nextPage ? feedSources.filter(item => pagesRef.current[sourcePageKey(item)]?.cursor) : feedSources.filter(item => !pagesRef.current[sourcePageKey(item)]);
    if (targets.length === 0) { setFeedLoading(false); return; }
    const current = ++feedRequest.current; setFeedLoading(true); setFeedError(null);
    const results = await mapLimited(targets, async item => { const key = sourcePageKey(item); try { return { key, page: await socialApi.sourceFeed(item.accountId, item.source, nextPage ? pagesRef.current[key]?.cursor ?? null : null), error: null }; } catch (cause) { return { key, page: null, error: cause instanceof Error ? cause.message : String(cause) }; } });
    if (current !== feedRequest.current) return;
    setPages(existing => { const updated = { ...existing }; for (const result of results) if (result.page) updated[result.key] = nextPage ? { cursor: result.page.cursor, posts: [...(existing[result.key]?.posts ?? []), ...result.page.posts] } : result.page; return updated; });
    const failures = results.filter(result => result.error).map(result => result.error); setFeedError(failures.length ? failures.join(" · ") : null); setFeedLoading(false);
  };
  const loadFeedsRef = useRef(loadFeeds); loadFeedsRef.current = loadFeeds;
  useEffect(() => { void sourceKey; void loadFeedsRef.current(false); return () => { feedRequest.current += 1; }; }, [sourceKey]);
  const sourceItems = mergeSocialPosts(Object.entries(pages).flatMap(([key, page]) => { const owner = feedSources.find(item => sourcePageKey(item) === key); return owner ? [{ accountId: owner.accountId, posts: page.posts }] : []; }));
  const merged = mergeFeedItems([...(needsHome(collectionId) ? [home.items] : []), sourceItems]);
  const posts = presentSocialPosts(merged, { query, order, mediaOnly, hideReposts });
  const loading = (needsHome(collectionId) && home.loading) || feedLoading;
  const hasMore = needsHome(collectionId) && home.hasMore || feedSources.some(item => pages[sourcePageKey(item)]?.cursor);
  const loadMore = async () => { await Promise.all([needsHome(collectionId) && home.hasMore ? home.more() : Promise.resolve(), loadFeeds(true)]); };
  return <div className="unified-page"><ViewHeader icon={<Users />} title="Following" subtitle="People, topics, lists, and feeds from every network — in one place." controls={<SourceControls accounts={props.workspace.accounts} selected={scope.selected} onToggle={scope.toggle} onSelectProvider={scope.selectProvider} />} /><div className="following-layout"><aside className="collection-column panel"><h2>Collections</h2>{collections.map(collection => <button key={collection.id} className={collectionId === collection.id ? "selected" : ""} onClick={() => setCollectionId(collection.id)}>{collection.id === "PERSON" ? <Users /> : collection.id === "TAG" ? <Hash /> : <Layers3 />}<span><strong>{collection.title}</strong><small>{sources.filter(item => belongs(collection.id, item.source)).length} sources</small></span></button>)}{Object.values(sourceCursors).some(Boolean) && <button className="button load-more" disabled={sourceLoading} onClick={() => void loadSources(true)}>{sourceLoading ? "Loading…" : "Load more sources"}</button>}</aside><aside className="details-column"><section className="panel sticky-detail"><h2>Source details</h2><div className="detail-topic">{selected.title}</div><p className="aside-copy">{selected.description}</p><div className="source-detail-list">{displayedSources.map(item => { const account = props.workspace.accounts.find(account => account.id === item.accountId); return <button className="actor-row click-row" key={`${item.accountId}:${item.source.id}`} onClick={() => props.onSource(item.accountId, item.source)}><span className="mini-avatar">{item.source.sourceType === "TAG" ? "#" : item.source.title[0]}</span><span><strong>{item.source.title}</strong><small>{item.source.sourceType} · @{account?.handle}</small></span></button>; })}</div></section></aside><section className="feed-column"><div className="collection-heading panel"><Layers3 /><div><h2>{selected.title}</h2><p>{selected.description}</p></div></div><SocialFeedToolbar query={query} order={order} mediaOnly={mediaOnly} hideReposts={hideReposts} onQuery={setQuery} onOrder={setOrder} onMediaOnly={setMediaOnly} onHideReposts={setHideReposts} />{(sourceError || feedError || home.failures.length > 0) && <Notice error>{[sourceError, feedError, ...home.failures.map(failure => failure.message)].filter(Boolean).join(" · ")}</Notice>}{loading && posts.length === 0 ? <div className="post-skeletons"><i /><i /></div> : <div className="row-list">{posts.map(item => <SocialPostCard item={item} accounts={props.workspace.accounts} key={item.post.canonicalKey} onPost={props.onPost} onProfile={props.onProfile} onTag={props.onTag} />)}</div>}{!loading && posts.length === 0 && <div className="panel empty-state"><Search /><h3>No posts in this collection</h3><p>The selected provider streams returned no posts for the loaded page.</p></div>}{hasMore && <button className="button load-more" disabled={loading} onClick={() => void loadMore()}>{loading ? "Loading…" : "Load more posts"}</button>}</section></div></div>;
}
