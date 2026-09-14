import { useCallback, useEffect, useState } from "react";
import { desktopApi } from "../services/desktop";
import type { CanonicalPost, PublishingPreview, WorkspaceState } from "../types";

type PreviewOutcome = { readonly post: CanonicalPost; readonly revision: number; readonly workspace: WorkspaceState } & (
  | { readonly status: "ready"; readonly preview: PublishingPreview }
  | { readonly status: "error"; readonly error: string }
);

export function usePostPreview(post: CanonicalPost, workspace: WorkspaceState, enabled: boolean) {
  const [outcome, setOutcome] = useState<PreviewOutcome | null>(null);
  const [revision, setRevision] = useState(0);
  const retry = useCallback(() => setRevision(value => value + 1), []);
  useEffect(() => {
    if (!enabled) return;
    let active = true;
    const timer = setTimeout(() => {
      const plan = async () => {
        try {
          const preview = await desktopApi.social.preview(post);
          if (active) setOutcome({ status: "ready", post, workspace, revision, preview });
        } catch (cause) {
          if (active) setOutcome({ status: "error", post, workspace, revision, error: cause instanceof Error ? cause.message : String(cause) });
        }
      };
      void plan();
    }, 250);
    return () => { active = false; clearTimeout(timer); };
  }, [post, workspace, enabled, revision]);
  const current = enabled && outcome?.post === post && outcome.workspace === workspace && outcome.revision === revision ? outcome : null;
  return {
    preview: current?.status === "ready" ? current.preview : null,
    error: current?.status === "error" ? current.error : null,
    planning: enabled && current === null,
    retry,
  };
}
