import type { Provider } from "./index";

/** Selects which posts to show in an account profile feed. */
export type ProfileFeedKind = "POSTS" | "REPLIES" | "MEDIA";
/** Provider source categories available in the following list. */
export type FollowedSourceKind = "PERSON" | "TAG" | "LIST" | "FEED";
/** Normalized social event categories used by notification views. */
export type NotificationKind = "MENTION" | "REPLY" | "LIKE" | "REPOST" | "FOLLOW" | "QUOTE" | "OTHER";

/** Author identity normalized from a provider response. */
export interface ProviderActor {
  readonly id: string;
  readonly displayName: string;
  readonly handle: string;
  readonly avatarUrl: string | null;
}

/** The signed-in viewer's reaction state and provider identifiers for undoing it. */
export interface ViewerState {
  readonly liked: boolean;
  readonly reposted: boolean;
  readonly likeUri: string | null;
  readonly repostUri: string | null;
}

/** Provider post normalized for timeline, detail, and interaction surfaces. */
export interface SocialPost {
  readonly canonicalKey: string;
  readonly provider: Provider;
  readonly remoteId: string;
  readonly remoteCid: string | null;
  readonly remoteUrl: string;
  readonly author: ProviderActor;
  readonly text: string;
  readonly contentWarning: string | null;
  readonly sensitive: boolean;
  readonly createdAt: string;
  readonly media: readonly { readonly url: string; readonly alt: string; readonly mediaType: string; readonly thumbnail?: string | null }[];
  readonly metrics: { readonly replies: number | null; readonly reposts: number | null; readonly likes: number | null };
  readonly viewer: ViewerState;
  readonly replyParentId: string | null;
  readonly replyRootId: string | null;
  readonly replyRootCid: string | null;
}

/** A page of posts returned by one account, with its provider pagination cursor. */
export interface FeedPage {
  readonly posts: readonly SocialPost[];
  readonly cursor: string | null;
}

/** Profile summary and whether the connected viewer follows that profile. */
export interface ProfileDetails {
  readonly actor: ProviderActor;
  readonly description: string;
  readonly followersCount: number | null;
  readonly followingCount: number | null;
  readonly postsCount: number | null;
  readonly followedByMe: boolean;
  readonly followUri: string | null;
}

/** A post together with its loaded ancestors, replies, and reply cursor. */
export interface ThreadView {
  readonly ancestors: readonly SocialPost[];
  readonly post: SocialPost;
  readonly replies: readonly SocialPost[];
  readonly cursor: string | null;
}

/** A followed person, tag, list, or feed in provider-neutral form. */
export interface FollowedSource {
  readonly id: string;
  readonly provider: Provider;
  readonly sourceType: FollowedSourceKind;
  readonly title: string;
  readonly description: string | null;
  readonly remoteId: string;
}

/** A page of followed sources returned by one connected account. */
export interface SourcePage {
  readonly sources: readonly FollowedSource[];
  readonly cursor: string | null;
}

/** Notification event with actor and optional associated post. */
export interface NotificationItem {
  readonly id: string;
  readonly kind: NotificationKind;
  readonly createdAt: string;
  readonly actor: ProviderActor;
  readonly post: SocialPost | null;
  readonly unread: boolean;
}

/** A page of notifications with the cursor for the next provider request. */
export interface NotificationPage {
  readonly notifications: readonly NotificationItem[];
  readonly cursor: string | null;
}

/** User action to apply to a post or profile on a specific provider. */
export type SocialAction =
  | { readonly kind: "LIKE"; readonly postId: string }
  | { readonly kind: "UNLIKE"; readonly postId: string }
  | { readonly kind: "REPOST"; readonly postId: string }
  | { readonly kind: "UNDO_REPOST"; readonly postId: string }
  | { readonly kind: "FOLLOW"; readonly profileId: string }
  | { readonly kind: "UNFOLLOW"; readonly profileId: string }
  | { readonly kind: "REPLY"; readonly postId: string; readonly text: string };

/** Provider response to a social action, including updated viewer state or content. */
export interface SocialActionResult {
  readonly targetId: string;
  readonly viewer: ViewerState | null;
  readonly followed: boolean | null;
  readonly recordId: string | null;
  readonly createdPost: SocialPost | null;
}
