import type { FollowingCollection, Provider, SourceAttribution, UnifiedFeedPage, UnifiedPost, UnifiedTopic } from "../../types";

const uniqueSources = (sources: readonly SourceAttribution[]) => [...new Map(sources.map(source => [source.accountId, source])).values()];

export function mergePosts(pages: readonly UnifiedFeedPage[]): UnifiedPost[] {
  const merged = new Map<string, UnifiedPost>();
  for (const post of pages.flatMap(page => page.posts)) {
    const current = merged.get(post.canonicalKey);
    merged.set(post.canonicalKey, current ? { ...current, sources: uniqueSources([...current.sources, ...post.sources]) } : { ...post, sources: uniqueSources(post.sources) });
  }
  return [...merged.values()].sort((a, b) => Date.parse(b.createdAt) - Date.parse(a.createdAt));
}

export function filterPosts(posts: readonly UnifiedPost[], accountIds: readonly string[], provider: Provider | "ALL" = "ALL") {
  const allowed = new Set(accountIds);
  return posts.filter(post => post.provider === provider || provider === "ALL").map(post => ({ ...post, sources: post.sources.filter(source => allowed.has(source.accountId)) })).filter(post => post.sources.length);
}

export function mergeTopics(topics: readonly UnifiedTopic[]): UnifiedTopic[] {
  const merged = new Map<string, UnifiedTopic>();
  for (const topic of topics) {
    const key = topic.name.replace(/^#/, "").trim().toLocaleLowerCase();
    const current = merged.get(key);
    if (!current) { merged.set(key, { ...topic, key }); continue; }
    const postCount = current.postCount === undefined ? topic.postCount : topic.postCount === undefined ? current.postCount : current.postCount + topic.postCount;
    const participantCount = current.participantCount === undefined ? topic.participantCount : topic.participantCount === undefined ? current.participantCount : current.participantCount + topic.participantCount;
    merged.set(key, { ...current, sources: uniqueSources([...current.sources, ...topic.sources]), ...(postCount === undefined ? {} : { postCount }), ...(participantCount === undefined ? {} : { participantCount }) });
  }
  return [...merged.values()].sort((a, b) => (b.postCount ?? -1) - (a.postCount ?? -1));
}

export function collectionPosts(collection: FollowingCollection, posts: readonly UnifiedPost[]) {
  const accountIds = new Set(collection.sources.map(source => source.accountId));
  return posts.map(post => ({ ...post, sources: post.sources.filter(source => accountIds.has(source.accountId)) })).filter(post => post.sources.length);
}
