import type { Account } from "./index";

export type OAuthProvider = "MASTODON" | "BLUESKY";

export type OAuthLoginState =
  | { readonly status: "IDLE" }
  | { readonly status: "PENDING"; readonly flowId: string; readonly provider: OAuthProvider }
  | { readonly status: "SUCCEEDED"; readonly account: Account }
  | { readonly status: "CANCELLED" }
  | { readonly status: "TIMED_OUT" }
  | { readonly status: "FAILED"; readonly message: string };

export type MastodonOAuthRequest = {
  readonly instanceUrl: string;
};

export type BlueskyOAuthRequest = {
  readonly identifier: string;
  readonly clientMetadataUrl?: string;
};

export type NativeOAuthAttempt = {
  readonly flowId: string;
  readonly completion: Promise<Account>;
  readonly cancel: () => Promise<void>;
};
