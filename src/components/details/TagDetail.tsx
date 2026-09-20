import { Hash } from "lucide-react";
import { useState } from "react";
import { useSocialFeed } from "../../hooks/useSocialFeed";
import { presentSocialPosts, type SocialOrder } from "../../services/social/presentation";
import type { WorkspaceState } from "../../types";
import { SocialFeedToolbar } from "../unified/SocialFeedToolbar";
import { SocialPostCard } from "../unified/SocialPostCard";
import { ViewHeader } from "../unified/ViewHeader";
import type { DetailTarget } from "./SocialDetailView";
import { DetailHeader } from "./DetailHeader";

interface TagProps { readonly target: Extract<DetailTarget, { readonly kind: "TAG" }>; readonly workspace: WorkspaceState; readonly onBack: () => void; readonly onPost: (accountId: string, postId: string) => void; readonly onProfile: (accountId: string, profileId: string) => void; readonly onTag: (accountId: string, tag: string) => void }
export function TagDetail(props: TagProps) {
  const account = props.workspace.accounts.find(account => account.id === props.target.accountId);
  const feed = useSocialFeed(account ? [account] : [], { kind: "TAG", tag: props.target.id });
  const [query, setQuery] = useState(""); const [order, setOrder] = useState<SocialOrder>("LATEST"); const [mediaOnly, setMediaOnly] = useState(false); const [hideReposts, setHideReposts] = useState(false);
  const posts = presentSocialPosts(feed.items, { query, order, mediaOnly, hideReposts });
  return <div className="unified-page"><DetailHeader origin={props.target.origin} label={`#${props.target.id}`} onBack={props.onBack} /><ViewHeader icon={<Hash />} title={`#${props.target.id}`} subtitle={`Posts matching this tag via ${account?.displayName ?? "the selected account"}.`} controls={<span />} /><div className="unified-layout"><section className="feed-column"><SocialFeedToolbar query={query} order={order} mediaOnly={mediaOnly} hideReposts={hideReposts} onQuery={setQuery} onOrder={setOrder} onMediaOnly={setMediaOnly} onHideReposts={setHideReposts} /><div className="row-list">{posts.map(item => <SocialPostCard key={item.post.canonicalKey} item={item} accounts={props.workspace.accounts} onPost={props.onPost} onProfile={props.onProfile} onTag={props.onTag} />)}</div>{!feed.loading && posts.length === 0 && <section className="panel empty-box">No provider posts were returned for this tag.</section>}{feed.hasMore && <button className="button load-more" onClick={() => void feed.more()}>Load more</button>}</section><aside className="details-column"><section className="panel"><h2>Topic details</h2><div className="detail-topic">#{props.target.id}</div><p className="aside-copy">Activity comes from the selected account’s provider. Counts are source observations, not combined reach.</p></section></aside></div></div>;
}
