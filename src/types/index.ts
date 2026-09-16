export type Provider = "MASTODON" | "BLUESKY";
export type PublishingPolicy = "COMMON_LIMIT" | "ADAPTIVE" | "ALWAYS_THREAD";
export interface Capabilities { maxTextLength: number; countingPolicy: "GRAPHEME" | "PLATFORM_NATIVE"; reservedUrlLength: number | null; maxMediaAttachments: number; supportedMediaTypes: string[]; supportsPolls: boolean; supportsContentWarnings: boolean }
export interface Account { id: string; provider: Provider; handle: string; displayName: string; instanceUrl: string | null; did: string | null; capabilities: Capabilities }
export interface CanonicalPost { text: string; media: MediaAttachment[]; policy: PublishingPolicy; destinationAccountIds: string[] }
export interface MediaAttachment { id: string; name: string; mimeType: string; sizeBytes: number; altText: string; dataBase64: string }
export interface DestinationPreview { accountId: string; label: string; maxLength: number; parts: string[] }
export interface PublishingPreview { graphemeCount: number; effectiveLimit: number | null; limitingAccountId: string | null; destinations: DestinationPreview[] }
export interface Publication { accountId: string; status: "PUBLISHED" | "FAILED"; remotePostIds: string[]; error: string | null }
export interface PublishResult { canonicalId: string; publications: Publication[] }
export type WorkspaceMode = "BROWSER" | "LIVE" | "DISCONNECTED";
export interface WorkspaceState {
  readonly accounts: Account[];
  readonly connectedAccountIds: readonly string[];
  readonly mode: WorkspaceMode;
}

export type HashtagActivity = { readonly kind: "MASTODON"; readonly uses: number; readonly days: number } | { readonly kind: "BLUESKY"; readonly matches: number | null } | { readonly kind: "UNAVAILABLE" };
export interface HashtagSuggestion { readonly name: string; readonly activity: HashtagActivity }

export interface SourceAttribution { readonly accountId: string; readonly accountHandle: string; readonly provider: Provider }
export interface UnifiedActor { readonly id: string; readonly displayName: string; readonly handle: string; readonly avatarUrl: string | null }
export interface UnifiedPost {
  readonly canonicalKey: string; readonly provider: Provider; readonly remoteId: string; readonly remoteUrl: string;
  readonly author: UnifiedActor; readonly text: string; readonly createdAt: string; readonly media: readonly { url: string; alt: string; type: string }[];
  readonly sources: readonly SourceAttribution[]; readonly metrics: { replies?: number; reposts?: number; likes?: number };
  readonly capabilities: { openOriginal: true; reply: boolean; like: boolean; repost: boolean };
}
export interface UnifiedFeedPage { readonly posts: readonly UnifiedPost[]; readonly cursor: string | null }
export interface UnifiedTopic { readonly key: string; readonly name: string; readonly sources: readonly SourceAttribution[]; readonly postCount?: number; readonly participantCount?: number; readonly history?: readonly number[] }
export interface UnifiedDiscoveryResult { readonly topics: readonly UnifiedTopic[]; readonly suggestedAccounts: readonly UnifiedActor[]; readonly popularPosts: readonly UnifiedPost[] }
export type UnifiedSourceType = "PERSON" | "TOPIC" | "FEED";
export interface UnifiedSource { readonly id: string; readonly provider: Provider; readonly type: UnifiedSourceType; readonly title: string; readonly description?: string; readonly accountId: string; readonly remoteId: string }
export interface FollowingCollection { readonly id: string; readonly title: string; readonly description: string; readonly sources: readonly UnifiedSource[] }
export interface ProviderFailure { readonly accountId: string; readonly provider: Provider; readonly message: string; readonly authExpired?: boolean }
