import { desktopApi } from "../desktop";
import type { Account, ProviderFailure, UnifiedDiscoveryResult, UnifiedFeedPage, UnifiedSource } from "../../types";
import { mergePosts, mergeTopics } from "./aggregate";

export type SettledFeed = { posts: ReturnType<typeof mergePosts>; pages: Readonly<Record<string, UnifiedFeedPage>>; failures: readonly ProviderFailure[] };
export async function getUnifiedTimeline(accounts: readonly Account[], cursors: Readonly<Record<string, string | null>> = {}, signal?: AbortSignal): Promise<SettledFeed> {
  const settled: ({ account: Account; page: UnifiedFeedPage } | { account: Account; error: string })[] = await Promise.all(accounts.map(async account => {
    try { return { account, page: await desktopApi.social.timeline(account.id, cursors[account.id] ?? null, signal) }; }
    catch (error) { return { account, error: error instanceof Error ? error.message : String(error) }; }
  }));
  const pages: Record<string, UnifiedFeedPage> = {}; for (const result of settled) if ("page" in result) pages[result.account.id] = result.page;
  return { pages, posts: mergePosts(Object.values(pages)), failures: settled.flatMap(result => "error" in result ? [{ accountId: result.account.id, provider: result.account.provider, message: result.error, authExpired: /auth|unauthor|expired/i.test(result.error) }] : []) };
}
export async function getUnifiedDiscovery(accounts: readonly Account[], signal?: AbortSignal) {
  const settled = await Promise.all(accounts.map(async account => { try { return { account, value: await desktopApi.social.discovery(account.id, signal) }; } catch (error) { return { account, error: error instanceof Error ? error.message : String(error) }; } }));
  const values = settled.flatMap(item => item.value ? [item.value] : []);
  const result: UnifiedDiscoveryResult = { topics: mergeTopics(values.flatMap(value => value.topics)), suggestedAccounts: [...new Map(values.flatMap(value => value.suggestedAccounts).map(actor => [actor.id, actor])).values()], popularPosts: mergePosts(values.map(value => ({ posts: value.popularPosts, cursor: null }))) };
  return { result, failures: settled.flatMap(item => item.error ? [{ accountId: item.account.id, provider: item.account.provider, message: item.error }] : []) };
}
export async function getUnifiedFollowing(accounts: readonly Account[], signal?: AbortSignal) {
  const settled = await Promise.all(accounts.map(async account => { try { return await desktopApi.social.following(account.id, signal); } catch { return [] as UnifiedSource[]; } }));
  return settled.flat();
}
