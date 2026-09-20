import { useEffect, useId, useRef, useState } from "react";
import type { RefObject } from "react";
import { createPortal } from "react-dom";
import { CornerDownLeft, Search, X } from "lucide-react";
import { useHashtags } from "../hooks/useHashtags";
import { aggregateHashtags, hashtagActivity } from "../hooks/hashtagResults";
import { useHashtagAnchor } from "../hooks/useHashtagAnchor";
import type { Provider, WorkspaceState } from "../types";
import { AccountStack, ProviderIcon } from "./ui";

interface Props {
  readonly query: string;
  readonly accountIds: readonly string[];
  readonly workspace: WorkspaceState;
  readonly editor: RefObject<HTMLTextAreaElement | null>;
  readonly text: string;
  readonly anchorIndex: number;
  readonly onChoose: (name: string) => void;
  readonly onClose: () => void;
}
const filters = [{ value: "ALL", label: "All" }, { value: "BLUESKY", label: "Bluesky" }, { value: "MASTODON", label: "Mastodon" }] as const;
export function HashtagSuggestions({ query, accountIds, workspace, editor, text, anchorIndex, onChoose, onClose }: Props) {
  const { results, retry } = useHashtags(query, accountIds, workspace);
  const [filter, setFilter] = useState<Provider | "ALL">("ALL");
  const [expandedKey, setExpandedKey] = useState<string | null>(null);
  const [cursor, setCursor] = useState<{ readonly key: string; readonly name: string } | null>(null);
  const panel = useRef<HTMLDivElement>(null);
  const position = useHashtagAnchor(editor, panel, text, anchorIndex);
  const listId = useId();
  const accounts = workspace.accounts.filter(account => accountIds.includes(account.id));
  const visibleAccounts = accounts.filter(account => filter === "ALL" || account.provider === filter);
  const rows = aggregateHashtags(accounts, results, filter);
  const key = JSON.stringify([query, accountIds, filter]);
  const expanded = expandedKey === key;
  const options = expanded ? rows : rows.slice(0, 5);
  const selectedIndex = Math.max(0, options.findIndex(row => cursor?.key === key && row.name === cursor.name));
  const activeId = options.length ? listId + "-" + selectedIndex : undefined;
  const loading = visibleAccounts.some(account => !results[account.id] && (query || account.provider !== "BLUESKY"));
  useEffect(() => {
    const input = editor.current;
    if (!input) return;
    input.setAttribute("aria-controls", listId);
    input.setAttribute("aria-autocomplete", "list");
    if (activeId) input.setAttribute("aria-activedescendant", activeId);
    else input.removeAttribute("aria-activedescendant");
    const keydown = (event: KeyboardEvent) => {
      if (event.isComposing || event.ctrlKey || event.metaKey || event.altKey || event.shiftKey || !position.visible) return;
      if (event.key === "Tab") { event.preventDefault(); panel.current?.querySelector<HTMLButtonElement>("button")?.focus(); return; }
      if (event.key === "Escape") { event.preventDefault(); onClose(); return; }
      if ((event.key === "ArrowDown" || event.key === "ArrowUp") && options.length) {
        event.preventDefault();
        const next = options[(selectedIndex + (event.key === "ArrowDown" ? 1 : -1) + options.length) % options.length];
        if (next) setCursor({ key, name: next.name });
      }
      if (event.key === "Enter" && options[selectedIndex]) { event.preventDefault(); onChoose(options[selectedIndex].name); }
    };
    input.addEventListener("keydown", keydown);
    return () => {
      input.removeEventListener("keydown", keydown);
      input.removeAttribute("aria-controls"); input.removeAttribute("aria-autocomplete"); input.removeAttribute("aria-activedescendant");
    };
  }, [editor, listId, activeId, options, selectedIndex, key, onChoose, onClose, position.visible]);
  useEffect(() => {
    if (activeId) document.getElementById(activeId)?.scrollIntoView({ block: "nearest" });
  }, [activeId, key]);
  useEffect(() => {
    const dismissOutside = (event: Event) => {
      if (event.target instanceof Node && event.target !== editor.current && !panel.current?.contains(event.target)) onClose();
    };
    document.addEventListener("pointerdown", dismissOutside);
    document.addEventListener("focusin", dismissOutside);
    return () => { document.removeEventListener("pointerdown", dismissOutside); document.removeEventListener("focusin", dismissOutside); };
  }, [editor, onClose]);
  return createPortal(<div ref={panel} className="hashtag-popover" style={{ left: position.left, top: position.top, width: position.width, maxHeight: position.maxHeight, visibility: position.visible ? "visible" : "hidden" }} onKeyDown={event => { if (event.key === "Escape") { onClose(); editor.current?.focus(); } }}>
    <div className="hashtag-filters" role="group" aria-label="Hashtag sources">
      {filters.map(item => <button type="button" className="hashtag-filter" aria-pressed={filter === item.value} key={item.value} onMouseDown={event => event.preventDefault()} onClick={() => { setFilter(item.value); setExpandedKey(null); editor.current?.focus(); }} >{item.value !== "ALL" && <ProviderIcon provider={item.value} />}{item.label}</button>)}
      <button type="button" className="hashtag-close" aria-label="Dismiss hashtag suggestions" onClick={() => { onClose(); editor.current?.focus(); }}><X size={14} /></button>
    </div>
    <div className="hashtag-scroll" tabIndex={0}>
      <div role="listbox" id={listId} aria-label="Hashtag suggestions">
        {options.map((row, index) => { const activity = hashtagActivity(row); return <div role="option" id={listId + "-" + index} aria-selected={selectedIndex === index} aria-label={"#" + row.name + ", " + activity.label + ", " + row.sources.length + " accounts"} className="hashtag-option" key={row.name} title={activity.detail} aria-description={activity.detail} onMouseDown={event => event.preventDefault()} onMouseMove={() => setCursor({ key, name: row.name })} onClick={() => onChoose(row.name)}>
          <strong>#{row.name}</strong><AccountStack accounts={row.sources.map(source => source.account)} /><span className="hashtag-count">{activity.label}</span>{selectedIndex === index && <CornerDownLeft size={14} />}
        </div>; })}
      </div>
      {workspace.mode === "BROWSER" ? <p className="hashtag-status" role="status">Hashtag lookup is available in the desktop app.</p> : <>
        {loading && <p className="hashtag-status" role="status">Looking up hashtags…</p>}
        {!loading && !options.length && !visibleAccounts.some(account => results[account.id]?.status === "error") && <p className="hashtag-status" role="status">{!visibleAccounts.length ? "Select an account for this network." : !query && filter === "BLUESKY" ? "Type a hashtag to check search activity." : "No matching hashtags found."}</p>}
        {visibleAccounts.map(account => { const result = results[account.id]; return result?.status === "error" && <div className="hashtag-error" role="status" key={account.id}><span>{account.displayName}: {result.error}</span><button type="button" className="button button-link" onClick={() => { retry(account.id); editor.current?.focus(); }}>Retry hashtags<span className="sr-only"> for {account.handle}</span></button></div>; })}
      </>}
    </div>
    <div className="hashtag-bottom"><button type="button" disabled={expanded || rows.length <= 5} onClick={() => { setExpandedKey(key); editor.current?.focus(); }}><Search size={13} />{expanded ? "All available suggestions" : "View more hashtag suggestions"}</button><span>↑↓ · ↵</span></div>
    <span className="sr-only" role="status">{options.length} suggestions. Up and Down to navigate. Enter to insert. Escape to close.</span>
  </div>, document.body);
}
