import { invoke } from "@tauri-apps/api/core";
import type { Account } from "../../types";
import type {
  BlueskyOAuthRequest,
  MastodonOAuthRequest,
  NativeOAuthAttempt,
} from "../../types/oauth";

const desktopAvailable = (): boolean => "__TAURI_INTERNALS__" in window;

class DesktopOAuthUnavailableError extends Error {
  constructor() {
    super("Open Threadline desktop to sign in with a browser.");
    this.name = "DesktopOAuthUnavailableError";
  }
}

const cancel = (flowId: string): Promise<void> =>
  desktopAvailable()
    ? invoke<void>("cancel_oauth_login", { flowId })
    : Promise.reject(new DesktopOAuthUnavailableError());

export const connectMastodonWithOAuth = (
  request: MastodonOAuthRequest,
): NativeOAuthAttempt => {
  const flowId = crypto.randomUUID();
  const completion = desktopAvailable()
    ? invoke<Account>("connect_mastodon_oauth", { flowId, instanceUrl: request.instanceUrl })
    : Promise.reject(new DesktopOAuthUnavailableError());
  return { flowId, completion, cancel: () => cancel(flowId) };
};

export const connectBlueskyWithOAuth = (
  request: BlueskyOAuthRequest,
): NativeOAuthAttempt => {
  const flowId = crypto.randomUUID();
  const completion = desktopAvailable()
    ? invoke<Account>("connect_bluesky_oauth", {
        flowId,
        identifier: request.identifier,
        clientMetadataUrl: request.clientMetadataUrl ?? null,
      })
    : Promise.reject(new DesktopOAuthUnavailableError());
  return { flowId, completion, cancel: () => cancel(flowId) };
};
