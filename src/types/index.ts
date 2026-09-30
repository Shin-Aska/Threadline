/** Provider identifiers shared by account, post, and source contracts. */
export type Provider = "MASTODON" | "BLUESKY";
/** Controls whether destinations share the smallest limit or split independently. */
export type PublishingPolicy = "COMMON_LIMIT" | "ADAPTIVE" | "ALWAYS_THREAD";
/** Text, media, and feature limits the composer uses for one destination. */
export interface Capabilities { maxTextLength: number; countingPolicy: "GRAPHEME" | "PLATFORM_NATIVE"; reservedUrlLength: number | null; maxMediaAttachments: number; supportedMediaTypes: string[]; supportsPolls: boolean; supportsContentWarnings: boolean; readonly maxVideoBytes?: number | null; readonly maxVideoDurationMs?: number | null }
/** A saved destination; credentials and live provider objects are stored separately. */
export interface Account { id: string; provider: Provider; handle: string; displayName: string; instanceUrl: string | null; did: string | null; capabilities: Capabilities }
/** User-authored content and the destination selection before provider-specific splitting. */
export interface CanonicalPost { text: string; media: MediaAttachment[]; policy: PublishingPolicy; destinationAccountIds: string[] }
/** User-selected media plus the encoded bytes sent through the desktop bridge. */
export interface MediaAttachment { id: string; name: string; mimeType: string; sizeBytes: number; altText: string; dataBase64: string; readonly durationMs?: number | null }
/** Preview of the text parts that will be sent to one account. */
export interface DestinationPreview { accountId: string; label: string; maxLength: number; parts: string[] }
/** Composer preview across destinations, including the shared constraint when applicable. */
export interface PublishingPreview { graphemeCount: number; effectiveLimit: number | null; limitingAccountId: string | null; destinations: DestinationPreview[] }
/** Confirmed or unresolved result for publishing one post to one account. */
export interface Publication { accountId: string; status: "PUBLISHED" | "FAILED" | "UNCERTAIN" | "BLOCKED"; remotePostIds: string[]; error: string | null }
/** Immediate result for one canonical post sent to multiple accounts. */
export interface PublishResult { canonicalId: string; publications: Publication[] }
/** Runtime availability of provider-backed features in the current workspace. */
export type WorkspaceMode = "BROWSER" | "LIVE" | "DISCONNECTED";
/** Account inventory and live-connection state provided to the workspace UI. */
export interface WorkspaceState {
  readonly accounts: Account[];
  readonly connectedAccountIds: readonly string[];
  readonly mode: WorkspaceMode;
}

/** Provider-specific hashtag activity; use `kind` to interpret the available count. */
export type HashtagActivity = { readonly kind: "MASTODON"; readonly uses: number; readonly days: number } | { readonly kind: "BLUESKY"; readonly matches: number | null } | { readonly kind: "UNAVAILABLE" };
/** A suggested hashtag and the activity reported by its source provider. */
export interface HashtagSuggestion { readonly name: string; readonly activity: HashtagActivity }

/** Identifies which connected account supplied a unified item. */
export interface SourceAttribution { readonly accountId: string; readonly accountHandle: string; readonly provider: Provider }
/** Provider-neutral author identity used in unified feeds and discovery. */
export interface UnifiedActor { readonly id: string; readonly displayName: string; readonly handle: string; readonly avatarUrl: string | null }
/** Provider-neutral post view with the source accounts and available actions. */
export interface UnifiedPost {
  readonly canonicalKey: string; readonly provider: Provider; readonly remoteId: string; readonly remoteUrl: string;
  readonly author: UnifiedActor; readonly text: string; readonly contentWarning?: string | null; readonly sensitive?: boolean; readonly createdAt: string; readonly media: readonly { url: string; alt: string; type: string; thumbnail?: string | null }[];
  readonly sources: readonly SourceAttribution[]; readonly metrics: { replies?: number; reposts?: number; likes?: number };
  readonly capabilities: { openOriginal: true; reply: boolean; like: boolean; repost: boolean };
}
/** One page of aggregated posts; a non-null cursor signals more provider data. */
export interface UnifiedFeedPage { readonly posts: readonly UnifiedPost[]; readonly cursor: string | null }
/** A normalized discovery topic with merged source attribution and optional trends. */
export interface UnifiedTopic { readonly key: string; readonly name: string; readonly sources: readonly SourceAttribution[]; readonly postCount?: number; readonly participantCount?: number; readonly history?: readonly number[] }
/** Combined topics, accounts, and posts returned by discovery. */
export interface UnifiedDiscoveryResult { readonly topics: readonly UnifiedTopic[]; readonly suggestedAccounts: readonly UnifiedActor[]; readonly popularPosts: readonly UnifiedPost[] }
/** Source category used by the unified following view. */
export type UnifiedSourceType = "PERSON" | "TOPIC" | "FEED";
/** A provider source attached to a connected account's following collection. */
export interface UnifiedSource { readonly id: string; readonly provider: Provider; readonly type: UnifiedSourceType; readonly title: string; readonly description?: string; readonly accountId: string; readonly remoteId: string }
/** Named grouping of followed people, topics, and feeds. */
export interface FollowingCollection { readonly id: string; readonly title: string; readonly description: string; readonly sources: readonly UnifiedSource[] }
/** A provider-scoped failure retained alongside successful unified results. */
export interface ProviderFailure { readonly accountId: string; readonly provider: Provider; readonly message: string; readonly authExpired?: boolean }
