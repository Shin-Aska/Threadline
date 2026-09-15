import { Eye, GitBranch, ShieldCheck } from "lucide-react";
import type { Account, MediaAttachment, PublishingPreview } from "../types";
import { Identity, ProviderIcon } from "./ui";
import { providerNames } from "./providers";
interface PreviewProps {
  readonly accounts: readonly Account[];
  readonly preview: PublishingPreview | null;
  readonly text: string;
  readonly media: readonly MediaAttachment[];
  readonly native: boolean;
}
export function DestinationPreview({ accounts, preview, text, media, native }: PreviewProps) {
  return <div className="preview-column"><section className="panel preview-panel" aria-labelledby="preview-heading"><div className="panel-heading"><h2 id="preview-heading"><Eye size={18} />Protocol Previews</h2><span className="badge">{preview ? "Native" : "Draft"}</span></div>
    {!accounts.length && <div className="empty-state"><Eye size={24} /><h3>Select a destination</h3><p>Select identities in the sidebar to preview your post.</p></div>}
    {(["BLUESKY", "MASTODON"] as const).map(provider => {
      const identities = accounts.filter(account => account.provider === provider);
      if (!identities.length) return null;
      const variants = new Map<string, { parts: readonly string[]; accounts: Account[]; planned: boolean }>();
      for (const account of identities) {
        const destination = preview?.destinations.find(item => item.accountId === account.id);
        const parts = destination?.parts ?? [text];
        const key = JSON.stringify([!!destination, parts]);
        const variant = variants.get(key) ?? { parts, accounts: [], planned: !!destination };
        variant.accounts.push(account); variants.set(key, variant);
      }
      return <section className={"protocol-group " + provider.toLowerCase()} key={provider} aria-label={providerNames[provider] + " previews"}>
        <div className="protocol-heading"><h3><ProviderIcon provider={provider} />{providerNames[provider]}</h3><span>{identities.length} {identities.length === 1 ? "account" : "accounts"} selected</span></div>
        {variants.size > 1 && <p className="variant-note">Different account limits produce {variants.size} thread plans.</p>}
        {[...variants.entries()].map(([key, variant]) => <div className="protocol-variant" key={key}>
          <div className="preview-identities">{variant.accounts.map(account => <div className="preview-identity" key={account.id} title={account.handle}><Identity account={account} /></div>)}</div>
          <article className="post-preview"><div className="thread-parts">{variant.parts.map((part, index) => <div className="thread-part" key={index}>{variant.parts.length > 1 && <span className="part-number">{index + 1}/{variant.parts.length}</span>}{(part || !media.length) && <p>{part || "Your next idea starts here."}</p>}{index === 0 && media.length > 0 && <div className="preview-images">{media.map((image, imageIndex) => <figure key={image.id}><img src={"data:" + image.mimeType + ";base64," + image.dataBase64} alt={image.altText || "Image " + (imageIndex + 1) + " (no description added)"} />{image.altText && <span className="badge" title={image.altText}>ALT</span>}</figure>)}</div>}</div>)}</div></article>
          <div className="protocol-status"><GitBranch size={13} />{variant.planned ? variant.parts.length + (variant.parts.length === 1 ? " post" : " thread parts") + " per account · Ready" : "Unsplit draft · Awaiting native plan"}</div>
        </div>)}
      </section>;
    })}
  </section><div className="vault-note"><ShieldCheck size={21} /><div><h3>{native ? "One source. Native threads." : "A preview of your next post"}</h3><p>{native ? "Threadline adapts your post to each account’s limits using the selected publishing policy." : "Thread splitting and publishing run in the desktop app."}</p></div></div></div>;
}
