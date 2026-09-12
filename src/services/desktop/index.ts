import { invoke } from "@tauri-apps/api/core";
import type { Account, CanonicalPost, PublishResult, PublishingPreview } from "../../types";
export interface DesktopApi {
  accounts: { list(): Promise<Account[]> };
  social: { preview(post: CanonicalPost): Promise<PublishingPreview>; publish(post: CanonicalPost): Promise<PublishResult> };
  storage: { health(): Promise<string> };
  notifications: { list(): Promise<readonly never[]> };
}
const runningInTauri = (): boolean => "__TAURI_INTERNALS__" in window;
const mockAccounts: Account[] = [
  { id:"bsky-alice",provider:"BLUESKY",handle:"alice.bsky.social",displayName:"Alice",instanceUrl:null,did:"did:plc:threadline-alice",capabilities:{maxTextLength:300,countingPolicy:"GRAPHEME",reservedUrlLength:null,maxMediaAttachments:4,supportedMediaTypes:["image/jpeg","image/png"],supportsPolls:false,supportsContentWarnings:false}},
  { id:"mastodon-social",provider:"MASTODON",handle:"@river@mastodon.social",displayName:"River",instanceUrl:"https://mastodon.social",did:null,capabilities:{maxTextLength:500,countingPolicy:"GRAPHEME",reservedUrlLength:23,maxMediaAttachments:4,supportedMediaTypes:["image/jpeg","image/png","video/mp4"],supportsPolls:true,supportsContentWarnings:true}},
  { id:"mastodon-long",provider:"MASTODON",handle:"@sora@long.example",displayName:"Sora",instanceUrl:"https://long.example",did:null,capabilities:{maxTextLength:5000,countingPolicy:"GRAPHEME",reservedUrlLength:23,maxMediaAttachments:8,supportedMediaTypes:["image/jpeg","image/png"],supportsPolls:true,supportsContentWarnings:true}}
];
const unavailable = (): never => { throw new Error("Native preview is available in the Tauri desktop application. Run npm run tauri dev."); };
export const desktopApi: DesktopApi = {
  accounts: { list: () => runningInTauri() ? invoke<Account[]>("list_accounts") : Promise.resolve(mockAccounts) },
  social: { preview: (post) => runningInTauri() ? invoke("preview_post", { post }) : Promise.reject(unavailable()), publish: (post) => runningInTauri() ? invoke("publish_post", { post }) : Promise.reject(unavailable()) },
  storage: { health: () => runningInTauri() ? invoke("storage_health") : Promise.resolve("browser preview") },
  notifications: { list: () => Promise.resolve([]) }
};
