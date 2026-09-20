import { ExternalLink, Heart, MessageCircle, Repeat2 } from "lucide-react";
import { useEffect, useState } from "react";
import { socialApi } from "../../services/desktop/social";
import type { SocialFeedItem } from "../../services/social/presentation";
import type { Account } from "../../types";
import type { ViewerState } from "../../types/social";
import { ProviderIcon } from "../ui";

interface SocialPostProps {
  readonly item: SocialFeedItem;
  readonly accounts: readonly Account[];
  readonly onPost: (accountId: string, postId: string) => void;
  readonly onProfile: (accountId: string, profileId: string) => void;
  readonly onTag: (accountId: string, tag: string) => void;
}

export function SocialPostCard({ item, accounts, onPost, onProfile, onTag }: SocialPostProps) {
  const firstAccount = item.observations[0]?.accountId ?? "";
  const [accountId, setAccountId] = useState(firstAccount);
  const [viewerByAccount, setViewerByAccount] = useState<Readonly<Record<string, ViewerState>>>(() => Object.fromEntries(item.observations.map(item => [item.accountId, item.viewer])));
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => { setAccountId(firstAccount); setViewerByAccount(Object.fromEntries(item.observations.map(item => [item.accountId, item.viewer]))); }, [item.post.canonicalKey, firstAccount, item.observations]);
  const viewer = viewerByAccount[accountId] ?? item.post.viewer;
  const act = async (kind: "LIKE" | "REPOST") => {
    if (!accountId || pending) return;
    setPending(true); setError(null);
    try {
      const action = kind === "LIKE"
        ? viewer.liked ? { kind: "UNLIKE", postId: item.post.remoteId } as const : { kind: "LIKE", postId: item.post.remoteId } as const
        : viewer.reposted ? { kind: "UNDO_REPOST", postId: item.post.remoteId } as const : { kind: "REPOST", postId: item.post.remoteId } as const;
      const result = await socialApi.act(accountId, action);
      if (result.viewer) setViewerByAccount(current => ({ ...current, [accountId]: result.viewer ?? viewer }));
    } catch (cause) { setError(cause instanceof Error ? cause.message : String(cause)); }
    finally { setPending(false); }
  };
  const acting = accounts.find(account => account.id === accountId);
  const relative = new Intl.RelativeTimeFormat("en", { numeric: "auto" }).format(-Math.max(1, Math.round((Date.now() - Date.parse(item.post.createdAt)) / 3600000)), "hour");
  const text = item.post.text.split(/(#[\p{L}\p{N}_]+)/u);
  return <article className="unified-post"><button className="post-avatar quiet" aria-label={`View ${item.post.author.displayName}'s profile`} onClick={() => onProfile(accountId, item.post.author.id)}>{item.post.author.avatarUrl ? <img src={item.post.author.avatarUrl} alt="" /> : item.post.author.displayName[0]}</button><div className="post-content"><header><button className="author-link quiet" onClick={() => onProfile(accountId, item.post.author.id)}>{item.post.author.displayName}</button><span>@{item.post.author.handle.replace(/^@/, "")}</span><span className={`provider-pill ${item.post.provider.toLowerCase()}`}><ProviderIcon provider={item.post.provider} />{item.post.provider === "BLUESKY" ? "Bluesky" : "Mastodon"}</span><time dateTime={item.post.createdAt}><button className="quiet" onClick={() => onPost(accountId, item.post.remoteId)}>{relative}</button></time></header><p>{text.map((part, index) => part.startsWith("#") ? <button className="link quiet" key={`${part}-${index}`} onClick={() => onTag(accountId, part.slice(1))}>{part}</button> : part)}</p>{item.post.media.length > 0 && <div className="post-media">{item.post.media.map(media => media.mediaType.startsWith("video") || media.mediaType === "gifv" ? <video key={media.url} src={media.url} controls preload="metadata" aria-label={media.alt || "Post video"} /> : <img key={media.url} src={media.url} alt={media.alt} />)}</div>}<div className="action-identity"><label>Act as <select value={accountId} onChange={event => setAccountId(event.target.value)}>{item.observations.map(observation => { const account = accounts.find(account => account.id === observation.accountId); return <option value={observation.accountId} key={observation.accountId}>{account?.displayName ?? observation.accountId} · {account?.provider === "BLUESKY" ? "Bluesky" : "Mastodon"}</option>; })}</select></label></div><footer><button onClick={() => onPost(accountId, item.post.remoteId)}><MessageCircle size={15} />{item.post.metrics.replies ?? "—"}</button><button disabled={pending} className={viewer.reposted ? "on" : ""} aria-pressed={viewer.reposted} onClick={() => void act("REPOST")}><Repeat2 size={15} />{item.post.metrics.reposts ?? "—"}</button><button disabled={pending} className={viewer.liked ? "on" : ""} aria-pressed={viewer.liked} onClick={() => void act("LIKE")}><Heart size={15} />{item.post.metrics.likes ?? "—"}</button><a href={item.post.remoteUrl} target="_blank" rel="noreferrer"><ExternalLink size={15} />Original</a><span className="via">Via {acting?.displayName ?? "selected account"} · {item.post.provider === "BLUESKY" ? "Bluesky" : "Mastodon"}</span></footer>{error && <p className="inline-error" role="alert">{error}</p>}</div></article>;
}
