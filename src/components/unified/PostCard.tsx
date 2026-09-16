import { ExternalLink, Heart, MessageCircle, Repeat2 } from "lucide-react";
import type { Account, UnifiedPost } from "../../types";
import { ProviderIcon } from "../ui";

export function PostCard({ post, accounts }: { post: UnifiedPost; accounts: readonly Account[] }) {
  const sourceNames = post.sources.map(source => accounts.find(account => account.id === source.accountId)?.displayName ?? source.accountHandle);
  return <article className="unified-post">
    <div className="post-avatar">{post.author.avatarUrl ? <img src={post.author.avatarUrl} alt="" /> : post.author.displayName[0]}</div>
    <div className="post-content"><header><strong>{post.author.displayName}</strong><span>@{post.author.handle.replace(/^@/, "")}</span><span className={`provider-pill ${post.provider.toLowerCase()}`}><ProviderIcon provider={post.provider} />{post.provider === "BLUESKY" ? "Bluesky" : "Mastodon"}</span><time dateTime={post.createdAt}>{new Intl.RelativeTimeFormat("en", { numeric: "auto" }).format(-Math.max(1, Math.round((Date.now() - Date.parse(post.createdAt)) / 3600000)), "hour")}</time></header>
      <p>{post.text}</p>{post.media.length > 0 && <div className="post-media">{post.media.map(media => <img key={media.url} src={media.url} alt={media.alt} />)}</div>}
      <footer>{post.metrics.replies !== undefined && <span><MessageCircle size={15} />{post.metrics.replies}</span>}{post.metrics.reposts !== undefined && <span><Repeat2 size={15} />{post.metrics.reposts}</span>}{post.metrics.likes !== undefined && <span><Heart size={15} />{post.metrics.likes}</span>}<a href={post.remoteUrl} target="_blank" rel="noreferrer" aria-label={`Open original post by ${post.author.displayName}`}><ExternalLink size={15} />Original</a><span className="via" title={sourceNames.join(", ")}>{post.sources.length > 1 ? `Also via ${post.sources.length} sources` : `Via ${sourceNames[0]}`}</span></footer>
    </div>
  </article>;
}
