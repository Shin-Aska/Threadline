import { ExternalLink as ExternalLinkIcon, Heart, MessageCircle, Repeat2 } from "lucide-react";
import { useState } from "react";
import type { Account, UnifiedPost } from "../../types";
import { ProviderIcon } from "../ui";
import { ExternalLink, LinkifiedText } from "./ExternalLinks";
import { ImageViewer } from "./ImageViewer";

export function PostCard({ post, accounts, onPost, onProfile }: { post: UnifiedPost; accounts: readonly Account[]; readonly onPost?: (accountId: string, postId: string) => void; readonly onProfile?: (accountId: string, profileId: string) => void }) {
  const [selectedImage, setSelectedImage] = useState<{ readonly url: string; readonly alt: string } | null>(null);
  const sourceNames = post.sources.map(source => accounts.find(account => account.id === source.accountId)?.displayName ?? source.accountHandle);
  const accountId = post.sources[0]?.accountId;
  const openConversation = () => { if (onPost && accountId) onPost(accountId, post.remoteId); };
  return <><article className={`unified-post ${onPost && accountId ? "post-openable" : ""}`} onClick={event => { if (event.target instanceof Element && !event.target.closest("button, a, select, input, textarea, video") && !window.getSelection()?.toString()) openConversation(); }}>
    {onPost && accountId && <button type="button" className="post-surface-action" aria-label={`Open conversation by ${post.author.displayName}`} onClick={openConversation} />}
    <div className="post-avatar">{post.author.avatarUrl ? <img src={post.author.avatarUrl} alt="" /> : post.author.displayName[0]}</div>
    <div className="post-content"><header>{onProfile && accountId ? <button className="author-link quiet" onClick={() => onProfile(accountId, post.author.id)}>{post.author.displayName}</button> : <strong>{post.author.displayName}</strong>}<span>@{post.author.handle.replace(/^@/, "")}</span><span className={`provider-pill ${post.provider.toLowerCase()}`}><ProviderIcon provider={post.provider} />{post.provider === "BLUESKY" ? "Bluesky" : "Mastodon"}</span><time dateTime={post.createdAt}>{onPost && accountId ? <button className="quiet" aria-label={`Open conversation with ${post.author.displayName}`} onClick={openConversation}>{new Intl.RelativeTimeFormat("en", { numeric: "auto" }).format(-Math.max(1, Math.round((Date.now() - Date.parse(post.createdAt)) / 3600000)), "hour")}</button> : new Intl.RelativeTimeFormat("en", { numeric: "auto" }).format(-Math.max(1, Math.round((Date.now() - Date.parse(post.createdAt)) / 3600000)), "hour")}</time></header>
      <p><LinkifiedText text={post.text} /></p>{post.media.length > 0 && <div className="post-media">{post.media.map(media => media.type === "video" || media.type === "gifv" ? <video key={media.url} src={media.url} controls preload="metadata" aria-label={media.alt || "Post video"} /> : <button type="button" key={media.url} aria-label={`View image: ${media.alt || "Post image"}`} onClick={() => setSelectedImage({ url: media.url, alt: media.alt })}><img src={media.url} alt={media.alt} /></button>)}</div>}
      <footer>{post.metrics.replies !== undefined && <span><MessageCircle size={15} />{post.metrics.replies}</span>}{post.metrics.reposts !== undefined && <span><Repeat2 size={15} />{post.metrics.reposts}</span>}{post.metrics.likes !== undefined && <span><Heart size={15} />{post.metrics.likes}</span>}<ExternalLink href={post.remoteUrl} ariaLabel={`Open original post by ${post.author.displayName}`}><ExternalLinkIcon size={15} />Original</ExternalLink><span className="via" title={sourceNames.join(", ")}>{post.sources.length > 1 ? `Also via ${post.sources.length} sources` : `Via ${sourceNames[0]}`}</span></footer>
    </div>
  </article>{selectedImage && <ImageViewer src={selectedImage.url} alt={selectedImage.alt} onClose={() => setSelectedImage(null)} />}</>;
}
