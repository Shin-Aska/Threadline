import { Users } from "lucide-react";
import { useEffect, useState } from "react";
import { useSocialFeed } from "../../hooks/useSocialFeed";
import { socialApi } from "../../services/desktop/social";
import { presentSocialPosts, type SocialOrder } from "../../services/social/presentation";
import type { WorkspaceState } from "../../types";
import type { ProfileDetails, ProfileFeedKind } from "../../types/social";
import { Notice } from "../ui";
import { SocialFeedToolbar } from "../unified/SocialFeedToolbar";
import { SocialPostCard } from "../unified/SocialPostCard";
import { ViewHeader } from "../unified/ViewHeader";
import { DetailHeader } from "./DetailHeader";
import type { DetailTarget } from "./SocialDetailView";

interface ProfileProps { readonly target: Extract<DetailTarget, { readonly kind: "PROFILE" }>; readonly workspace: WorkspaceState; readonly onBack: () => void; readonly onPost: (accountId: string, postId: string) => void; readonly onProfile: (accountId: string, profileId: string) => void; readonly onTag: (accountId: string, tag: string) => void }
const kinds: readonly ProfileFeedKind[] = ["POSTS", "REPLIES", "MEDIA"];
export function ProfileDetail(props: ProfileProps) {
  const account = props.workspace.accounts.find(account => account.id === props.target.accountId);
  const [kind, setKind] = useState<ProfileFeedKind>("POSTS");
  const [details, setDetails] = useState<ProfileDetails | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const [query, setQuery] = useState(""); const [order, setOrder] = useState<SocialOrder>("LATEST"); const [mediaOnly, setMediaOnly] = useState(false); const [hideReposts, setHideReposts] = useState(false);
  const feed = useSocialFeed(account ? [account] : [], { kind: "PROFILE", profileId: props.target.id, feedKind: kind });
  useEffect(() => { let current = true; setDetails(null); setError(null); void socialApi.profile(props.target.accountId, props.target.id).then(value => { if (current) setDetails(value); }).catch(cause => { if (current) setError(cause instanceof Error ? cause.message : String(cause)); }); return () => { current = false; }; }, [props.target.accountId, props.target.id]);
  const toggleFollow = async () => {
    if (!details || pending) return;
    setPending(true); setError(null);
    try {
      const result = await socialApi.act(props.target.accountId, details.followedByMe ? { kind: "UNFOLLOW", profileId: details.actor.id } : { kind: "FOLLOW", profileId: details.actor.id });
      const followed = result.followed;
      if (followed !== null) setDetails(current => current ? { ...current, followedByMe: followed, followUri: result.recordId } : current);
    } catch (cause) { setError(cause instanceof Error ? cause.message : String(cause)); }
    finally { setPending(false); }
  };
  const posts = presentSocialPosts(feed.items, { query, order, mediaOnly: mediaOnly || kind === "MEDIA", hideReposts });
  return <div className="unified-page"><DetailHeader origin={props.target.origin} label={details?.actor.displayName ?? "Profile"} onBack={props.onBack} /><ViewHeader icon={<Users />} title="Profile" subtitle={`An account on ${account?.provider === "BLUESKY" ? "Bluesky" : "Mastodon"}.`} controls={<span />} /><div className="unified-layout"><section className="feed-column"><section className="panel profile-header"><div className="post-avatar">{details?.actor.avatarUrl ? <img src={details.actor.avatarUrl} alt="" /> : details?.actor.displayName[0] ?? "?"}</div><div className="profile-line"><div><h1>{details?.actor.displayName ?? "Loading profile…"}</h1>{details && <span className="muted">@{details.actor.handle}</span>}</div>{details && <button className={`button ${details.followedByMe ? "" : "button-blue"}`} disabled={pending} onClick={() => void toggleFollow()}>{pending ? "Updating…" : details.followedByMe ? `Following as ${account?.displayName ?? "selected account"}` : `Follow as ${account?.displayName ?? "selected account"}`}</button>}</div>{details?.description && <p>{details.description}</p>}{details && <div className="profile-stats"><span><strong>{details.followingCount?.toLocaleString() ?? "—"}</strong> Following</span><span><strong>{details.followersCount?.toLocaleString() ?? "—"}</strong> Followers</span><span><strong>{details.postsCount?.toLocaleString() ?? "—"}</strong> Posts</span></div>}{error && <Notice error>{error}</Notice>}</section><div className="tabs">{kinds.map(value => <button className={`button ${kind === value ? "selected" : ""}`} key={value} onClick={() => setKind(value)}>{value[0] + value.slice(1).toLocaleLowerCase()}</button>)}</div><SocialFeedToolbar query={query} order={order} mediaOnly={mediaOnly} hideReposts={hideReposts} onQuery={setQuery} onOrder={setOrder} onMediaOnly={setMediaOnly} onHideReposts={setHideReposts} /><div className="row-list">{posts.map(item => <SocialPostCard key={item.post.canonicalKey} item={item} accounts={props.workspace.accounts} onPost={props.onPost} onProfile={props.onProfile} onTag={props.onTag} />)}</div>{feed.hasMore && <button className="button load-more" onClick={() => void feed.more()}>Load more</button>}</section><aside className="details-column"><section className="panel"><h2>Action identity</h2><p className="aside-copy">Profile actions use {account?.displayName ?? "the selected account"} on {account?.provider === "BLUESKY" ? "Bluesky" : "Mastodon"}.</p></section></aside></div></div>;
}
