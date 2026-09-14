import type { Provider } from "../types";
export const providerNames: Readonly<Record<Provider, string>> = { BLUESKY: "Bluesky", MASTODON: "Mastodon" };
export const protocolNames: Readonly<Record<Provider, string>> = { BLUESKY: "AT Protocol", MASTODON: "ActivityPub" };
