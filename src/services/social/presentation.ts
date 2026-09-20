import type { SocialPost, ViewerState } from "../../types/social";

export type SocialOrder = "LATEST" | "OLDEST" | "LIKES" | "DISCUSSED";

export interface SocialObservation {
  readonly accountId: string;
  readonly viewer: ViewerState;
}

export interface SocialFeedItem {
  readonly post: SocialPost;
  readonly observations: readonly SocialObservation[];
}

export interface SocialPresentation {
  readonly query: string;
  readonly mediaOnly: boolean;
  readonly hideReposts: boolean;
  readonly order: SocialOrder;
}

export function mergeSocialPosts(feeds: readonly { readonly accountId: string; readonly posts: readonly SocialPost[] }[]): SocialFeedItem[] {
  const merged = new Map<string, SocialFeedItem>();
  for (const feed of feeds) for (const post of feed.posts) {
    const observation = { accountId: feed.accountId, viewer: post.viewer };
    const current = merged.get(post.canonicalKey);
    merged.set(post.canonicalKey, current
      ? { ...current, observations: [...current.observations.filter(item => item.accountId !== feed.accountId), observation] }
      : { post, observations: [observation] });
  }
  return [...merged.values()];
}

export function presentSocialPosts(items: readonly SocialFeedItem[], options: SocialPresentation): SocialFeedItem[] {
  const query = options.query.trim().toLocaleLowerCase();
  const visible = items.filter(({ post }) => {
    const matchesQuery = query.length === 0 || `${post.author.displayName} ${post.author.handle} ${post.text}`.toLocaleLowerCase().includes(query);
    return matchesQuery && (!options.mediaOnly || post.media.length > 0) && (!options.hideReposts || post.replyParentId === null);
  });
  return visible.sort((left, right) => {
    switch (options.order) {
      case "LATEST": return Date.parse(right.post.createdAt) - Date.parse(left.post.createdAt);
      case "OLDEST": return Date.parse(left.post.createdAt) - Date.parse(right.post.createdAt);
      case "LIKES": return (right.post.metrics.likes ?? -1) - (left.post.metrics.likes ?? -1);
      case "DISCUSSED": return (right.post.metrics.replies ?? -1) - (left.post.metrics.replies ?? -1);
    }
  });
}
