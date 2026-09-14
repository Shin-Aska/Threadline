export type Provider = "MASTODON" | "BLUESKY";
export type PublishingPolicy = "COMMON_LIMIT" | "ADAPTIVE" | "ALWAYS_THREAD";
export interface Capabilities { maxTextLength: number; countingPolicy: "GRAPHEME" | "PLATFORM_NATIVE"; reservedUrlLength: number | null; maxMediaAttachments: number; supportedMediaTypes: string[]; supportsPolls: boolean; supportsContentWarnings: boolean }
export interface Account { id: string; provider: Provider; handle: string; displayName: string; instanceUrl: string | null; did: string | null; capabilities: Capabilities }
export interface CanonicalPost { text: string; media: MediaAttachment[]; policy: PublishingPolicy; destinationAccountIds: string[] }
export interface MediaAttachment { id: string; kind: string; altText?: string }
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
