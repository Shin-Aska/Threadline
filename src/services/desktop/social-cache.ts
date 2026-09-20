export interface SocialReadCacheOptions {
  readonly maxEntries: number;
  readonly now?: () => number;
}

export interface SocialReadRequest<T> {
  readonly accountId: string;
  readonly command: string;
  readonly args: Readonly<Record<string, unknown>>;
  readonly ttlMs: number;
  readonly refresh?: boolean | undefined;
  readonly load: () => Promise<T>;
}

interface CacheEntry {
  readonly accountId: string;
  readonly promise: Promise<unknown>;
  expiresAt: number;
}

/**
 * Keeps reads isolated by acting account; mutations invalidate that account before
 * and after the native call so cached state cannot outlive a mutation attempt.
 */
export class SocialReadCache {
  readonly #entries = new Map<string, CacheEntry>();
  readonly #maxEntries: number;
  readonly #now: () => number;

  constructor(options: SocialReadCacheOptions) {
    if (!Number.isInteger(options.maxEntries) || options.maxEntries < 1) {
      throw new RangeError("Social read cache maxEntries must be a positive integer");
    }
    this.#maxEntries = options.maxEntries;
    this.#now = options.now ?? Date.now;
  }

  get size(): number {
    return this.#entries.size;
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

    const entry: CacheEntry = {
      accountId: request.accountId,
      expiresAt: Number.POSITIVE_INFINITY,
      promise: request.load(),
    };
    this.#entries.set(key, entry);
    this.#evictOverflow();
    void entry.promise.then(
      () => {
        if (this.#entries.get(key) === entry) entry.expiresAt = this.#now() + request.ttlMs;
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
    while (this.#entries.size > this.#maxEntries) {
      const oldest = this.#entries.keys().next().value;
      if (oldest === undefined) return;
      this.#entries.delete(oldest);
    }
  }
}

export async function invalidateAroundMutation<T>(cache: SocialReadCache, accountId: string, mutation: () => Promise<T>): Promise<T> {
  cache.invalidateAccount(accountId);
  try {
    return await mutation();
  } finally {
    cache.invalidateAccount(accountId);
  }
}
