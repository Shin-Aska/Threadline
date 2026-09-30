import { ExternalLink as ExternalLinkIcon, Heart, MessageCircle, Repeat2 } from "lucide-react";
import { useEffect, useState } from "react";
import { socialApi, subscribeSocialActions } from "../../services/desktop/social";
import type { SocialFeedItem } from "../../services/social/presentation";
import type { Account } from "../../types";
import type { ViewerState } from "../../types/social";
import { ProviderIcon } from "../ui";
import { ExternalLink, LinkifiedText } from "./ExternalLinks";
import { ImageViewer } from "./ImageViewer";
import { PostVisibility } from "./PostVisibility";
import { PostVideo } from "./PostVideo";

interface SocialPostProps {
  readonly item: SocialFeedItem;
  readonly accounts: readonly Account[];
  readonly onPost: (accountId: string, postId: string) => void;
  readonly onProfile: (accountId: string, profileId: string) => void;
  readonly onTag: (accountId: string, tag: string) => void;
}

/** Renders a social post with per-account reactions and provider actions. */
export function SocialPostCard({ item, accounts, onPost, onProfile, onTag }: SocialPostProps) {
  const firstAccount = item.observations[0]?.accountId ?? "";
  const [accountId, setAccountId] = useState(firstAccount);
  const [viewerByAccount, setViewerByAccount] = useState<Readonly<Record<string, ViewerState>>>({});
  const [metrics, setMetrics] = useState(item.post.metrics);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedImage, setSelectedImage] = useState<{ readonly url: string; readonly alt: string } | null>(null);
  const [revealed, setRevealed] = useState(false);
  useEffect(() => { setAccountId(firstAccount); }, [item.post.canonicalKey, firstAccount]);
  useEffect(() => { setMetrics(item.post.metrics); }, [item.post.metrics]);
  useEffect(() => subscribeSocialActions((actingAccountId, action, result) => {
    if (!("postId" in action) || action.postId !== item.post.remoteId || !item.observations.some(observation => observation.accountId === actingAccountId)) return;
    if (result.viewer) setViewerByAccount(current => ({ ...current, [actingAccountId]: result.viewer ?? current[actingAccountId] }));
    setMetrics(current => {
      switch (action.kind) {
        case "LIKE": return { ...current, likes: current.likes === null ? null : current.likes + 1 };
        case "UNLIKE": return { ...current, likes: current.likes === null ? null : Math.max(0, current.likes - 1) };
        case "REPOST": return { ...current, reposts: current.reposts === null ? null : current.reposts + 1 };
        case "UNDO_REPOST": return { ...current, reposts: current.reposts === null ? null : Math.max(0, current.reposts - 1) };
        case "REPLY": return { ...current, replies: current.replies === null ? null : current.replies + 1 };
      }
    });
  }), [item.post.remoteId, item.observations]);
  const viewer = viewerByAccount[accountId] ?? item.observations.find(observation => observation.accountId === accountId)?.viewer ?? item.post.viewer;
  const act = async (kind: "LIKE" | "REPOST") => {
    if (!accountId || pending) return;
    setPending(true); setError(null);
    try {
      const action = kind === "LIKE"
        ? viewer.liked ? { kind: "UNLIKE", postId: item.post.remoteId } as const : { kind: "LIKE", postId: item.post.remoteId } as const
        : viewer.reposted ? { kind: "UNDO_REPOST", postId: item.post.remoteId } as const : { kind: "REPOST", postId: item.post.remoteId } as const;
      await socialApi.act(accountId, action);
    } catch (cause) { setError(cause instanceof Error ? cause.message : String(cause)); }
    finally { setPending(false); }
  };
  const acting = accounts.find(account => account.id === accountId);
  const relative = new Intl.RelativeTimeFormat("en", { numeric: "auto" }).format(-Math.max(1, Math.round((Date.now() - Date.parse(item.post.createdAt)) / 3600000)), "hour");
  const openConversation = () => onPost(accountId, item.post.remoteId);
  const hasWarning = Boolean(item.post.contentWarning?.trim());
  const showText = !hasWarning || revealed;
  const showMedia = !(hasWarning || item.post.sensitive) || revealed;
  return <>
    <article className="unified-post post-openable" onClick={event => { if (event.target instanceof Element && !event.target.closest("button, a, select, input, textarea, video") && !window.getSelection()?.toString()) openConversation(); }}>
      <button type="button" className="post-surface-action" aria-label={`Open conversation by ${item.post.author.displayName}`} onClick={openConversation} />
      <button className="post-avatar quiet" aria-label={`View ${item.post.author.displayName}'s profile`} onClick={() => onProfile(accountId, item.post.author.id)}>{item.post.author.avatarUrl ? <img src={item.post.author.avatarUrl} alt="" /> : item.post.author.displayName[0]}</button>
      <div className="post-content">
        <header><button className="author-link quiet" onClick={() => onProfile(accountId, item.post.author.id)}>{item.post.author.displayName}</button><span>@{item.post.author.handle.replace(/^@/, "")}</span><span className={`provider-pill ${item.post.provider.toLowerCase()}`}><ProviderIcon provider={item.post.provider} />{item.post.provider === "BLUESKY" ? "Bluesky" : "Mastodon"}</span><time dateTime={item.post.createdAt}><button className="quiet" onClick={openConversation}>{relative}</button></time></header>
        <PostVisibility warning={item.post.contentWarning} sensitive={item.post.sensitive} hasMedia={item.post.media.length > 0} revealed={revealed} onToggle={() => setRevealed(value => !value)} />
        {showText && <p><LinkifiedText text={item.post.text} onTag={tag => onTag(accountId, tag)} /></p>}
        {showMedia && item.post.media.length > 0 && <div className="post-media">{item.post.media.map(media => media.mediaType.startsWith("video") || media.mediaType === "gifv" ? <PostVideo key={media.url} url={media.url} alt={media.alt} thumbnail={media.thumbnail} mediaType={media.mediaType} originalUrl={item.post.remoteUrl} /> : <button type="button" key={media.url} aria-label={`View image: ${media.alt || "Post image"}`} onClick={() => setSelectedImage({ url: media.url, alt: media.alt })}><img src={media.url} alt={media.alt} /></button>)}</div>}
        <div className="action-identity"><label>Act as <select value={accountId} onChange={event => setAccountId(event.target.value)}>{item.observations.map(observation => { const account = accounts.find(account => account.id === observation.accountId); return <option value={observation.accountId} key={observation.accountId}>{account?.displayName ?? observation.accountId} · {account?.provider === "BLUESKY" ? "Bluesky" : "Mastodon"}</option>; })}</select></label></div>
        <footer><button aria-label={`Open conversation with ${item.post.author.displayName}`} onClick={openConversation}><MessageCircle size={15} />{metrics.replies ?? "—"}</button><button disabled={pending} className={viewer.reposted ? "on" : ""} aria-pressed={viewer.reposted} onClick={() => void act("REPOST")}><Repeat2 size={15} />{metrics.reposts ?? "—"}</button><button disabled={pending} className={viewer.liked ? "on" : ""} aria-pressed={viewer.liked} onClick={() => void act("LIKE")}><Heart size={15} />{metrics.likes ?? "—"}</button><ExternalLink href={item.post.remoteUrl}><ExternalLinkIcon size={15} />Original</ExternalLink><span className="via">Via {acting?.displayName ?? "selected account"} · {item.post.provider === "BLUESKY" ? "Bluesky" : "Mastodon"}</span></footer>
        {error && <p className="inline-error" role="alert">{error}</p>}
      </div>
    </article>
    {selectedImage && <ImageViewer src={selectedImage.url} alt={selectedImage.alt} onClose={() => setSelectedImage(null)} />}
  </>;
}
