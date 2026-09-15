import type { Account, HashtagActivity, Provider } from "../types";
import type { HashtagResult } from "./useHashtags";

export interface HashtagRow {
  readonly name: string;
  readonly sources: readonly { readonly account: Account; readonly activity: HashtagActivity }[];
}
export function aggregateHashtags(accounts: readonly Account[], results: Readonly<Record<string, HashtagResult>>, filter: Provider | "ALL"): readonly HashtagRow[] {
  const rows = new Map<string, { name: string; sources: { account: Account; activity: HashtagActivity }[] }>();
  for (const account of accounts) {
    if (filter !== "ALL" && account.provider !== filter) continue;
    const result = results[account.id];
    if (result?.status !== "ready") continue;
    for (const suggestion of result.suggestions) {
      const key = suggestion.name.toLowerCase();
      const row = rows.get(key) ?? { name: suggestion.name, sources: [] };
      if (!row.sources.some(source => source.account.id === account.id)) row.sources.push({ account, activity: suggestion.activity });
      rows.set(key, row);
    }
  }
  return [...rows.values()];
}
const compact = new Intl.NumberFormat(undefined, { notation: "compact", maximumFractionDigits: 1 });
export function hashtagActivity(row: HashtagRow): { readonly label: string; readonly detail: string } {
  let matches: number | null = null;
  const instances = new Map<string, number>();
  const details: string[] = [];
  let unknown = false;
  for (const { account, activity } of row.sources) {
    let description: string;
    switch (activity.kind) {
      case "BLUESKY":
        if (activity.matches !== null) matches = Math.max(matches ?? 0, activity.matches);
        else unknown = true;
        description = activity.matches === null ? "Search count unavailable" : "approximately " + activity.matches.toLocaleString() + " search matches";
        break;
      case "MASTODON": {
        const instance = (account.instanceUrl ?? account.id).replace(/\/+$/, "").toLowerCase();
        instances.set(instance, Math.max(instances.get(instance) ?? 0, activity.uses));
        description = activity.uses.toLocaleString() + " uses · " + activity.days + " days observed";
        break;
      }
      case "UNAVAILABLE":
        unknown = true;
        description = "Activity unavailable";
        break;
    }
    details.push(account.displayName + " (" + account.handle + ", " + (account.instanceUrl ?? account.provider) + "): " + description);
  }
  const labels = [];
  if (matches !== null) labels.push("≈ " + compact.format(matches) + " matches");
  if (instances.size) labels.push(compact.format([...instances.values()].reduce((sum, value) => sum + value, 0)) + " uses");
  const label = labels.length ? labels.join(" · ") + (unknown ? " *" : "") : "Count unavailable";
  return { label, detail: details.join("\n") + "\nShared sources counted once. Instance observations may overlap; matches and uses are different measures." + (unknown ? "\n* Some sources have no count." : "") };
}
