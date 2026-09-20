import { Layers3, Users } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useSocialFeed } from "../hooks/useSocialFeed";
import { socialApi } from "../services/desktop/social";
import { presentSocialPosts, type SocialOrder } from "../services/social/presentation";
import type { WorkspaceState } from "../types";
import type { ProfileDetails, ProfileFeedKind } from "../types/social";
import { ProviderIcon } from "./ui";
import { SocialFeedToolbar } from "./unified/SocialFeedToolbar";
import { SocialPostCard } from "./unified/SocialPostCard";
import { ViewHeader } from "./unified/ViewHeader";

interface ProfilesProps {
  readonly workspace: WorkspaceState;
  readonly accountId: string | null;
  readonly onAccount: (accountId: string | null) => void;
  readonly onAccounts: () => void;
  readonly onPost: (accountId: string, postId: string) => void;
  readonly onProfile: (accountId: string, profileId: string) => void;
  readonly onTag: (accountId: string, tag: string) => void;
}

const kinds: readonly { readonly value: ProfileFeedKind; readonly label: string }[] = [{ value: "POSTS", label: "Posts" }, { value: "REPLIES", label: "Replies" }, { value: "MEDIA", label: "Media" }];

export function MyProfilesView(props: ProfilesProps) {
  const account = props.workspace.accounts.find(item => item.id === props.accountId);
  const accounts = useMemo(() => account ? [account] : props.workspace.accounts, [account, props.workspace.accounts]);
  const [kind, setKind] = useState<ProfileFeedKind>("POSTS");
  const feed = useSocialFeed(accounts, { kind: "OWN", feedKind: kind });
  const [details, setDetails] = useState<ProfileDetails | null>(null);
  const [detailsError, setDetailsError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [order, setOrder] = useState<SocialOrder>("LATEST");
  const [mediaOnly, setMediaOnly] = useState(false);
  const [hideReposts, setHideReposts] = useState(false);
  useEffect(() => {
    let current = true;
    setDetails(null); setDetailsError(null);
    if (!account) return () => { current = false; };
    void socialApi.ownProfile(account.id).then(value => { if (current) setDetails(value); }).catch(cause => { if (current) setDetailsError(cause instanceof Error ? cause.message : String(cause)); });
    return () => { current = false; };
  }, [account]);
  const posts = presentSocialPosts(feed.items, { query, order, mediaOnly: mediaOnly || kind === "MEDIA", hideReposts });
  const controls = <div className="tabs profile-scope"><button className={`button ${account ? "" : "selected"}`} onClick={() => props.onAccount(null)}><Layers3 size={16} />Unified</button>{props.workspace.accounts.map(item => <button className={`button ${item.id === account?.id ? "selected" : ""}`} key={item.id} onClick={() => props.onAccount(item.id)}><ProviderIcon provider={item.provider} />{item.provider === "BLUESKY" ? "Bluesky" : "Mastodon"}</button>)}</div>;
  return <div className="unified-page"><ViewHeader icon={<Users />} title="My profiles" subtitle="Your public timeline on each connected account." controls={controls} /><div className="unified-layout"><section className="feed-column">{account ? <section className="panel profile-header"><div className="post-avatar">{details?.actor.avatarUrl ? <img src={details.actor.avatarUrl} alt="" /> : account.displayName[0]}</div><div className="profile-line"><div><h2>{details?.actor.displayName ?? account.displayName}</h2><span className="muted">@{details?.actor.handle ?? account.handle} · {account.provider === "BLUESKY" ? "Bluesky" : "Mastodon"}</span></div><span className={`provider-pill ${account.provider.toLowerCase()}`}>Your profile</span></div>{details?.description && <p>{details.description}</p>}{details && <div className="profile-stats"><span><strong>{details.followingCount?.toLocaleString() ?? "—"}</strong> Following</span><span><strong>{details.followersCount?.toLocaleString() ?? "—"}</strong> Followers</span><span><strong>{details.postsCount?.toLocaleString() ?? "—"}</strong> Posts</span></div>}{detailsError && <p className="inline-error" role="alert">{detailsError}</p>}</section> : <section className="panel profile-header"><h2>Your unified profile timeline</h2><p>Posts from all {accounts.length} connected {accounts.length === 1 ? "account" : "accounts"}, together in one timeline with provider attribution.</p></section>}<div className="tabs">{kinds.map(item => <button className={`button ${kind === item.value ? "selected" : ""}`} key={item.value} onClick={() => setKind(item.value)}>{item.label}</button>)}</div><SocialFeedToolbar query={query} order={order} mediaOnly={mediaOnly} hideReposts={hideReposts} onQuery={setQuery} onOrder={setOrder} onMediaOnly={setMediaOnly} onHideReposts={setHideReposts} />{feed.loading && feed.items.length === 0 ? <div className="post-skeletons" aria-label="Loading profile posts"><i /><i /></div> : <div className="row-list">{posts.map(item => <SocialPostCard key={item.post.canonicalKey} item={item} accounts={props.workspace.accounts} onPost={props.onPost} onProfile={(accountId) => props.onAccount(accountId)} onTag={props.onTag} />)}</div>}{!feed.loading && posts.length === 0 && <section className="panel empty-box">No posts match this profile view.</section>}{feed.hasMore && <button className="button load-more" disabled={feed.loading} onClick={() => void feed.more()}>{feed.loading ? "Loading…" : "Load more"}</button>}</section><aside className="details-column"><section className="panel"><h2>Connected profiles</h2>{props.workspace.accounts.map(item => <button className="actor-row click-row" key={item.id} onClick={() => props.onAccount(item.id)}><ProviderIcon provider={item.provider} /><span><strong>{item.displayName}</strong><small>@{item.handle}</small></span></button>)}</section><section className="panel"><h2>Your account</h2><p className="aside-copy section-space">{account ? `You are viewing only posts from ${account.displayName}'s ${account.provider === "BLUESKY" ? "Bluesky" : "Mastodon"} profile.` : "You are viewing posts from all connected profiles."} Home Timeline reading scope stays separate.</p><button className="button" onClick={props.onAccounts}>Manage accounts</button></section></aside></div></div>;
}
