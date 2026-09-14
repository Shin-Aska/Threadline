import { useMemo, useRef, useState } from "react";
import { Check, CircleAlert, Feather, GitBranch, Link2, Send, Type } from "lucide-react";
import { desktopApi } from "../services/desktop";
import { usePostPreview } from "../hooks/usePostPreview";
import { useComposerStore } from "../stores/composer";
import type { CanonicalPost, PublishResult, WorkspaceState } from "../types";
import { DestinationPreview } from "./DestinationPreview";
import { LimitMonitor } from "./LimitMonitor";
import { Notice, ProviderIcon } from "./ui";
import { protocolNames, providerNames } from "./providers";

const policies = [
  { value: "ADAPTIVE", label: "Adaptive", detail: "Thread only where needed" },
  { value: "COMMON_LIMIT", label: "Common limit", detail: "Use the shortest account limit" },
  { value: "ALWAYS_THREAD", label: "Always thread", detail: "Number every destination’s posts" },
] as const;
interface ComposerProps {
  readonly workspace: WorkspaceState;
  readonly refreshing: boolean;
  readonly onAccounts: () => void;
}

export function ComposerPanel({ workspace, refreshing, onAccounts }: ComposerProps) {
  const { accounts, mode, connectedAccountIds } = workspace;
  const native = mode !== "BROWSER";
  const { text, policy, selected, setText, setPolicy, toggle, setSelected } = useComposerStore();
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<PublishResult | null>(null);
  const [publishing, setPublishing] = useState(false);
  const publishPending = useRef(false);
  const succeeded = !!result?.publications.length && result.publications.every(publication => publication.status === "PUBLISHED");
  const receivedPosts = result?.publications.some(publication => publication.status === "PUBLISHED" || publication.remotePostIds.length > 0) ?? false;
  const feedback = succeeded ? "Published successfully. Ready for your next post." : receivedPosts ? "Some posts were published. Review results before retrying." : result || error ? "Publishing failed. Your draft was kept." : null;
  const destinations = accounts.filter(account => selected.includes(account.id));
  const post = useMemo<CanonicalPost>(() => ({ text, media: [], policy, destinationAccountIds: selected }), [text, policy, selected]);
  const { preview, error: previewError, planning, retry } = usePostPreview(post, workspace, native && !refreshing && selected.length > 0 && !!text.trim());
  const disconnected = destinations.some(account => !connectedAccountIds.includes(account.id));
  const canPublish = native && !!preview && !disconnected && !refreshing && !publishing;
  const count = [...new Intl.Segmenter(undefined, { granularity: "grapheme" }).segment(text)].length;
  const publish = async () => {
    if (!canPublish || publishPending.current) return;
    publishPending.current = true;
    setPublishing(true); setError(null); setResult(null);
    try {
      const outcome = await desktopApi.social.publish(post);
      setResult(outcome);
      if (outcome.publications.length > 0 && outcome.publications.every(publication => publication.status === "PUBLISHED")) setText("");
      else {
        const sent = new Set(outcome.publications.filter(publication => publication.status === "PUBLISHED" || publication.remotePostIds.length > 0).map(publication => publication.accountId));
        setSelected(selected.filter(id => !sent.has(id)));
      }
    }
    catch (cause) { setError(cause instanceof Error ? cause.message : String(cause)); }
    finally { publishPending.current = false; setPublishing(false); }
  };
  return <div className="page composer-page">
    <div className="page-heading"><div className="heading-with-icon"><span className="heading-icon"><Feather size={22} /></span><div><p className="eyebrow">UNIFIED COMPOSER</p><h1>Federated Broadcast Studio</h1><p>Write once. Belong everywhere.</p></div></div><span className="badge"><span className="status-dot" />{native ? "Desktop engine" : "Browser preview"}</span></div>
    <section aria-labelledby="targets-heading"><div className="section-label"><h2 id="targets-heading">Publish targets <span>· {destinations.length} selected</span></h2><button className="button button-link" onClick={onAccounts}><Link2 size={13} />Link an account</button></div>
      <div className="target-grid">{accounts.map(account => <label className={`target-tile ${account.provider.toLowerCase()} ${selected.includes(account.id) ? "selected" : ""}`} key={account.id}><input type="checkbox" disabled={publishing || refreshing} checked={selected.includes(account.id)} onChange={() => toggle(account.id)} /><div className="target-details"><div><strong>{account.displayName} <span>{providerNames[account.provider]}</span></strong><span className="badge">{account.capabilities.maxTextLength.toLocaleString()} max</span></div><span className="handle">{account.handle}</span><span className="target-protocol"><ProviderIcon provider={account.provider} />{protocolNames[account.provider]}</span></div></label>)}</div>
      {!accounts.length && <Notice>No accounts yet. Link an account to choose a publishing destination.</Notice>}
    </section>
    <LimitMonitor accounts={destinations} count={count} />
    <div className="composer-grid"><div className="editor-column"><section className="panel editor-panel" aria-labelledby="editor-heading"><div className="panel-heading"><h2 id="editor-heading"><Type size={18} />Your post</h2><span className="badge">Plain text</span></div><label className="sr-only" htmlFor="post-text">Post text</label><textarea id="post-text" disabled={publishing} value={text} onChange={event => { setText(event.target.value); if (succeeded) setResult(null); }} placeholder="What’s happening?" spellCheck /><div className="editor-footer"><span>One source for every community</span><span>{count.toLocaleString()} graphemes</span></div></section>
    <section className="panel policy-panel"><fieldset disabled={publishing}><legend><GitBranch size={17} />Publishing policy</legend><p className="muted">Choose how your post adapts across networks.</p><div className="policies">{policies.map(item => <label key={item.value} className={`policy ${policy === item.value ? "selected" : ""}`}><input type="radio" name="publishing-policy" checked={policy === item.value} onChange={() => setPolicy(item.value)} /><span><strong>{item.label}</strong><small>{item.detail}</small></span>{policy === item.value && <Check size={14} />}</label>)}</div></fieldset></section>
    {policy === "COMMON_LIMIT" && preview?.effectiveLimit && <Notice>Common limit: {preview.effectiveLimit} graphemes, set by {accounts.find(account => account.id === preview.limitingAccountId)?.handle}.</Notice>}
    {!native && <Notice>Preview mode. Open the desktop app to calculate native threads and publish to your accounts.</Notice>}
    {native && disconnected && <Notice error>Some selected accounts need reconnecting. <button className="button" onClick={onAccounts}>Manage accounts</button></Notice>}
    {planning && <Notice>Planning native threads…</Notice>}
    {previewError && <Notice error><span>{previewError}</span><button className="button" onClick={retry}>Retry preview</button></Notice>}
    {error && <Notice error>{error}</Notice>}

    </div><DestinationPreview accounts={destinations} preview={preview} text={text} native={native} /></div>
    {result && <Notice error={!succeeded}><strong>{succeeded ? "Published successfully" : receivedPosts ? "Some posts were published" : "Publishing failed"}</strong><ul className="publication-results">{result.publications.map(publication => <li key={publication.accountId}><span>{accounts.find(account => account.id === publication.accountId)?.handle ?? publication.accountId}</span><span>{publication.status === "PUBLISHED" ? "Published" : publication.remotePostIds.length ? "Partially published" : "Failed"}{publication.error ? `: ${publication.error}` : ""}</span></li>)}</ul>{!succeeded && receivedPosts && <p>Your draft was kept. Destinations that received posts have been deselected. Check any partially published threads before sending again.</p>}</Notice>}
    <div className="dispatch-bar"><div role="status" aria-live="polite" aria-atomic="true">{succeeded ? <Check size={16} aria-hidden="true" /> : feedback ? <CircleAlert size={16} aria-hidden="true" /> : <span className="status-dot" />}<span>{publishing ? "Publishing to your destinations…" : feedback ? feedback : !native ? "Desktop app required to publish" : !destinations.length ? "Select a destination" : !text.trim() ? "Write your first line" : refreshing ? "Refreshing accounts…" : disconnected ? "Reconnect selected accounts" : planning ? "Planning native threads…" : previewError ? "Resolve preview error" : "Ready to multi-publish"}</span></div><button className="button button-primary publish-button" disabled={!canPublish} aria-busy={publishing} onClick={() => void publish()}><Send size={16} />{publishing ? "Publishing…" : `Publish to ${destinations.length} selected`}</button></div>
  </div>;
}
