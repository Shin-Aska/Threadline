import { useRef, useState } from "react";
import { ArrowRight, ExternalLink, KeyRound, Link2, LockKeyhole, Plus, ShieldCheck, Trash2, Users, X } from "lucide-react";
import { desktopApi } from "../services/desktop";
import { connectBlueskyWithOAuth, connectMastodonWithOAuth } from "../services/desktop/oauth";
import type { Account, Provider, WorkspaceState } from "../types";
import { Identity, Notice, ProviderIcon } from "./ui";
import { protocolNames, providerNames } from "./providers";
import type { NativeOAuthAttempt } from "../types/oauth";

interface AccountsProps {
  readonly setup?: boolean;
  readonly workspace: WorkspaceState;
  readonly refreshing: boolean;
  readonly onConnected: (account: Account) => Promise<void>;
  readonly onRemoved: (id: string) => Promise<void>;
}

export function AccountsPanel({ workspace, refreshing, onConnected, onRemoved, setup = false }: AccountsProps) {
  const { accounts, mode, connectedAccountIds } = workspace;
  const native = mode !== "BROWSER";
  const secretInput = useRef<HTMLInputElement>(null);
  const [provider, setProvider] = useState<Provider>("BLUESKY");
  const [server, setServer] = useState("https://bsky.social");
  const [identifier, setIdentifier] = useState("");
  const [secret, setSecret] = useState("");
  const [pending, setPending] = useState(false);
  const oauthAttempt = useRef<NativeOAuthAttempt | null>(null);
  const [oauthPending, setOauthPending] = useState(false);
  const [removing, setRemoving] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const busy = pending || oauthPending || removing !== null || refreshing;
  const reconnect = (account: Account) => {
    setProvider(account.provider); setServer(account.instanceUrl ?? "");
    setIdentifier(account.handle); setSecret(""); setError(null);
    setMessage(account.instanceUrl ? `Reconnect ${account.handle} with fresh credentials.` : `Enter the original service URL for ${account.handle}, then add fresh credentials.`);
    requestAnimationFrame(() => secretInput.current?.focus());
  };
  const connect = async () => {
    if (!native || busy) return;
    setPending(true); setError(null); setMessage(null);
    try {
      const account = provider === "BLUESKY"
        ? await desktopApi.accounts.connectBluesky(server.trim(), identifier.trim(), secret)
        : await desktopApi.accounts.connectMastodon(server.trim(), secret);
      setSecret(""); await onConnected(account); setMessage(`${account.handle} connected and verified.`);
    } catch (cause) { setError(cause instanceof Error ? cause.message : String(cause)); }
    finally { setPending(false); }
  };
  const remove = async (account: Account) => {
    setRemoving(account.id); setError(null); setMessage(null);
    try { await desktopApi.accounts.remove(account.id); await onRemoved(account.id); setMessage(`${account.handle} removed.`); }
    catch (cause) { setError(cause instanceof Error ? cause.message : String(cause)); }
    finally { setRemoving(null); }
  };
  const connectOAuth = async () => {
    if (!native || busy) return;
    setOauthPending(true); setError(null); setMessage(null);
    try {
      const metadataUrl = document.querySelector<HTMLMetaElement>('meta[name="threadline-bluesky-client-metadata"]')?.content.trim() || undefined;
      const localDevelopment = ["localhost", "127.0.0.1"].includes(window.location.hostname);
      if (provider === "BLUESKY" && !metadataUrl && !localDevelopment) throw new Error("Bluesky browser sign-in needs a hosted public client metadata URL in this build.");
      const attempt = provider === "BLUESKY" ? connectBlueskyWithOAuth({ identifier: identifier.trim(), ...(metadataUrl ? { clientMetadataUrl: metadataUrl } : {}) }) : connectMastodonWithOAuth({ instanceUrl: server.trim() });
      oauthAttempt.current = attempt;
      const account = await attempt.completion;
      oauthAttempt.current = null; await onConnected(account); setMessage(`${account.handle} connected through your browser.`);
    } catch (cause) { setError(cause instanceof Error ? cause.message : String(cause)); }
    finally { oauthAttempt.current = null; setOauthPending(false); }
  };
  const cancelOAuth = async () => {
    const attempt = oauthAttempt.current;
    if (!attempt) return;
    try { await attempt.cancel(); setMessage("Browser sign-in cancelled."); }
    catch (cause) { setError(cause instanceof Error ? cause.message : String(cause)); }
  };
  const oauthReady = native && !busy && (provider === "BLUESKY" ? identifier.trim().length > 0 : server.trim().length > 0);
  return <div className="page accounts-page">
    <div className="page-heading"><div><p className="eyebrow">{setup ? "WELCOME TO THREADLINE" : "DECENTRALIZED CREDENTIAL VAULT"}</p><h1>{setup ? "Connect your first account" : "Connected Identities & Instances"}</h1><p>{setup ? "Bring your Bluesky or Mastodon account. Compose once, publish across your communities." : "Manage your identities across the open social web."}</p></div><span className="badge"><ShieldCheck size={13} />Local credentials</span></div>
    {!setup && <div className="stat-grid"><div className="panel stat"><span className="micro-heading">Connected accounts</span><strong>{accounts.length}<small> identities</small></strong><span className="muted">Available in this workspace</span></div><div className="panel stat bluesky"><span className="micro-heading">Bluesky / AT Protocol</span><strong>{accounts.filter(a => a.provider === "BLUESKY").length}<small> accounts</small></strong><span className="muted">App password authentication</span></div><div className="panel stat mastodon"><span className="micro-heading">Mastodon / ActivityPub</span><strong>{accounts.filter(a => a.provider === "MASTODON").length}<small> accounts</small></strong><span className="muted">Instance access tokens</span></div></div>}
    {!native && <Notice>You’re viewing Threadline in a browser. Open the desktop app to connect your account.</Notice>}
    <section className="panel connect-panel" aria-labelledby="connect-heading"><div className="panel-heading"><h2 id="connect-heading"><Plus size={18} />{setup ? "Choose your network" : "Authorize New Identity"}</h2><KeyRound size={16} className="muted" /></div>
      <div className="provider-tabs" aria-label="Account provider">{(["BLUESKY", "MASTODON"] as const).map(value => <button key={value} disabled={busy} className={`provider-tab ${value.toLowerCase()} ${provider === value ? "selected" : ""}`} aria-pressed={provider === value} onClick={() => { setProvider(value); setServer(value === "BLUESKY" ? "https://bsky.social" : "https://mastodon.social"); setSecret(""); setError(null); }}><ProviderIcon provider={value} /><span><strong>{providerNames[value]}</strong><small>{protocolNames[value]}</small></span></button>)}</div>
      <section className="oauth-connect"><div><h3>Sign in with your browser</h3><p className="muted">Threadline opens your provider’s authorization page. Tokens stay in the native credential store and never enter this form.</p></div><div className="oauth-fields">{provider === "BLUESKY" ? <label className="field"><span>Browser sign-in handle</span><input value={identifier} disabled={busy} onChange={event => setIdentifier(event.target.value)} placeholder="you.bsky.social" autoComplete="username" /></label> : <label className="field"><span>Browser sign-in instance URL</span><input type="url" value={server} disabled={busy} onChange={event => setServer(event.target.value)} placeholder="https://mastodon.social" autoComplete="url" /></label>}<button className="button button-blue" disabled={!oauthReady} onClick={() => void connectOAuth()}><ExternalLink size={15} />{oauthPending ? "Waiting for browser…" : "Continue in browser"}</button>{oauthPending && <button className="button" onClick={() => void cancelOAuth()}><X size={15} />Cancel sign-in</button>}</div></section>
      <div className="fallback-heading"><span>Credential fallback</span><p className="muted">Use a provider-issued app password or access token when browser sign-in is unavailable.</p></div>
      <form onSubmit={event => { event.preventDefault(); void connect(); }} className="connect-form">
        <label className="field"><span>{provider === "BLUESKY" ? "Service URL" : "Instance URL"}</span><input type="url" required value={server} disabled={busy} onChange={event => setServer(event.target.value)} placeholder={provider === "BLUESKY" ? "https://bsky.social" : "https://mastodon.social"} autoComplete="url" /></label>
        {provider === "BLUESKY" && <label className="field"><span>Handle</span><input required value={identifier} disabled={busy} onChange={event => setIdentifier(event.target.value)} placeholder="you.bsky.social" autoComplete="username" /></label>}
        <label className="field"><span>{provider === "BLUESKY" ? "App password" : "Access token"}</span><input ref={secretInput} type="password" required value={secret} disabled={busy} onChange={event => setSecret(event.target.value)} placeholder={provider === "BLUESKY" ? "xxxx-xxxx-xxxx-xxxx" : "Your instance access token"} autoComplete="off" /></label>
        <div className="form-action"><p><LockKeyhole size={14} />Verified with your provider before connecting.</p><button className="button button-primary" disabled={!native || busy || !server.trim() || !secret || (provider === "BLUESKY" && !identifier.trim())} type="submit">{pending ? "Verifying…" : "Connect account"}<ArrowRight size={15} /></button></div>
      </form>
      {error && <Notice error>{error}</Notice>}{message && <Notice>{message}</Notice>}
    </section>
    {!setup && <section className="panel account-list" aria-labelledby="identities-heading"><div className="panel-heading"><h2 id="identities-heading"><Users size={18} />Your identities</h2><span className="badge">{accounts.length} accounts</span></div>
      {accounts.map(account => <div className="managed-account" key={account.id}><Identity account={account} /><span className={`badge ${account.provider.toLowerCase()}`}><ProviderIcon provider={account.provider} />{providerNames[account.provider]}</span><span className="account-limit">{connectedAccountIds.includes(account.id) ? "Available" : "Reconnect required"}</span><div className="account-actions">{native && !connectedAccountIds.includes(account.id) && <button className="button" disabled={busy} aria-label={`Reconnect ${account.handle}`} onClick={() => reconnect(account)}>Reconnect</button>}<button className="button button-ghost danger" disabled={!native || busy} aria-label={`Remove ${account.handle}`} onClick={() => void remove(account)}><Trash2 size={15} /><span>{removing === account.id ? "Removing…" : "Remove"}</span></button></div></div>)}

      {!accounts.length && <div className="empty-state"><Link2 size={24} /><h3>No identities connected</h3><p>Connect your first account above to start cross-publishing.</p></div>}
    </section>}
    <div className="vault-note"><ShieldCheck size={21} /><div><h3>Local Vault & Privacy</h3><p>Passwords and access tokens stay in your operating system’s credential store.</p></div></div>
  </div>;
}
