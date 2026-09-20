import type { CanonicalPost } from "../../types";
import type { DraftRecord } from "../../types/publishing";
import { publishingApi } from "../desktop/publishing";

let current: DraftRecord | null = null;
let serial: Promise<void> = Promise.resolve();

export const draftWriter = {
  seed(draft: DraftRecord | null): void { current = draft; },
  save(post: CanonicalPost, hint: DraftRecord | null): Promise<DraftRecord> {
    const operation = serial.then(async () => {
      const draft = current ?? hint;
      const saved = await publishingApi.drafts.save({ id: draft?.id ?? null, expectedRevision: draft?.revision ?? null, post });
      current = saved;
      return saved;
    });
    serial = operation.then(() => undefined, () => undefined);
    return operation;
  },
};
