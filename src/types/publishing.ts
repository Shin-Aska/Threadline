import type { CanonicalPost } from "./index";

declare const publishingIdBrand: unique symbol;
type PublishingId<Name extends string> = string & { readonly [publishingIdBrand]: Name };

export type DraftId = PublishingId<"DraftId">;
export type ScheduleId = PublishingId<"ScheduleId">;
export type PublicationId = PublishingId<"PublicationId">;

export interface SaveDraftInput {
  readonly id: DraftId | null;
  readonly expectedRevision: number | null;
  readonly post: CanonicalPost;
}

export interface DraftRecord {
  readonly id: DraftId;
  readonly revision: number;
  readonly post: CanonicalPost;
  readonly createdAtEpochMs: number;
  readonly updatedAtEpochMs: number;
}

export type PublicationOutcome = "PENDING" | "IN_FLIGHT" | "PUBLISHED" | "FAILED" | "UNCERTAIN" | "BLOCKED";

export interface PublicationSegment {
  readonly index: number;
  readonly status: PublicationOutcome;
  readonly remotePostId: string | null;
  readonly error: string | null;
}

export interface DestinationPublication {
  readonly accountId: string;
  readonly status: PublicationOutcome;
  readonly remotePostIds: readonly string[];
  readonly error: string | null;
  readonly segments: readonly PublicationSegment[];
}

export interface PublicationRecord {
  readonly id: PublicationId;
  readonly draftId: DraftId;
  readonly draftRevision: number;
  readonly post: CanonicalPost;
  readonly createdAtEpochMs: number;
  readonly completedAtEpochMs: number | null;
  readonly destinations: readonly DestinationPublication[];
}

export type ScheduleStatus = "QUEUED" | "NEEDS_ATTENTION" | "DISPATCHING" | "COMPLETED" | "CANCELLED";

export interface ScheduledPublication {
  readonly id: ScheduleId;
  readonly draftId: DraftId;
  readonly draftRevision: number;
  readonly post: CanonicalPost;
  readonly revision: number;
  readonly scheduledForEpochMs: number;
  readonly timeZone: string;
  readonly status: ScheduleStatus;
  readonly attentionReason: string | null;
  readonly result: PublicationRecord | null;
  readonly createdAtEpochMs: number;
  readonly updatedAtEpochMs: number;
}
