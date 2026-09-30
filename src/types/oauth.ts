import type { Account } from "./index";

/** Provider whose authorization flow is being started or completed. */
export type OAuthProvider = "MASTODON" | "BLUESKY";

/** Observable state of an OAuth login, keyed by status for exhaustive handling. */
export type OAuthLoginState =
  | { readonly status: "IDLE" }
  | { readonly status: "PENDING"; readonly flowId: string; readonly provider: OAuthProvider }
  | { readonly status: "SUCCEEDED"; readonly account: Account }
  | { readonly status: "CANCELLED" }
  | { readonly status: "TIMED_OUT" }
  | { readonly status: "FAILED"; readonly message: string };

/** Mastodon login input; the instance URL selects the authorization server. */
export type MastodonOAuthRequest = {
  readonly instanceUrl: string;
};

/** Bluesky login input, with optional client metadata for OAuth discovery. */
export type BlueskyOAuthRequest = {
  readonly identifier: string;
  readonly clientMetadataUrl?: string;
};

/** Handle for a native login attempt, allowing the UI to await or cancel it. */
export type NativeOAuthAttempt = {
  readonly flowId: string;
  readonly completion: Promise<Account>;
  readonly cancel: () => Promise<void>;
};
