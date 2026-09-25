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
import { invalidateAroundMutation, SocialReadCache } from "./social-cache";

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
const cache = new SocialReadCache({ maxEntries: 96 });
type SocialActionListener = (accountId: string, action: SocialAction, result: SocialActionResult) => void;
const actionListeners = new Set<SocialActionListener>();

export function subscribeSocialActions(listener: SocialActionListener): () => void {
  actionListeners.add(listener);
  return () => { actionListeners.delete(listener); };
}

const call = <T>(command: string, args: Readonly<Record<string, unknown>>): Promise<T> =>
  invoke<T>(command, args);

const cachedCall = <T>(request: CachedCall): Promise<T> => cache.read({
  accountId: request.accountId,
  command: request.command,
  args: request.args,
  ttlMs: request.ttlMs,
  refresh: request.options?.refresh,
  load: () => call<T>(request.command, request.args),
});

const mutate = <T>(accountId: string, command: string, args: Readonly<Record<string, unknown>>): Promise<T> =>
  invalidateAroundMutation(cache, accountId, () => call<T>(command, args));

export const invalidateSocialReadCache = (accountId?: string): void => {
  if (accountId === undefined) cache.clear();
  else cache.invalidateAccount(accountId);
};

export const socialApi = {
  home: (accountId: string, cursor: string | null = null, options?: SocialReadOptions): Promise<FeedPage> =>
    cachedCall({ accountId, command: "get_home_feed", args: { accountId, cursor }, ttlMs: CONTENT_TTL_MS, options }),
  own: (accountId: string, kind: ProfileFeedKind, cursor: string | null = null, options?: SocialReadOptions): Promise<FeedPage> =>
    cachedCall({ accountId, command: "get_own_feed", args: { accountId, kind, cursor }, ttlMs: CONTENT_TTL_MS, options }),
  ownProfile: (accountId: string, options?: SocialReadOptions): Promise<ProfileDetails> =>
    cachedCall({ accountId, command: "get_own_profile", args: { accountId }, ttlMs: REFERENCE_TTL_MS, options }),
  profile: (accountId: string, profileId: string, options?: SocialReadOptions): Promise<ProfileDetails> =>
    cachedCall({ accountId, command: "get_profile", args: { accountId, profileId }, ttlMs: REFERENCE_TTL_MS, options }),
  profileFeed: (accountId: string, profileId: string, kind: ProfileFeedKind, cursor: string | null = null, options?: SocialReadOptions): Promise<FeedPage> =>
    cachedCall({ accountId, command: "get_profile_feed", args: { accountId, profileId, kind, cursor }, ttlMs: CONTENT_TTL_MS, options }),
  thread: (accountId: string, postId: string, options?: SocialReadOptions): Promise<ThreadView> =>
    cachedCall({ accountId, command: "get_thread", args: { accountId, postId }, ttlMs: CONTENT_TTL_MS, options }),
  tagFeed: (accountId: string, tag: string, cursor: string | null = null, options?: SocialReadOptions): Promise<FeedPage> =>
    cachedCall({ accountId, command: "get_tag_feed", args: { accountId, tag, cursor }, ttlMs: CONTENT_TTL_MS, options }),
  following: (accountId: string, cursor: string | null = null, options?: SocialReadOptions): Promise<SourcePage> =>
    cachedCall({ accountId, command: "get_followed_sources", args: { accountId, cursor }, ttlMs: REFERENCE_TTL_MS, options }),
  sourceFeed: (accountId: string, source: FollowedSource, cursor: string | null = null, options?: SocialReadOptions): Promise<FeedPage> =>
    cachedCall({ accountId, command: "get_source_feed", args: { accountId, source, cursor }, ttlMs: CONTENT_TTL_MS, options }),
  notifications: (accountId: string, cursor: string | null = null, options?: SocialReadOptions): Promise<NotificationPage> =>
    cachedCall({ accountId, command: "get_notifications", args: { accountId, cursor }, ttlMs: NOTIFICATION_TTL_MS, options }),
  markNotificationsRead: (accountId: string, notificationIds: readonly string[]): Promise<void> =>
    mutate(accountId, "mark_notifications_read", { accountId, notificationIds }),
  act: async (accountId: string, action: SocialAction): Promise<SocialActionResult> => {
    const result = await mutate<SocialActionResult>(accountId, "perform_social_action", { accountId, action });
    for (const listener of actionListeners) listener(accountId, action, result);
    return result;
  },
} as const;
