import { useCallback, useEffect, useRef, useState } from "react";
import type { Page } from "../components/AppShell";
import type { DetailTarget } from "../components/details/SocialDetailView";

interface NavigationState {
  readonly page: Page;
  readonly profileAccountId: string | null;
  readonly detail: DetailTarget | null;
  readonly sequence: number;
}

const initial: NavigationState = { page: "timeline", profileAccountId: null, detail: null, sequence: 0 };
const isNavigationState = (value: unknown): value is NavigationState => typeof value === "object" && value !== null && "page" in value && "sequence" in value;
const routeUrl = (state: NavigationState): string => {
  const params = new URLSearchParams();
  params.set("view", state.detail?.kind.toLocaleLowerCase() ?? state.page);
  if (state.profileAccountId) params.set("account", state.profileAccountId);
  if (state.detail) {
    params.set("acting", state.detail.accountId);
    params.set("target", state.detail.kind === "SOURCE" ? state.detail.source.id : state.detail.id);
  }
  return `${window.location.pathname}?${params.toString()}`;
};

/** History entries retain route state and scroll state while their views stay mounted. */
export function useWorkspaceNavigation() {
  const [state, setState] = useState<NavigationState>(() => isNavigationState(window.history.state) ? window.history.state : initial);
  const scrollPositions = useRef(new Map<number, number>());
  useEffect(() => {
    if (!isNavigationState(window.history.state)) window.history.replaceState(initial, "", routeUrl(initial));
    const pop = (event: PopStateEvent) => {
      if (!isNavigationState(event.state)) return;
      setState(event.state);
      requestAnimationFrame(() => window.scrollTo({ top: scrollPositions.current.get(event.state.sequence) ?? 0 }));
    };
    window.addEventListener("popstate", pop);
    return () => window.removeEventListener("popstate", pop);
  }, []);
  const commit = useCallback((next: Omit<NavigationState, "sequence">) => {
    scrollPositions.current.set(state.sequence, window.scrollY);
    const value = { ...next, sequence: state.sequence + 1 };
    window.history.pushState(value, "", routeUrl(value));
    setState(value);
    requestAnimationFrame(() => { document.getElementById("main")?.focus({ preventScroll: true }); window.scrollTo({ top: 0 }); });
  }, [state]);
  const navigate = useCallback((page: Page) => commit({ page, profileAccountId: page === "myprofiles" ? state.profileAccountId : null, detail: null }), [commit, state.profileAccountId]);
  const selectProfileAccount = useCallback((profileAccountId: string | null) => commit({ page: "myprofiles", profileAccountId, detail: null }), [commit]);
  const openDetail = useCallback((detail: DetailTarget) => commit({ page: state.page, profileAccountId: state.profileAccountId, detail }), [commit, state.page, state.profileAccountId]);
  const back = useCallback(() => { if (state.sequence > 0) window.history.back(); else navigate(state.page); }, [navigate, state.page, state.sequence]);
  return { ...state, navigate, selectProfileAccount, openDetail, back };
}
