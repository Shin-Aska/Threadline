import { invoke } from "@tauri-apps/api/core";
import type {
  FeedPage,
  FollowedSource,
  NotificationPage,
  ProfileDetails,
  ProfileFeedKind,
  SocialAction,
  SocialActionResult,
  SourcePage,
  ThreadView,
} from "../../types/social";
import { readPostCacheLimit, savePostCacheLimit, type PostCacheLimit } from "./post-cache-settings";
import { invalidateAroundMutation, SocialReadCache } from "./social-cache";

/** Options shared by cached reads, including an explicit bypass of cached data. */
export interface SocialReadOptions {
  readonly refresh?: boolean | undefined;
}

interface CachedCall {
  readonly accountId: string;
  readonly command: string;
  readonly args: Readonly<Record<string, unknown>>;
  readonly ttlMs: number;
  readonly options: SocialReadOptions | undefined;
}

const CONTENT_TTL_MS = 30_000;
const NOTIFICATION_TTL_MS = 20_000;
const REFERENCE_TTL_MS = 120_000;
let postCacheLimit = readPostCacheLimit();
const cache = new SocialReadCache({ maxEntries: 96, maxPosts: postCacheLimit });
const feedPostCount = (page: FeedPage): number => page.posts.length;
const threadPostCount = (thread: ThreadView): number => thread.ancestors.length + 1 + thread.replies.length;
const notificationPostCount = (page: NotificationPage): number => page.notifications.filter(item => item.post !== null).length;
type SocialActionListener = (accountId: string, action: SocialAction, result: SocialActionResult) => void;
const actionListeners = new Set<SocialActionListener>();

/** Subscribes to completed social actions and returns an unsubscribe function. */
export function subscribeSocialActions(listener: SocialActionListener): () => void {
  actionListeners.add(listener);
  return () => { actionListeners.delete(listener); };
}

const call = <T>(command: string, args: Readonly<Record<string, unknown>>): Promise<T> =>
  invoke<T>(command, args);

const cachedCall = <T>(request: CachedCall, countPosts?: (value: T) => number): Promise<T> => cache.read<T>({
  accountId: request.accountId,
  command: request.command,
  args: request.args,
  ttlMs: request.ttlMs,
  refresh: request.options?.refresh,
  ...(countPosts ? { countPosts } : {}),
  load: () => call<T>(request.command, request.args),
});

const mutate = <T>(accountId: string, command: string, args: Readonly<Record<string, unknown>>): Promise<T> =>
  invalidateAroundMutation(cache, accountId, () => call<T>(command, args));

/** Invalidates all cached reads or only those owned by one account. */
export const invalidateSocialReadCache = (accountId?: string): void => {
  if (accountId === undefined) cache.clear();
  else cache.invalidateAccount(accountId);
};

/** Returns the persisted upper bound for cached post records. */
export const getPostCacheLimit = (): PostCacheLimit => postCacheLimit;

/** Persists a new post-cache bound and immediately evicts excess cached posts. */
export const setPostCacheLimit = (limit: PostCacheLimit): void => {
  savePostCacheLimit(limit);
  cache.setMaxPosts(limit);
  postCacheLimit = limit;
};

/** Typed desktop bridge for cached social reads and account-scoped mutations. */
export const socialApi = {
  home: (accountId: string, cursor: string | null = null, options?: SocialReadOptions): Promise<FeedPage> =>
    cachedCall<FeedPage>({ accountId, command: "get_home_feed", args: { accountId, cursor }, ttlMs: CONTENT_TTL_MS, options }, feedPostCount),
  own: (accountId: string, kind: ProfileFeedKind, cursor: string | null = null, options?: SocialReadOptions): Promise<FeedPage> =>
    cachedCall<FeedPage>({ accountId, command: "get_own_feed", args: { accountId, kind, cursor }, ttlMs: CONTENT_TTL_MS, options }, feedPostCount),
  ownProfile: (accountId: string, options?: SocialReadOptions): Promise<ProfileDetails> =>
    cachedCall<ProfileDetails>({ accountId, command: "get_own_profile", args: { accountId }, ttlMs: REFERENCE_TTL_MS, options }),
  profile: (accountId: string, profileId: string, options?: SocialReadOptions): Promise<ProfileDetails> =>
    cachedCall<ProfileDetails>({ accountId, command: "get_profile", args: { accountId, profileId }, ttlMs: REFERENCE_TTL_MS, options }),
  profileFeed: (accountId: string, profileId: string, kind: ProfileFeedKind, cursor: string | null = null, options?: SocialReadOptions): Promise<FeedPage> =>
    cachedCall<FeedPage>({ accountId, command: "get_profile_feed", args: { accountId, profileId, kind, cursor }, ttlMs: CONTENT_TTL_MS, options }, feedPostCount),
  thread: (accountId: string, postId: string, options?: SocialReadOptions): Promise<ThreadView> =>
    cachedCall<ThreadView>({ accountId, command: "get_thread", args: { accountId, postId }, ttlMs: CONTENT_TTL_MS, options }, threadPostCount),
  tagFeed: (accountId: string, tag: string, cursor: string | null = null, options?: SocialReadOptions): Promise<FeedPage> =>
    cachedCall<FeedPage>({ accountId, command: "get_tag_feed", args: { accountId, tag, cursor }, ttlMs: CONTENT_TTL_MS, options }, feedPostCount),
  following: (accountId: string, cursor: string | null = null, options?: SocialReadOptions): Promise<SourcePage> =>
    cachedCall<SourcePage>({ accountId, command: "get_followed_sources", args: { accountId, cursor }, ttlMs: REFERENCE_TTL_MS, options }),
  sourceFeed: (accountId: string, source: FollowedSource, cursor: string | null = null, options?: SocialReadOptions): Promise<FeedPage> =>
    cachedCall<FeedPage>({ accountId, command: "get_source_feed", args: { accountId, source, cursor }, ttlMs: CONTENT_TTL_MS, options }, feedPostCount),
  notifications: (accountId: string, cursor: string | null = null, options?: SocialReadOptions): Promise<NotificationPage> =>
    cachedCall<NotificationPage>({ accountId, command: "get_notifications", args: { accountId, cursor }, ttlMs: NOTIFICATION_TTL_MS, options }, notificationPostCount),
  markNotificationsRead: (accountId: string, notificationIds: readonly string[]): Promise<void> =>
    mutate(accountId, "mark_notifications_read", { accountId, notificationIds }),
  act: async (accountId: string, action: SocialAction): Promise<SocialActionResult> => {
    const result = await mutate<SocialActionResult>(accountId, "perform_social_action", { accountId, action });
    for (const listener of actionListeners) listener(accountId, action, result);
    return result;
  },
} as const;
