import { Hash, MessageCircle, Send } from "lucide-react";
import { useEffect, useState } from "react";
import { socialApi } from "../../services/desktop/social";
import type { SocialFeedItem } from "../../services/social/presentation";
import type { WorkspaceState } from "../../types";
import type { SocialPost, ThreadView } from "../../types/social";
import { Notice } from "../ui";
import { SocialPostCard } from "../unified/SocialPostCard";
import { ViewHeader } from "../unified/ViewHeader";
import { DetailHeader } from "./DetailHeader";
import type { DetailTarget } from "./SocialDetailView";

interface ThreadProps { readonly target: Extract<DetailTarget, { readonly kind: "POST" }>; readonly workspace: WorkspaceState; readonly onBack: () => void; readonly onPost: (accountId: string, postId: string) => void; readonly onProfile: (accountId: string, profileId: string) => void; readonly onTag: (accountId: string, tag: string) => void }
export function ThreadDetail(props: ThreadProps) {
  const [thread, setThread] = useState<ThreadView | null>(null);
  const [reply, setReply] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const account = props.workspace.accounts.find(account => account.id === props.target.accountId);
  useEffect(() => { let current = true; setThread(null); setError(null); void socialApi.thread(props.target.accountId, props.target.id).then(value => { if (current) setThread(value); }).catch(cause => { if (current) setError(cause instanceof Error ? cause.message : String(cause)); }); return () => { current = false; }; }, [props.target.accountId, props.target.id]);
  const item = (post: SocialPost): SocialFeedItem => ({ post, observations: [{ accountId: props.target.accountId, viewer: post.viewer }] });
  const send = async () => {
    const text = reply.trim();
    if (!thread || !text || pending) return;
    setPending(true); setError(null);
    try {
      const result = await socialApi.act(props.target.accountId, { kind: "REPLY", postId: thread.post.remoteId, text });
      const createdPost = result.createdPost;
      if (createdPost) setThread(current => current ? { ...current, replies: [...current.replies, createdPost] } : current);
      setReply("");
    } catch (cause) { setError(cause instanceof Error ? cause.message : String(cause)); }
    finally { setPending(false); }
  };
  const card = (post: SocialPost) => <SocialPostCard key={post.canonicalKey} item={item(post)} accounts={props.workspace.accounts} onPost={props.onPost} onProfile={props.onProfile} onTag={props.onTag} />;
  return <div className="unified-page"><DetailHeader origin={props.target.origin} label="Conversation" onBack={props.onBack} /><ViewHeader icon={<MessageCircle />} title="Conversation" subtitle={`Post and replies as ${account?.displayName ?? "the selected account"}.`} controls={<span />} /><div className="unified-layout"><section className="feed-column">{error && <Notice error>{error}</Notice>}{!thread && !error && <div className="post-skeletons" aria-label="Loading conversation"><i /><i /></div>}{thread && <>{thread.ancestors.map(card)}{card(thread.post)}<h2 className="section-space">Replies</h2><div className="reply-list">{thread.replies.map(card)}{thread.replies.length === 0 && <p className="muted">No replies returned yet.</p>}</div><section className="panel reply-box"><div><h2>Write a reply</h2><span className="muted">As {account?.displayName} · {account?.provider === "BLUESKY" ? "Bluesky" : "Mastodon"}</span></div><label className="sr-only" htmlFor="thread-reply">Your reply</label><textarea id="thread-reply" value={reply} onChange={event => setReply(event.target.value)} placeholder="Join the conversation…" /><div><small className="muted">This sends through the selected provider account.</small><button className="button button-blue" disabled={pending || !reply.trim()} onClick={() => void send()}><Send size={15} />{pending ? "Replying…" : "Reply"}</button></div></section></>}</section><aside className="details-column">{thread && <><section className="panel"><h2>About the author</h2><button className="actor-row click-row" onClick={() => props.onProfile(props.target.accountId, thread.post.author.id)}><span className="mini-avatar">{thread.post.author.displayName[0]}</span><span><strong>{thread.post.author.displayName}</strong><small>@{thread.post.author.handle}</small></span></button></section><section className="panel"><h2><Hash size={16} /> Explore topics</h2><p className="aside-copy">Open a hashtag from the post text to browse its provider feed.</p></section></>}</aside></div></div>;
}
