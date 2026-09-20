import { invoke } from "@tauri-apps/api/core";
import type {
  DraftId,
  DraftRecord,
  PublicationRecord,
  SaveDraftInput,
  ScheduleId,
  ScheduledPublication,
} from "../../types/publishing";

const runningInTauri = (): boolean => "__TAURI_INTERNALS__" in window;

class DesktopPublishingUnavailableError extends Error {
  constructor() {
    super("Open Threadline desktop to manage drafts, schedules, and publications.");
    this.name = "DesktopPublishingUnavailableError";
  }
}

const invokeDesktop = <Result>(command: string, args?: Record<string, unknown>): Promise<Result> =>
  runningInTauri()
    ? invoke<Result>(command, args)
    : Promise.reject(new DesktopPublishingUnavailableError());

export const publishingApi = {
  drafts: {
    save: (input: SaveDraftInput): Promise<DraftRecord> => invokeDesktop("save_draft", { input }),
    list: (): Promise<readonly DraftRecord[]> => invokeDesktop("list_drafts"),
    get: (id: DraftId): Promise<DraftRecord> => invokeDesktop("get_draft", { id }),
    delete: (id: DraftId): Promise<void> => invokeDesktop("delete_draft", { id }),
  },
  publishDraft: (id: DraftId): Promise<PublicationRecord> => invokeDesktop("publish_draft", { id }),
  history: (): Promise<readonly PublicationRecord[]> => invokeDesktop("list_publications"),
  deleteHistory: (id: PublicationRecord["id"]): Promise<void> => invokeDesktop("delete_publication", { id }),
  schedules: {
    create: (draftId: DraftId, scheduledForEpochMs: number, timeZone: string): Promise<ScheduledPublication> =>
      invokeDesktop("create_schedule", { input: { draftId, scheduledForEpochMs, timeZone } }),
    list: (): Promise<readonly ScheduledPublication[]> => invokeDesktop("list_schedules"),
    reschedule: (id: ScheduleId, expectedRevision: number, scheduledForEpochMs: number, timeZone: string): Promise<ScheduledPublication> =>
      invokeDesktop("reschedule_publication", { input: { id, expectedRevision, scheduledForEpochMs, timeZone } }),
    cancel: (id: ScheduleId, expectedRevision: number): Promise<ScheduledPublication> =>
      invokeDesktop("cancel_schedule", { input: { id, expectedRevision } }),
    sendNow: (id: ScheduleId, expectedRevision: number): Promise<ScheduledPublication> =>
      invokeDesktop("send_schedule_now", { input: { id, expectedRevision } }),
  },
} as const;
