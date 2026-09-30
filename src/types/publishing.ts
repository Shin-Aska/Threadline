import type { CanonicalPost } from "./index";

declare const publishingIdBrand: unique symbol;
type PublishingId<Name extends string> = string & { readonly [publishingIdBrand]: Name };

/** Branded identifier for a saved draft. */
export type DraftId = PublishingId<"DraftId">;
/** Branded identifier for a queued publication. */
export type ScheduleId = PublishingId<"ScheduleId">;
/** Branded identifier for a publication ledger record. */
export type PublicationId = PublishingId<"PublicationId">;

/** Creates a draft, or updates one only if its current revision still matches. */
export interface SaveDraftInput {
  readonly id: DraftId | null;
  readonly expectedRevision: number | null;
  readonly post: CanonicalPost;
}

/** Persisted editable draft and its optimistic-concurrency revision. */
export interface DraftRecord {
  readonly id: DraftId;
  readonly revision: number;
  readonly post: CanonicalPost;
  readonly createdAtEpochMs: number;
  readonly updatedAtEpochMs: number;
}

/** Lifecycle state shared by a destination and each segment in its thread. */
export type PublicationOutcome = "PENDING" | "IN_FLIGHT" | "PUBLISHED" | "FAILED" | "UNCERTAIN" | "BLOCKED";

/** Result for one numbered text segment sent to a destination. */
export interface PublicationSegment {
  readonly index: number;
  readonly status: PublicationOutcome;
  readonly remotePostId: string | null;
  readonly error: string | null;
}

/** Publication state and segment ledger for one destination account. */
export interface DestinationPublication {
  readonly accountId: string;
  readonly status: PublicationOutcome;
  readonly remotePostIds: readonly string[];
  readonly error: string | null;
  readonly segments: readonly PublicationSegment[];
}

/** Durable record of the exact draft revision sent and its destination results. */
export interface PublicationRecord {
  readonly id: PublicationId;
  readonly draftId: DraftId;
  readonly draftRevision: number;
  readonly post: CanonicalPost;
  readonly createdAtEpochMs: number;
  readonly completedAtEpochMs: number | null;
  readonly destinations: readonly DestinationPublication[];
}

/** Lifecycle state for a future publication. */
export type ScheduleStatus = "QUEUED" | "NEEDS_ATTENTION" | "DISPATCHING" | "COMPLETED" | "CANCELLED";

/** Scheduled snapshot, including its edit revision and any completed result. */
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
