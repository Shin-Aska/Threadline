import { Cloud, Network } from "lucide-react";
import type { Account, Provider } from "../types";


export function ProviderIcon({ provider }: { readonly provider: Provider }) {
  const Icon = provider === "BLUESKY" ? Cloud : Network;
  return <Icon size={18} aria-hidden="true" />;
}

export function Identity({ account }: { readonly account: Account }) {
  return <div className={`identity ${account.provider.toLowerCase()}`}>
    <span className="avatar" aria-hidden="true">{account.displayName.slice(0, 1)}</span>
    <div className="identity-text"><strong>{account.displayName}</strong><span className="handle">{account.handle}</span></div>
  </div>;
}

export function Notice({ children, error = false }: { readonly children: React.ReactNode; readonly error?: boolean }) {
  return <div className={`notice ${error ? "notice-error" : ""}`} role={error ? "alert" : "status"}>{children}</div>;
}
