import { useEffect, useRef, useState } from "react";
import { Plus, X } from "lucide-react";
import type { WorkspaceState } from "../types";
import { useComposerStore } from "../stores/composer";
import { ProviderIcon } from "./ui";
import { providerNames } from "./providers";

interface DestinationSelectorProps {
  readonly workspace: WorkspaceState;
  readonly disabled: boolean;
  readonly onAccounts: () => void;
}

export function DestinationSelector({ workspace, disabled, onAccounts }: DestinationSelectorProps) {
  const { selected, toggle } = useComposerStore();
  const [open, setOpen] = useState(false);
  const picker = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const destinations = workspace.accounts.filter(account => selected.includes(account.id));
  useEffect(() => {
    if (!open) return;
    const dismiss = (event: PointerEvent) => {
      if (event.target instanceof Node && !picker.current?.contains(event.target)) setOpen(false);
    };
    document.addEventListener("pointerdown", dismiss);
    return () => document.removeEventListener("pointerdown", dismiss);
  }, [open]);

  return <section className="destination-selector" aria-labelledby="destinations-heading">
    <h2 id="destinations-heading">Post to</h2>
    <div className="destination-chips">
      {destinations.map(account => <div className={"destination-chip " + account.provider.toLowerCase()} key={account.id}>
        <ProviderIcon provider={account.provider} />
        <span className="destination-copy"><strong>{providerNames[account.provider]} <span>· {account.displayName}</span></strong><small>{account.handle}</small>{!workspace.connectedAccountIds.includes(account.id) && <small className="danger">Reconnect required</small>}</span>
        <button type="button" className="remove-destination" disabled={disabled} aria-label={`Remove destination ${account.displayName} · ${providerNames[account.provider]} · ${account.handle}`} onClick={() => { toggle(account.id); trigger.current?.focus(); }}><X size={16} aria-hidden="true" /></button>
      </div>)}
      <div className="destination-picker" ref={picker} onBlur={event => { if (!event.currentTarget.contains(event.relatedTarget)) setOpen(false); }} onKeyDown={event => { if (event.key === "Escape" && open) { event.stopPropagation(); setOpen(false); trigger.current?.focus(); } }}>
        <button ref={trigger} type="button" className="button add-destination" disabled={disabled} aria-expanded={open && !disabled} aria-controls="destination-options" onClick={() => setOpen(value => !value)}><Plus size={18} />Add more</button>
        {open && !disabled && <div className="destination-options" id="destination-options">
          <fieldset><legend>Choose accounts</legend>
            {workspace.accounts.map(account => <label className="destination-option" key={account.id}>
              <input type="checkbox" checked={selected.includes(account.id)} onChange={() => toggle(account.id)} aria-label={`${account.displayName} · ${providerNames[account.provider]} · ${account.handle}`} />
              <ProviderIcon provider={account.provider} />
              <span><strong>{account.displayName}</strong><small>{account.handle}</small>{!workspace.connectedAccountIds.includes(account.id) && <small className="danger">Reconnect required</small>}</span>
            </label>)}
          </fieldset>
          <button type="button" className="button link-account" onClick={() => { setOpen(false); onAccounts(); }}><Plus size={16} />Link an account</button>
        </div>}
      </div>
    </div>
    {!destinations.length && <p className="destination-hint">Choose an account to preview and publish your post.</p>}
  </section>;
}
