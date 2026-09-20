import { Layers3 } from "lucide-react";
import { useState } from "react";
import { useSocialFeed } from "../../hooks/useSocialFeed";
import { presentSocialPosts, type SocialOrder } from "../../services/social/presentation";
import type { WorkspaceState } from "../../types";
import { SocialFeedToolbar } from "../unified/SocialFeedToolbar";
import { SocialPostCard } from "../unified/SocialPostCard";
import { ViewHeader } from "../unified/ViewHeader";
import { DetailHeader } from "./DetailHeader";
import type { DetailTarget } from "./SocialDetailView";

interface SourceProps { readonly target: Extract<DetailTarget, { readonly kind: "SOURCE" }>; readonly workspace: WorkspaceState; readonly onBack: () => void; readonly onPost: (accountId: string, postId: string) => void; readonly onProfile: (accountId: string, profileId: string) => void; readonly onTag: (accountId: string, tag: string) => void }
export function SourceDetail(props: SourceProps) {
  const account = props.workspace.accounts.find(account => account.id === props.target.accountId);
  const feed = useSocialFeed(account ? [account] : [], { kind: "SOURCE", source: props.target.source });
  const [query, setQuery] = useState(""); const [order, setOrder] = useState<SocialOrder>("LATEST"); const [mediaOnly, setMediaOnly] = useState(false); const [hideReposts, setHideReposts] = useState(false);
  const posts = presentSocialPosts(feed.items, { query, order, mediaOnly, hideReposts });
  return <div className="unified-page"><DetailHeader origin={props.target.origin} label={props.target.source.title} onBack={props.onBack} /><ViewHeader icon={<Layers3 />} title={props.target.source.title} subtitle={props.target.source.description ?? `Posts from this ${props.target.source.sourceType.toLocaleLowerCase()}.`} controls={<span />} /><div className="unified-layout"><section className="feed-column"><SocialFeedToolbar query={query} order={order} mediaOnly={mediaOnly} hideReposts={hideReposts} onQuery={setQuery} onOrder={setOrder} onMediaOnly={setMediaOnly} onHideReposts={setHideReposts} /><div className="row-list">{posts.map(item => <SocialPostCard key={item.post.canonicalKey} item={item} accounts={props.workspace.accounts} onPost={props.onPost} onProfile={props.onProfile} onTag={props.onTag} />)}</div>{!feed.loading && posts.length === 0 && <section className="panel empty-box">No posts were returned from this source.</section>}{feed.hasMore && <button className="button load-more" onClick={() => void feed.more()}>Load more</button>}</section><aside className="details-column"><section className="panel"><h2>Source details</h2><div className="detail-topic">{props.target.source.title}</div><p className="aside-copy">{props.target.source.description ?? "No provider description is available."}</p><small className="muted">{props.target.source.sourceType} · {account?.handle}</small></section></aside></div></div>;
}
