interface PostVisibilityProps {
  readonly warning: string | null | undefined;
  readonly sensitive: boolean | undefined;
  readonly hasMedia: boolean;
  readonly revealed: boolean;
  readonly onToggle: () => void;
}

/** Hides warned or sensitive content until the reader explicitly reveals it. */
export function PostVisibility({ warning, sensitive, hasMedia, revealed, onToggle }: PostVisibilityProps) {
  const summary = warning?.trim();
  if (!summary && !(sensitive && hasMedia)) return null;
  const label = summary ? "post" : "media";
  return <div className="post-content-warning">
    <div><span className="micro-heading">{summary ? "Content warning" : "Sensitive media"}</span><strong>{summary || "Media marked sensitive by the author"}</strong></div>
    <button type="button" className="button" aria-expanded={revealed} onClick={onToggle}>{revealed ? `Hide ${label}` : `Show ${label}`}</button>
  </div>;
}
