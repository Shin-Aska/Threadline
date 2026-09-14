import { Eye, GitBranch, ShieldCheck } from "lucide-react";
import type { Account, PublishingPreview } from "../types";
import { Identity } from "./ui";
import { protocolNames, providerNames } from "./providers";

interface PreviewProps {
  readonly accounts: readonly Account[];
  readonly preview: PublishingPreview | null;
  readonly text: string;
  readonly native: boolean;
}

export function DestinationPreview({ accounts, preview, text, native }: PreviewProps) {
  return <div className="preview-column"><section className="panel preview-panel" aria-labelledby="preview-heading"><div className="panel-heading"><h2 id="preview-heading"><Eye size={18} />Protocol Previews</h2><span className="badge">{preview ? "Native" : "Draft"}</span></div>
    {!accounts.length && <div className="empty-state"><Eye size={24} /><h3>Select a destination</h3><p>Your post will appear here for each selected account.</p></div>}
    {accounts.map(account => {
      const destination = preview?.destinations.find(item => item.accountId === account.id);
      const parts = destination?.parts ?? [text];
      return <div className={`destination-preview ${account.provider.toLowerCase()}`} key={account.id}>
        <div className="preview-label"><span><span className="status-dot" />{providerNames[account.provider]} preview</span><span>{account.capabilities.maxTextLength.toLocaleString()} max</span></div>
        <article className="post-preview"><Identity account={account} /><div className="post-meta">{destination ? `${parts.length} ${parts.length === 1 ? "post" : "parts"}` : "Unsplit draft"} · {protocolNames[account.provider]}</div><div className="thread-parts">{parts.map((part, index) => <div className="thread-part" key={index}>{parts.length > 1 && <span className="part-number">{index + 1}</span>}<p>{part || "Your next idea starts here."}</p></div>)}</div><div className="post-preview-footer"><GitBranch size={13} /><span>{destination ? "Planned by Threadline" : "Final thread plan available in desktop"}</span></div></article>
      </div>;
    })}
  </section><div className="vault-note"><ShieldCheck size={21} /><div><h3>{native ? "One source. Native threads." : "A preview of your next post"}</h3><p>{native ? "Threadline adapts your post to each account’s limits using the selected publishing policy." : "Draft text is shown here. Thread splitting and publishing run in the desktop app."}</p></div></div></div>;
}
