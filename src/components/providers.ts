import type { Provider } from "../types";
/** Human-readable provider labels used in interface copy. */
export const providerNames: Readonly<Record<Provider, string>> = { BLUESKY: "Bluesky", MASTODON: "Mastodon" };
/** Protocol names shown when explaining each provider integration. */
export const protocolNames: Readonly<Record<Provider, string>> = { BLUESKY: "AT Protocol", MASTODON: "ActivityPub" };
