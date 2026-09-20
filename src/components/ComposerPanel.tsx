import { useMemo, useRef, useState } from "react";
import { Calendar, Check, CircleAlert, Info, Save, Send } from "lucide-react";
import { publishingApi } from "../services/desktop/publishing";
import { draftWriter } from "../services/publishing/draftWriter";
import { usePostPreview } from "../hooks/usePostPreview";
import { useComposerStore } from "../stores/composer";
import type { CanonicalPost, WorkspaceState } from "../types";
import type { DraftRecord, PublicationRecord } from "../types/publishing";
import { DestinationPreview } from "./DestinationPreview";
import { DestinationSelector } from "./DestinationSelector";
import { ImageAttachments } from "./ImageAttachments";
import { HashtagSuggestions } from "./HashtagSuggestions";
import { activeHashtag } from "../hooks/useHashtags";
import { LimitMonitor } from "./LimitMonitor";
import { Notice } from "./ui";

const policies = [
  { value: "ADAPTIVE", label: "Adaptive", detail: "Thread only where needed" },
  {
    value: "COMMON_LIMIT",
    label: "Common limit",
    detail: "Use the shortest account limit",
  },
  {
    value: "ALWAYS_THREAD",
    label: "Always thread",
    detail: "Number every destination’s posts",
  },
] as const;
interface ComposerProps {
  readonly workspace: WorkspaceState;
  readonly refreshing: boolean;
  readonly onAccounts: () => void;
  readonly draft: DraftRecord | null;
  readonly onDraftSaved: (draft: DraftRecord) => void;
  readonly onQueueChanged: () => void;
}

export function ComposerPanel({
  workspace,
  refreshing,
  onAccounts,
  draft,
  onDraftSaved,
  onQueueChanged,
}: ComposerProps) {
  const { accounts, mode, connectedAccountIds } = workspace;
  const native = mode !== "BROWSER";
  const {
    text,
    media,
    policy,
    selected,
    setText,
    setMedia,
    clearDraft,
    setPolicy,
    publishing,
    setPublishing,
    setSelected,
  } = useComposerStore();
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<PublicationRecord | null>(null);
  const [scheduleOpen, setScheduleOpen] = useState(false);
  const [scheduleValue, setScheduleValue] = useState("");
  const [readingImages, setReadingImages] = useState(false);
  const publishPending = useRef(false);
  const editor = useRef<HTMLTextAreaElement>(null);
  const [selection, setSelection] = useState({ start: 0, end: 0 });
  const [composing, setComposing] = useState(false);
  const [dismissedTag, setDismissedTag] = useState<string | null>(null);
  const tagKey = JSON.stringify([text, selection.start, selection.end]);
  const activeTag =
    !publishing && !composing && dismissedTag !== tagKey
      ? activeHashtag(text, selection.start, selection.end)
      : null;
  const chooseTag = (name: string) => {
    if (!activeTag || publishing) return;
    const nextText =
      text.slice(0, activeTag.start) + "#" + name + text.slice(activeTag.end);
    const caret = activeTag.start + name.length + 1;
    setText(nextText);
    setSelection({ start: caret, end: caret });
    setDismissedTag(JSON.stringify([nextText, caret, caret]));
    if (succeeded) setResult(null);
    requestAnimationFrame(() => {
      editor.current?.focus();
      editor.current?.setSelectionRange(caret, caret);
    });
  };
  const succeeded =
    !!result?.destinations.length &&
    result.destinations.every(
      (publication) => publication.status === "PUBLISHED",
    );
  const receivedPosts =
    result?.destinations.some(
      (publication) =>
        publication.status === "PUBLISHED" ||
        publication.remotePostIds.length > 0,
    ) ?? false;
  const uncertain =
    result?.destinations.some(
      (publication) => publication.status === "UNCERTAIN",
    ) ?? false;
  const feedback = succeeded
    ? "Published successfully. Ready for your next post."
    : uncertain
      ? "Provider result uncertain. Check the original network before retrying."
      : receivedPosts
        ? "Some posts were published. Review results before retrying."
        : result || error
          ? "Publishing failed. Your draft was kept."
          : null;
  const destinations = accounts.filter((account) =>
    selected.includes(account.id),
  );
  const post = useMemo<CanonicalPost>(
    () => ({ text, media, policy, destinationAccountIds: selected }),
    [text, media, policy, selected],
  );
  const {
    preview,
    error: previewError,
    planning,
    retry,
  } = usePostPreview(
    post,
    workspace,
    native &&
      !refreshing &&
      selected.length > 0 &&
      (!!text.trim() || media.length > 0),
  );
  const disconnected = destinations.some(
    (account) => !connectedAccountIds.includes(account.id),
  );
  const canPublish =
    native &&
    !!preview &&
    !disconnected &&
    !refreshing &&
    !publishing &&
    !readingImages;
  const count = [
    ...new Intl.Segmenter(undefined, { granularity: "grapheme" }).segment(text),
  ].length;
  const publish = async () => {
    if (!canPublish || publishPending.current) return;
    publishPending.current = true;
    setPublishing(true);
    setError(null);
    setResult(null);
    try {
      const saved = await draftWriter.save(post, draft);
      onDraftSaved(saved);
      const outcome = await publishingApi.publishDraft(saved.id);
      setResult(outcome);
      onQueueChanged();
      if (
        outcome.destinations.length > 0 &&
        outcome.destinations.every(
          (publication) => publication.status === "PUBLISHED",
        )
      )
        clearDraft();
      else {
        const sent = new Set(
          outcome.destinations
            .filter(
              (publication) =>
                publication.status === "PUBLISHED" ||
                publication.remotePostIds.length > 0,
            )
            .map((publication) => publication.accountId),
        );
        setSelected(selected.filter((id) => !sent.has(id)));
      }
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      publishPending.current = false;
      setPublishing(false);
    }
  };
  const save = async () => {
    if (!native || publishing || (!text.trim() && media.length === 0)) return;
    setError(null);
    try {
      const saved = await draftWriter.save(post, draft);
      onDraftSaved(saved);
      onQueueChanged();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  };
  const schedule = async () => {
    const scheduledForEpochMs = new Date(scheduleValue).getTime();
    if (!native || !Number.isFinite(scheduledForEpochMs)) return;
    setPublishing(true);
    setError(null);
    try {
      const saved = await draftWriter.save(post, draft);
      onDraftSaved(saved);
      await publishingApi.schedules.create(
        saved.id,
        scheduledForEpochMs,
        Intl.DateTimeFormat().resolvedOptions().timeZone,
      );
      setScheduleOpen(false);
      setScheduleValue("");
      onQueueChanged();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setPublishing(false);
    }
  };
  return (
    <div className="page composer-page">
      <div className="composer-grid">
        <div className="editor-column">
          <header className="composer-heading">
            <h1 id="editor-heading">Composer</h1>
            <p>Write a post and share it across your networks.</p>
          </header>
          <DestinationSelector
            workspace={workspace}
            disabled={publishing || refreshing}
            onAccounts={onAccounts}
          />
          <section
            className="panel editor-panel"
            aria-labelledby="editor-heading"
          >
            <label className="sr-only" htmlFor="post-text">
              Post text
            </label>
            <textarea
              ref={editor}
              id="post-text"
              disabled={publishing}
              value={text}
              onSelect={(event) =>
                setSelection({
                  start: event.currentTarget.selectionStart,
                  end: event.currentTarget.selectionEnd,
                })
              }
              onCompositionStart={() => setComposing(true)}
              onCompositionEnd={() => setComposing(false)}
              onChange={(event) => {
                setText(event.target.value);
                setSelection({
                  start: event.target.selectionStart,
                  end: event.target.selectionEnd,
                });
                setDismissedTag(null);
                if (succeeded) setResult(null);
              }}
              placeholder="What’s on your mind?"
              spellCheck
            />
            {activeTag && (
              <HashtagSuggestions
                key={activeTag.start}
                editor={editor}
                text={text}
                anchorIndex={activeTag.start}
                query={activeTag.query}
                accountIds={selected}
                workspace={workspace}
                onChoose={chooseTag}
                onClose={() => setDismissedTag(tagKey)}
              />
            )}
            <ImageAttachments
              media={media}
              accounts={destinations}
              disabled={publishing}
              onReading={setReadingImages}
              counter={<LimitMonitor accounts={destinations} count={count} />}
              onChange={(images) => {
                setMedia(images);
                if (succeeded) setResult(null);
              }}
            />
          </section>
          <section className="panel policy-panel">
            <fieldset disabled={publishing}>
              <legend>
                Publishing policy <Info size={16} aria-hidden="true" />
              </legend>
              <div className="policies">
                {policies.map((item) => (
                  <label
                    key={item.value}
                    className={`policy ${policy === item.value ? "selected" : ""}`}
                  >
                    <input
                      type="radio"
                      name="publishing-policy"
                      checked={policy === item.value}
                      onChange={() => setPolicy(item.value)}
                    />
                    <span>
                      <strong>{item.label}</strong>
                      <small>{item.detail}</small>
                    </span>
                  </label>
                ))}
              </div>
            </fieldset>
          </section>
          {policy === "COMMON_LIMIT" && preview?.effectiveLimit && (
            <Notice>
              Common limit: {preview.effectiveLimit} graphemes, set by{" "}
              {
                accounts.find(
                  (account) => account.id === preview.limitingAccountId,
                )?.handle
              }
              .
            </Notice>
          )}
          {!native && (
            <Notice>
              Preview mode. Open the desktop app to calculate native threads and
              publish to your accounts.
            </Notice>
          )}
          {native && disconnected && (
            <Notice error>
              Some selected accounts need reconnecting.{" "}
              <button className="button" onClick={onAccounts}>
                Manage accounts
              </button>
            </Notice>
          )}
          {planning && <Notice>Planning native threads…</Notice>}
          {previewError && (
            <Notice error>
              <span>{previewError}</span>
              <button className="button" onClick={retry}>
                Retry preview
              </button>
            </Notice>
          )}
          {error && <Notice error>{error}</Notice>}

          {result && (
            <Notice error={!succeeded}>
              <strong>
                {succeeded
                  ? "Published successfully"
                  : uncertain
                    ? "Publication result uncertain"
                    : receivedPosts
                      ? "Some posts were published"
                      : "Publishing failed"}
              </strong>
              <ul className="publication-results">
                {result.destinations.map((publication) => (
                  <li key={publication.accountId}>
                    <span>
                      {accounts.find(
                        (account) => account.id === publication.accountId,
                      )?.handle ?? publication.accountId}
                    </span>
                    <span>
                      {publication.status.replaceAll("_", " ")}
                      {publication.error ? `: ${publication.error}` : ""}
                    </span>
                  </li>
                ))}
              </ul>
              {uncertain && (
                <p>
                  Do not retry automatically. Check the provider for the post
                  first.
                </p>
              )}
              {!succeeded && receivedPosts && (
                <p>
                  Your draft was kept. Destinations that received posts have
                  been deselected. Check any partially published threads before
                  sending again.
                </p>
              )}
            </Notice>
          )}
          {scheduleOpen && (
            <section className="panel schedule-editor">
              <label>
                Schedule on this device (
                {Intl.DateTimeFormat().resolvedOptions().timeZone})
                <input
                  type="datetime-local"
                  value={scheduleValue}
                  onChange={(event) => setScheduleValue(event.target.value)}
                />
              </label>
              <p className="muted">
                Threadline flags missed times for attention and never silently
                sends them late.
              </p>
              <div className="heading-actions">
                <button
                  className="button"
                  onClick={() => setScheduleOpen(false)}
                >
                  Cancel
                </button>
                <button
                  className="button button-blue"
                  disabled={!scheduleValue || !canPublish}
                  onClick={() => void schedule()}
                >
                  Save schedule
                </button>
              </div>
            </section>
          )}
          <div className="dispatch-bar">
            <div role="status" aria-live="polite" aria-atomic="true">
              {succeeded ? (
                <Check size={16} aria-hidden="true" />
              ) : feedback ? (
                <CircleAlert size={16} aria-hidden="true" />
              ) : (
                <span className="status-dot" />
              )}
              <span>
                {publishing
                  ? "Publishing to your destinations…"
                  : feedback
                    ? feedback
                    : !native
                      ? "Desktop app required to publish"
                      : !destinations.length
                        ? "Select a destination"
                        : !text.trim() && !media.length
                          ? "Write your first line or add media"
                          : readingImages
                            ? "Reading media…"
                            : refreshing
                              ? "Refreshing accounts…"
                              : disconnected
                                ? "Reconnect selected accounts"
                                : planning
                                  ? "Planning native threads…"
                                  : previewError
                                    ? "Resolve preview error"
                                    : "Ready to publish to " +
                                      destinations.length +
                                      (destinations.length === 1
                                        ? " account"
                                        : " accounts")}
              </span>
            </div>
            <div className="heading-actions">
              <button
                className="button"
                disabled={
                  !native || publishing || (!text.trim() && media.length === 0)
                }
                onClick={() => void save()}
              >
                <Save size={16} />
                Save draft
              </button>
              <button
                className="button"
                disabled={!canPublish}
                onClick={() => setScheduleOpen(true)}
              >
                <Calendar size={16} />
                Schedule
              </button>
              <button
                className="button button-primary publish-button"
                disabled={!canPublish}
                aria-busy={publishing}
                onClick={() => void publish()}
              >
                <Send size={16} />
                {publishing
                  ? "Publishing…"
                  : `Publish to ${destinations.length} ${destinations.length === 1 ? "account" : "accounts"}`}
              </button>
            </div>
          </div>
        </div>
        <DestinationPreview
          accounts={destinations}
          preview={preview}
          text={text}
          media={media}
          native={native}
        />
      </div>
    </div>
  );
}
