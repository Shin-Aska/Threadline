import { Check, ChevronDown, Filter, Search, Settings2 } from "lucide-react";
import { useState } from "react";
import type { SocialOrder } from "../../services/social/presentation";

const orders: readonly { readonly value: SocialOrder; readonly label: string }[] = [
  { value: "LATEST", label: "Latest first" },
  { value: "OLDEST", label: "Oldest first" },
  { value: "LIKES", label: "Most liked" },
  { value: "DISCUSSED", label: "Most discussed" },
];

interface ToolbarProps {
  readonly query: string;
  readonly order: SocialOrder;
  readonly mediaOnly: boolean;
  readonly hideReposts: boolean;
  readonly onQuery: (value: string) => void;
  readonly onOrder: (value: SocialOrder) => void;
  readonly onMediaOnly: (value: boolean) => void;
  readonly onHideReposts: (value: boolean) => void;
}

export function SocialFeedToolbar(props: ToolbarProps) {
  const [sortOpen, setSortOpen] = useState(false);
  const [filtersOpen, setFiltersOpen] = useState(false);
  const label = orders.find(item => item.value === props.order)?.label ?? "Latest first";
  return <><div className="search-row"><label><Search size={17} /><span className="sr-only">Search loaded posts</span><input value={props.query} onChange={event => props.onQuery(event.target.value)} placeholder="Search loaded posts, people, or topics…" /></label><div className="sort-wrap"><button className="button" aria-expanded={sortOpen} onClick={() => setSortOpen(value => !value)}><Filter size={16} />{label}<ChevronDown size={14} /></button>{sortOpen && <div className="source-menu">{orders.map(item => <button key={item.value} onClick={() => { props.onOrder(item.value); setSortOpen(false); }}>{item.value === props.order && <Check size={14} />}{item.label}</button>)}<small>Applies to loaded posts</small></div>}</div><button className="button" aria-expanded={filtersOpen} onClick={() => setFiltersOpen(value => !value)}><Settings2 size={16} />Filters</button></div>{filtersOpen && <div className="panel filter-panel"><label><input type="checkbox" checked={props.mediaOnly} onChange={event => props.onMediaOnly(event.target.checked)} />Images &amp; video</label><label><input type="checkbox" checked={props.hideReposts} onChange={event => props.onHideReposts(event.target.checked)} />Hide replies</label><small className="muted">Filters apply to posts already loaded from providers.</small></div>}</>;
}
