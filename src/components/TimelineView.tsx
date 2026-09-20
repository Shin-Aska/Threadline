import { Filter, Home } from "lucide-react";
import { useMemo, useState } from "react";
import { useBrowsingScope } from "../hooks/useBrowsingScope";
import { useSocialFeed } from "../hooks/useSocialFeed";
import { presentSocialPosts, type SocialOrder } from "../services/social/presentation";
import type { WorkspaceState } from "../types";
import { FailureStrip } from "./unified/FailureStrip";
import { SocialFeedToolbar } from "./unified/SocialFeedToolbar";
import { SocialPostCard } from "./unified/SocialPostCard";
import { SourceControls } from "./unified/SourceControls";
import { ViewHeader } from "./unified/ViewHeader";

interface TimelineProps {
  readonly workspace: WorkspaceState;
  readonly onPost: (accountId: string, postId: string) => void;
  readonly onProfile: (accountId: string, profileId: string) => void;
  readonly onTag: (accountId: string, tag: string) => void;
}

export function TimelineView({ workspace, onPost, onProfile, onTag }: TimelineProps) {
  const scope = useBrowsingScope("timeline", workspace.accounts);
  const accounts = useMemo(() => workspace.accounts.filter(account => scope.selected.includes(account.id)), [workspace.accounts, scope.selected]);
  const feed = useSocialFeed(accounts, { kind: "HOME" });
  const [query, setQuery] = useState("");
  const [order, setOrder] = useState<SocialOrder>("LATEST");
  const [mediaOnly, setMediaOnly] = useState(false);
  const [hideReposts, setHideReposts] = useState(false);
  const posts = presentSocialPosts(feed.items, { query, order, mediaOnly, hideReposts });
  const failures = feed.failures.map(failure => ({ ...failure, provider: workspace.accounts.find(account => account.id === failure.accountId)?.provider ?? "BLUESKY" }));
  return <div className="unified-page"><ViewHeader icon={<Home />} title="Timeline" subtitle="One chronological stream across every selected account." controls={<SourceControls accounts={workspace.accounts} selected={scope.selected} onToggle={scope.toggle} onSelectProvider={scope.selectProvider} />} /><div className="unified-layout"><section className="feed-column"><SocialFeedToolbar query={query} order={order} mediaOnly={mediaOnly} hideReposts={hideReposts} onQuery={setQuery} onOrder={setOrder} onMediaOnly={setMediaOnly} onHideReposts={setHideReposts} /><FailureStrip failures={failures} accounts={workspace.accounts} retry={() => void feed.refresh()} />{feed.loading && feed.items.length === 0 ? <div className="post-skeletons" aria-label="Loading timeline"><i /><i /><i /></div> : <div className="row-list">{posts.map(item => <SocialPostCard key={item.post.canonicalKey} item={item} accounts={workspace.accounts} onPost={onPost} onProfile={onProfile} onTag={onTag} />)}</div>}{!feed.loading && posts.length === 0 && <div className="panel empty-state"><Home /><h3>No posts in this scope</h3><p>Select another account, change the loaded-post filters, or retry a failed source.</p></div>}{feed.hasMore && <button className="button load-more" disabled={feed.loading} onClick={() => void feed.more()}>{feed.loading ? "Loading…" : "Load more"}</button>}</section><aside className="details-column"><section className="panel"><h2><Filter size={17} /> Reading scope</h2><p className="aside-copy">Timeline combines provider home feeds, removes duplicate remote IDs, and keeps viewer state for every account.</p>{workspace.accounts.map(account => <label className="scope-account" key={account.id}><input type="checkbox" checked={scope.selected.includes(account.id)} onChange={() => scope.toggle(account.id)} /><span>{account.displayName}<small>{account.handle}</small></span></label>)}<div className="identity-note"><strong>Action identities</strong><p className="aside-copy">Each post names the account used for likes, reposts, replies, and follows. Reading scope stays separate from Composer destinations.</p></div></section></aside></div></div>;
}
