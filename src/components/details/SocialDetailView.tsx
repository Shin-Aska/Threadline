import type { WorkspaceState } from "../../types";
import type { FollowedSource } from "../../types/social";
import { ProfileDetail } from "./ProfileDetail";
import { SourceDetail } from "./SourceDetail";
import { TagDetail } from "./TagDetail";
import { ThreadDetail } from "./ThreadDetail";

export type DetailTarget =
  | { readonly kind: "PROFILE"; readonly accountId: string; readonly id: string; readonly origin: string }
  | { readonly kind: "POST"; readonly accountId: string; readonly id: string; readonly origin: string }
  | { readonly kind: "TAG"; readonly accountId: string; readonly id: string; readonly origin: string }
  | { readonly kind: "SOURCE"; readonly accountId: string; readonly source: FollowedSource; readonly origin: string };

interface DetailProps {
  readonly target: DetailTarget;
  readonly workspace: WorkspaceState;
  readonly onBack: () => void;
  readonly onPost: (accountId: string, postId: string) => void;
  readonly onProfile: (accountId: string, profileId: string) => void;
  readonly onTag: (accountId: string, tag: string) => void;
}

export function SocialDetailView(props: DetailProps) {
  switch (props.target.kind) {
    case "PROFILE": return <ProfileDetail {...props} target={props.target} />;
    case "POST": return <ThreadDetail {...props} target={props.target} />;
    case "TAG": return <TagDetail {...props} target={props.target} />;
    case "SOURCE": return <SourceDetail {...props} target={props.target} />;
  }
}
