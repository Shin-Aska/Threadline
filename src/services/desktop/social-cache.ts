/** Capacity, post-count, and clock settings for a social read cache. */
export interface SocialReadCacheOptions {
  readonly maxEntries: number;
  readonly maxPosts?: number;
  readonly now?: () => number;
}

/** One cacheable provider read and the metadata used to expire and bound it. */
export interface SocialReadRequest<T> {
  readonly accountId: string;
  readonly command: string;
  readonly args: Readonly<Record<string, unknown>>;
  readonly ttlMs: number;
  readonly refresh?: boolean | undefined;
  readonly countPosts?: (value: T) => number;
  readonly load: () => Promise<T>;
}

interface CacheEntry {
  readonly accountId: string;
  readonly promise: Promise<unknown>;
  expiresAt: number;
  postCount: number;
}

/**
 * Keeps reads isolated by acting account; mutations invalidate that account before
 * and after the native call so cached state cannot outlive a mutation attempt.
 */
/** Caches in-flight and completed reads per account with entry and post-count bounds. */
export class SocialReadCache {
  readonly #entries = new Map<string, CacheEntry>();
  readonly #maxEntries: number;
  readonly #now: () => number;
  #maxPosts: number;

  constructor(options: SocialReadCacheOptions) {
    if (!Number.isInteger(options.maxEntries) || options.maxEntries < 1) {
      throw new RangeError("Social read cache maxEntries must be a positive integer");
    }
    if (options.maxPosts !== undefined && (!Number.isInteger(options.maxPosts) || options.maxPosts < 0)) {
      throw new RangeError("Social read cache maxPosts must be a non-negative integer");
    }
    this.#maxEntries = options.maxEntries;
    this.#maxPosts = options.maxPosts ?? Number.POSITIVE_INFINITY;
    this.#now = options.now ?? Date.now;
  }

  get size(): number {
    return this.#entries.size;
  }

  get maxPosts(): number {
    return this.#maxPosts;
  }

  setMaxPosts(value: number): void {
    if (!Number.isInteger(value) || value < 0) throw new RangeError("Social read cache maxPosts must be a non-negative integer");
    this.#maxPosts = value;
    this.#evictOverflow();
  }

  read<T>(request: SocialReadRequest<T>): Promise<T> {
    const key = JSON.stringify([request.accountId, request.command, request.args]);
    if (request.refresh) this.#entries.delete(key);
    const cached = this.#entries.get(key);
    if (cached && cached.expiresAt > this.#now()) {
      this.#entries.delete(key);
      this.#entries.set(key, cached);
      return cached.promise as Promise<T>;
    }
    if (cached) this.#entries.delete(key);

    const promise = request.load();
    const entry: CacheEntry = {
      accountId: request.accountId,
      expiresAt: Number.POSITIVE_INFINITY,
      postCount: 0,
      promise,
    };
    this.#entries.set(key, entry);
    this.#evictOverflow();
    void promise.then(
      value => {
        if (this.#entries.get(key) === entry) {
          entry.expiresAt = this.#now() + request.ttlMs;
          entry.postCount = request.countPosts?.(value) ?? 0;
          this.#evictOverflow();
        }
      },
      () => {
        if (this.#entries.get(key) === entry) this.#entries.delete(key);
      },
    );
    return entry.promise as Promise<T>;
  }

  invalidateAccount(accountId: string): void {
    for (const [key, entry] of this.#entries) {
      if (entry.accountId === accountId) this.#entries.delete(key);
    }
  }

  clear(): void {
    this.#entries.clear();
  }

  #evictOverflow(): void {
    let postCount = [...this.#entries.values()].reduce((total, entry) => total + entry.postCount, 0);
    while (this.#entries.size > this.#maxEntries || postCount > this.#maxPosts) {
      const oldest = this.#entries.keys().next().value;
      if (oldest === undefined) return;
      postCount -= this.#entries.get(oldest)?.postCount ?? 0;
      this.#entries.delete(oldest);
    }
  }
}

/** Clears an account's reads before and after a mutation, including failed attempts. */
export async function invalidateAroundMutation<T>(cache: SocialReadCache, accountId: string, mutation: () => Promise<T>): Promise<T> {
  cache.invalidateAccount(accountId);
  try {
    return await mutation();
  } finally {
    cache.invalidateAccount(accountId);
  }
}
