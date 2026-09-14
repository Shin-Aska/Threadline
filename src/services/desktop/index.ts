import { invoke } from "@tauri-apps/api/core";
import type { Account, CanonicalPost, PublishResult, PublishingPreview, WorkspaceState } from "../../types";
export interface DesktopApi {
  workspace: { load(): Promise<WorkspaceState> };
  accounts: { list(): Promise<Account[]>; connectBluesky(serviceUrl: string, identifier: string, appPassword: string): Promise<Account>; connectMastodon(baseUrl: string, accessToken: string): Promise<Account>; remove(accountId: string): Promise<void> };
  social: { preview(post: CanonicalPost): Promise<PublishingPreview>; publish(post: CanonicalPost): Promise<PublishResult> };
  storage: { health(): Promise<string> };
  notifications: { list(): Promise<readonly never[]> };
}
const runningInTauri = (): boolean => "__TAURI_INTERNALS__" in window;
class DesktopOnlyError extends Error {
  constructor() { super("Open Threadline desktop to connect accounts, plan threads, and publish."); this.name = "DesktopOnlyError"; }
}
const unavailable = (): DesktopOnlyError => new DesktopOnlyError();
export const desktopApi: DesktopApi = {
  workspace: { load: () => runningInTauri() ? invoke<WorkspaceState>("get_workspace") : Promise.resolve({ accounts: [], connectedAccountIds: [], mode: "BROWSER" }) },
  accounts: {
    list: () => runningInTauri() ? invoke<Account[]>("list_accounts") : Promise.resolve([]),
    connectBluesky: (serviceUrl, identifier, appPassword) => runningInTauri() ? invoke<Account>("connect_bluesky", { serviceUrl, identifier, appPassword }) : Promise.reject(unavailable()),
    connectMastodon: (baseUrl, accessToken) => runningInTauri() ? invoke<Account>("connect_mastodon", { baseUrl, accessToken }) : Promise.reject(unavailable()),
    remove: (accountId) => runningInTauri() ? invoke<void>("remove_account", { accountId }) : Promise.reject(unavailable()),
  },
  social: { preview: (post) => runningInTauri() ? invoke("preview_post", { post }) : Promise.reject(unavailable()), publish: (post) => runningInTauri() ? invoke("publish_post", { post }) : Promise.reject(unavailable()) },
  storage: { health: () => runningInTauri() ? invoke("storage_health") : Promise.resolve("browser preview") },
  notifications: { list: () => Promise.resolve([]) }
};
