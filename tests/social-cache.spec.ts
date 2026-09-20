import { expect, test } from "@playwright/test";
import { invalidateAroundMutation, SocialReadCache } from "../src/services/desktop/social-cache";

interface Deferred<T> {
  readonly promise: Promise<T>;
  readonly resolve: (value: T) => void;
  readonly reject: (cause: Error) => void;
}

function deferred<T>(): Deferred<T> {
  let resolve: ((value: T) => void) | undefined;
  let reject: ((cause: Error) => void) | undefined;
  const promise = new Promise<T>((accept, decline) => { resolve = accept; reject = decline; });
  if (!resolve || !reject) throw new Error("Deferred callbacks were not initialized");
  return { promise, resolve, reject };
}

test("reuses an account request until its TTL expires", async () => {
  let now = 1_000;
  let invocations = 0;
  const cache = new SocialReadCache({ maxEntries: 8, now: () => now });
  const request = () => cache.read({ accountId: "account-a", command: "get_home_feed", args: { accountId: "account-a", cursor: null }, ttlMs: 30_000, load: async () => ({ invocation: ++invocations }) });

  const first = await request();
  const cached = await request();
  now += 30_001;
  const expired = await request();

  expect([first.invocation, cached.invocation, expired.invocation]).toEqual([1, 1, 2]);
  expect(invocations).toBe(2);
});

test("isolates identical read arguments by account", async () => {
  let invocations = 0;
  const cache = new SocialReadCache({ maxEntries: 8 });
  const request = (accountId: string) => cache.read({ accountId, command: "get_notifications", args: { cursor: null }, ttlMs: 20_000, load: async () => ++invocations });

  const first = await request("account-a");
  const second = await request("account-b");

  expect([first, second]).toEqual([1, 2]);
});

test("coalesces concurrent requests and retries rejected requests", async () => {
  const pending = deferred<number>();
  let invocations = 0;
  const cache = new SocialReadCache({ maxEntries: 8 });
  const sharedRequest = () => cache.read({ accountId: "account-a", command: "get_thread", args: { accountId: "account-a", postId: "post-1" }, ttlMs: 30_000, load: () => { invocations += 1; return pending.promise; } });

  const first = sharedRequest();
  const second = sharedRequest();
  pending.resolve(7);

  expect(await Promise.all([first, second])).toEqual([7, 7]);
  expect(invocations).toBe(1);

  const failedCache = new SocialReadCache({ maxEntries: 8 });
  let attempts = 0;
  const retry = () => failedCache.read({ accountId: "account-a", command: "get_profile", args: { accountId: "account-a", profileId: "profile-1" }, ttlMs: 30_000, load: async () => { attempts += 1; if (attempts === 1) throw new TypeError("provider unavailable"); return 9; } });
  await expect(retry()).rejects.toThrow("provider unavailable");
  await expect(retry()).resolves.toBe(9);
  expect(attempts).toBe(2);
});

test("bounds retained entries", async () => {
  let invocations = 0;
  const cache = new SocialReadCache({ maxEntries: 2 });
  const request = (cursor: string) => cache.read({ accountId: "account-a", command: "get_home_feed", args: { accountId: "account-a", cursor }, ttlMs: 30_000, load: async () => ++invocations });

  await request("one");
  await request("two");
  await request("three");
  await request("one");

  expect(invocations).toBe(4);
  expect(cache.size).toBe(2);
});

test("refresh bypasses a cached response", async () => {
  let invocations = 0;
  const cache = new SocialReadCache({ maxEntries: 8 });
  const request = (refresh = false) => cache.read({ accountId: "account-a", command: "get_home_feed", args: { accountId: "account-a", cursor: null }, ttlMs: 30_000, refresh, load: async () => ++invocations });

  expect(await request()).toBe(1);
  expect(await request()).toBe(1);
  expect(await request(true)).toBe(2);

  expect(invocations).toBe(2);
});

test("account invalidation prevents an older in-flight response from being reused", async () => {
  const stale = deferred<number>();
  let invocations = 0;
  const cache = new SocialReadCache({ maxEntries: 8 });
  const request = () => cache.read({ accountId: "account-a", command: "get_home_feed", args: { accountId: "account-a", cursor: null }, ttlMs: 30_000, load: () => { invocations += 1; return invocations === 1 ? stale.promise : Promise.resolve(invocations); } });

  const inFlight = request();
  cache.invalidateAccount("account-a");
  stale.resolve(1);
  expect(await inFlight).toBe(1);
  expect(await request()).toBe(2);

  expect(invocations).toBe(2);
});

test("mutation invalidation clears reads started before and during the mutation", async () => {
  const stale = deferred<number>();
  const mutation = deferred<void>();
  let invocations = 0;
  const cache = new SocialReadCache({ maxEntries: 8 });
  const request = () => cache.read({ accountId: "account-a", command: "get_home_feed", args: { accountId: "account-a", cursor: null }, ttlMs: 30_000, load: () => { invocations += 1; return invocations === 1 ? stale.promise : Promise.resolve(invocations); } });

  const beforeMutation = request();
  const action = invalidateAroundMutation(cache, "account-a", () => mutation.promise);
  stale.resolve(1);
  expect(await beforeMutation).toBe(1);
  expect(await request()).toBe(2);
  mutation.resolve();
  await action;
  expect(await request()).toBe(3);

  expect(invocations).toBe(3);
});

test("mutation invalidation clears reads when the mutation fails", async () => {
  let invocations = 0;
  const cache = new SocialReadCache({ maxEntries: 8 });
  const request = () => cache.read({ accountId: "account-a", command: "get_notifications", args: { accountId: "account-a", cursor: null }, ttlMs: 20_000, load: async () => ++invocations });
  await request();

  await expect(invalidateAroundMutation(cache, "account-a", async () => { throw new TypeError("uncertain provider outcome"); })).rejects.toThrow("uncertain provider outcome");
  await request();

  expect(invocations).toBe(2);
});

test("social API coalesces native invokes and invalidates after an action", async ({ page }) => {
  await page.addInitScript(() => {
    const counts: Record<string, number> = {};
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string) => {
      counts[command] = (counts[command] ?? 0) + 1;
      sessionStorage.setItem(`social-cache-invokes:${command}`, String(counts[command]));
      if (command === "get_workspace") return { accounts: [], connectedAccountIds: [], mode: "LIVE" };
      if (command === "get_home_feed") return { posts: [], cursor: null };
      if (command === "perform_social_action") return { targetId: "post-1", viewer: null, followed: null, recordId: "like-1", createdPost: null };
      throw new TypeError(`Unexpected command ${command}`);
    } } });
  });
  await page.goto("/");

  const counts = await page.evaluate(async () => {
    const modulePath = "/src/services/desktop/social.ts";
    const module: typeof import("../src/services/desktop/social") = await import(modulePath);
    await Promise.all([
      module.socialApi.home("cache-account"),
      module.socialApi.home("cache-account"),
    ]);
    await module.socialApi.home("cache-account");
    await module.socialApi.act("cache-account", { kind: "LIKE", postId: "post-1" });
    await module.socialApi.home("cache-account");
    return [
      Number(sessionStorage.getItem("social-cache-invokes:get_home_feed")),
      Number(sessionStorage.getItem("social-cache-invokes:perform_social_action")),
    ];
  });

  expect(counts).toEqual([2, 1]);
});
