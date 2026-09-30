import type { ReactNode } from "react";
/** Standard heading row for a unified view, including its primary controls. */
export function ViewHeader({ icon, title, subtitle, controls }: { icon: ReactNode; title: string; subtitle: string; controls: ReactNode }) { return <header className="unified-header"><div className="unified-title">{icon}<div><h1>{title}</h1><p>{subtitle}</p></div></div>{controls}</header>; }
