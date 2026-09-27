import { type MouseEvent, type ReactNode, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import "./external-links.css";

const WARNING_PREFERENCE = "threadline:external-link-warning";
const PREFERENCE_CHANGED = "threadline:external-link-warning-changed";
const LINK_OR_TAG = /https?:\/\/[^\s<>"']+|#[\p{L}\p{N}_]+/giu;

function httpUrl(value: string): URL | null {
  try {
    const url = new URL(value);
    return url.protocol === "http:" || url.protocol === "https:" ? url : null;
  } catch {
    return null;
  }
}

function warningEnabled(): boolean {
  try {
    return localStorage.getItem(WARNING_PREFERENCE) !== "off";
  } catch {
    return true;
  }
}

function setWarningEnabled(enabled: boolean): void {
  localStorage.setItem(WARNING_PREFERENCE, enabled ? "on" : "off");
  window.dispatchEvent(new Event(PREFERENCE_CHANGED));
}

function urlLength(candidate: string): number {
  let end = candidate.length;
  while (end > 0) {
    const last = candidate[end - 1];
    if (last && /[.,!?;:]/.test(last)) { end -= 1; continue; }
    if (last === ")" || last === "]" || last === "}") {
      const open = last === ")" ? "(" : last === "]" ? "[" : "{";
      const matched = candidate.slice(0, end);
      if (matched.split(last).length > matched.split(open).length) { end -= 1; continue; }
    }
    break;
  }
  return end;
}

type ExternalLinkProps = {
  readonly href: string;
  readonly children: ReactNode;
  readonly className?: string;
  readonly ariaLabel?: string;
};

export function ExternalLink({ href, children, className, ariaLabel }: ExternalLinkProps) {
  const url = httpUrl(href);
  const [confirming, setConfirming] = useState(false);
  const [remember, setRemember] = useState(false);
  const dialogRef = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog || !confirming) return;
    dialog.showModal();
    return () => dialog.close();
  }, [confirming]);

  if (!url) return <span className={className}>{children}</span>;

  const handleClick = (event: MouseEvent<HTMLAnchorElement>) => {
    event.stopPropagation();
    if (!warningEnabled()) return;
    event.preventDefault();
    setRemember(false);
    setConfirming(true);
  };

  return <>
    <a href={url.href} target="_blank" rel="noopener noreferrer" className={className} aria-label={ariaLabel} onClick={handleClick} onAuxClick={event => { if (event.button === 1) handleClick(event); }}>{children}</a>
    {confirming && createPortal(<dialog ref={dialogRef} className="external-link-dialog" aria-labelledby="external-link-title" onCancel={() => setConfirming(false)} onClick={event => event.stopPropagation()}>
      <h2 id="external-link-title">Open an external link?</h2>
      <p>This link leaves Threadline and opens <strong>{url.hostname}</strong> in your browser.</p>
      <p className="external-link-destination">{url.href}</p>
      <label className="external-link-remember"><input type="checkbox" checked={remember} onChange={event => setRemember(event.target.checked)} /> Don’t warn me again</label>
      <div className="external-link-actions">
        <button type="button" className="button" onClick={() => setConfirming(false)}>Cancel</button>
        <a className="button button-blue" href={url.href} target="_blank" rel="noopener noreferrer" onClick={event => { event.stopPropagation(); if (remember) setWarningEnabled(false); setConfirming(false); }}>Continue</a>
      </div>
    </dialog>, document.body)}
  </>;
}

type LinkifiedTextProps = {
  readonly text: string;
  readonly onTag?: (tag: string) => void;
};

export function LinkifiedText({ text, onTag }: LinkifiedTextProps) {
  const parts: ReactNode[] = [];
  let cursor = 0;
  for (const match of text.matchAll(LINK_OR_TAG)) {
    const start = match.index;
    if (start > cursor) parts.push(text.slice(cursor, start));
    const token = match[0];
    if (token.startsWith("#")) {
      parts.push(onTag ? <button type="button" className="link quiet" key={start} onClick={event => { event.stopPropagation(); onTag(token.slice(1)); }}>{token}</button> : token);
      cursor = start + token.length;
      continue;
    }
    const length = urlLength(token);
    const href = token.slice(0, length);
    parts.push(<ExternalLink key={start} href={href} className="external-post-link">{href}</ExternalLink>);
    if (length < token.length) parts.push(token.slice(length));
    cursor = start + token.length;
  }
  if (cursor < text.length) parts.push(text.slice(cursor));
  return <>{parts}</>;
}

export function ExternalLinkWarningSetting() {
  const [enabled, setEnabled] = useState(warningEnabled);
  useEffect(() => {
    const sync = () => setEnabled(warningEnabled());
    window.addEventListener("storage", sync);
    window.addEventListener(PREFERENCE_CHANGED, sync);
    return () => { window.removeEventListener("storage", sync); window.removeEventListener(PREFERENCE_CHANGED, sync); };
  }, []);
  return <label className="external-link-setting"><input type="checkbox" checked={enabled} onChange={event => { setWarningEnabled(event.target.checked); setEnabled(event.target.checked); }} />Warn before opening external links</label>;
}
