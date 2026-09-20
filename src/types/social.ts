import type { Provider } from "./index";

export type ProfileFeedKind = "POSTS" | "REPLIES" | "MEDIA";
export type FollowedSourceKind = "PERSON" | "TAG" | "LIST" | "FEED";
export type NotificationKind = "MENTION" | "REPLY" | "LIKE" | "REPOST" | "FOLLOW" | "QUOTE" | "OTHER";

export interface ProviderActor {
  readonly id: string;
  readonly displayName: string;
  readonly handle: string;
  readonly avatarUrl: string | null;
}

export interface ViewerState {
  readonly liked: boolean;
  readonly reposted: boolean;
  readonly likeUri: string | null;
  readonly repostUri: string | null;
}

export interface SocialPost {
  readonly canonicalKey: string;
  readonly provider: Provider;
  readonly remoteId: string;
  readonly remoteCid: string | null;
  readonly remoteUrl: string;
  readonly author: ProviderActor;
  readonly text: string;
  readonly createdAt: string;
  readonly media: readonly { readonly url: string; readonly alt: string; readonly mediaType: string }[];
  readonly metrics: { readonly replies: number | null; readonly reposts: number | null; readonly likes: number | null };
  readonly viewer: ViewerState;
  readonly replyParentId: string | null;
  readonly replyRootId: string | null;
  readonly replyRootCid: string | null;
}

export interface FeedPage {
  readonly posts: readonly SocialPost[];
  readonly cursor: string | null;
}

export interface ProfileDetails {
  readonly actor: ProviderActor;
  readonly description: string;
  readonly followersCount: number | null;
  readonly followingCount: number | null;
  readonly postsCount: number | null;
  readonly followedByMe: boolean;
  readonly followUri: string | null;
}

export interface ThreadView {
  readonly ancestors: readonly SocialPost[];
  readonly post: SocialPost;
  readonly replies: readonly SocialPost[];
  readonly cursor: string | null;
}

export interface FollowedSource {
  readonly id: string;
  readonly provider: Provider;
  readonly sourceType: FollowedSourceKind;
  readonly title: string;
  readonly description: string | null;
  readonly remoteId: string;
}

export interface SourcePage {
  readonly sources: readonly FollowedSource[];
  readonly cursor: string | null;
}

export interface NotificationItem {
  readonly id: string;
  readonly kind: NotificationKind;
  readonly createdAt: string;
  readonly actor: ProviderActor;
  readonly post: SocialPost | null;
  readonly unread: boolean;
}

export interface NotificationPage {
  readonly notifications: readonly NotificationItem[];
  readonly cursor: string | null;
}

export type SocialAction =
  | { readonly kind: "LIKE"; readonly postId: string }
  | { readonly kind: "UNLIKE"; readonly postId: string }
  | { readonly kind: "REPOST"; readonly postId: string }
  | { readonly kind: "UNDO_REPOST"; readonly postId: string }
  | { readonly kind: "FOLLOW"; readonly profileId: string }
  | { readonly kind: "UNFOLLOW"; readonly profileId: string }
  | { readonly kind: "REPLY"; readonly postId: string; readonly text: string };

export interface SocialActionResult {
  readonly targetId: string;
  readonly viewer: ViewerState | null;
  readonly followed: boolean | null;
  readonly recordId: string | null;
  readonly createdPost: SocialPost | null;
}
