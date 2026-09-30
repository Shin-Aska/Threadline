/** Supported limits for retaining normalized posts in the in-memory read cache. */
export const POST_CACHE_LIMITS = [0, 50, 100, 250, 500] as const;
/** A post-cache limit offered by the settings UI. */
export type PostCacheLimit = (typeof POST_CACHE_LIMITS)[number];

const PREFERENCE_KEY = "threadline:post-cache-limit";
const DEFAULT_LIMIT: PostCacheLimit = 250;

/** Reads the saved cache limit, falling back when storage is unavailable or invalid. */
export function readPostCacheLimit(): PostCacheLimit {
  if (typeof window === "undefined") return DEFAULT_LIMIT;
  try {
    const saved = localStorage.getItem(PREFERENCE_KEY);
    return POST_CACHE_LIMITS.find(limit => String(limit) === saved) ?? DEFAULT_LIMIT;
  } catch {
    // Storage can be disabled, blocked by policy, or replaced by an embedding
    // webview implementation that throws something other than DOMException.
    return DEFAULT_LIMIT;
  }
}

/** Saves the selected cache limit when browser storage permits persistence. */
export function savePostCacheLimit(limit: PostCacheLimit): void {
  try {
    localStorage.setItem(PREFERENCE_KEY, String(limit));
  } catch {
    // The in-memory setting still applies for this session when persistence is
    // unavailable. Provider responses are never written to browser storage.
  }
}
