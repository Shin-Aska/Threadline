export const POST_CACHE_LIMITS = [0, 50, 100, 250, 500] as const;
export type PostCacheLimit = (typeof POST_CACHE_LIMITS)[number];

const PREFERENCE_KEY = "threadline:post-cache-limit";
const DEFAULT_LIMIT: PostCacheLimit = 250;

export function readPostCacheLimit(): PostCacheLimit {
  if (typeof window === "undefined") return DEFAULT_LIMIT;
  try {
    const saved = localStorage.getItem(PREFERENCE_KEY);
    return POST_CACHE_LIMITS.find(limit => String(limit) === saved) ?? DEFAULT_LIMIT;
  } catch (cause) {
    if (cause instanceof DOMException) return DEFAULT_LIMIT;
    throw cause;
  }
}

export function savePostCacheLimit(limit: PostCacheLimit): void {
  localStorage.setItem(PREFERENCE_KEY, String(limit));
}
