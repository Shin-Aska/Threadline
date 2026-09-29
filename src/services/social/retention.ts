import type { FeedPage, NotificationPage } from "../../types/social";

/** Mounted views retain at most this many post-bearing rows, independently of the read cache. */
export const VIEW_POST_LIMIT = 500;

function remaining(total: number): number {
  return Math.max(0, VIEW_POST_LIMIT - total);
}

export function boundFeedPages(pages: Readonly<Record<string, FeedPage>>): Readonly<Record<string, FeedPage>> {
  let total = 0;
  let truncated = false;
  const bounded: Record<string, FeedPage> = {};
  for (const [key, page] of Object.entries(pages)) {
    const posts = page.posts.slice(0, remaining(total));
    total += posts.length;
    truncated ||= posts.length < page.posts.length || total >= VIEW_POST_LIMIT && page.cursor !== null;
    bounded[key] = { posts, cursor: truncated ? null : page.cursor };
  }
  return truncated ? Object.fromEntries(Object.entries(bounded).map(([key, page]) => [key, { ...page, cursor: null }])) : bounded;
}

export function boundNotificationPages(pages: Readonly<Record<string, NotificationPage>>): Readonly<Record<string, NotificationPage>> {
  let postCount = 0;
  let truncated = false;
  const bounded: Record<string, NotificationPage> = {};
  for (const [key, page] of Object.entries(pages)) {
    const notifications = page.notifications.filter(item => {
      if (item.post === null) return true;
      if (postCount >= VIEW_POST_LIMIT) { truncated = true; return false; }
      postCount += 1;
      return true;
    });
    truncated ||= notifications.length < page.notifications.length || postCount >= VIEW_POST_LIMIT && page.cursor !== null;
    bounded[key] = { notifications, cursor: truncated ? null : page.cursor };
  }
  return truncated ? Object.fromEntries(Object.entries(bounded).map(([key, page]) => [key, { ...page, cursor: null }])) : bounded;
}
