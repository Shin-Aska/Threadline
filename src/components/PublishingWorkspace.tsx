import { Calendar, Clock, FileText, Feather, Send, Trash2 } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { publishingApi } from "../services/desktop/publishing";
import { draftWriter } from "../services/publishing/draftWriter";
import { useComposerStore } from "../stores/composer";
import type { WorkspaceState } from "../types";
import type {
  DraftRecord,
  PublicationRecord,
  ScheduledPublication,
} from "../types/publishing";
import { ComposerPanel } from "./ComposerPanel";
import { Notice } from "./ui";

type PublishingTab = "NEW" | "DRAFTS" | "SCHEDULED" | "PUBLISHED";
interface PublishingProps {
  readonly workspace: WorkspaceState;
  readonly refreshing: boolean;
  readonly onAccounts: () => void;
}
const tabs: readonly {
  readonly id: PublishingTab;
  readonly label: string;
  readonly icon: typeof Feather;
}[] = [
  { id: "NEW", label: "New post", icon: Feather },
  { id: "DRAFTS", label: "Drafts", icon: FileText },
  { id: "SCHEDULED", label: "Scheduled", icon: Calendar },
  { id: "PUBLISHED", label: "Published", icon: Clock },
];

export function PublishingWorkspace(props: PublishingProps) {
  const [tab, setTab] = useState<PublishingTab>("NEW");
  const [drafts, setDrafts] = useState<readonly DraftRecord[]>([]);
  const [schedules, setSchedules] = useState<readonly ScheduledPublication[]>(
    [],
  );
  const [history, setHistory] = useState<readonly PublicationRecord[]>([]);
  const [activeDraft, setActiveDraft] = useState<DraftRecord | null>(null);
  const [selectedSchedule, setSelectedSchedule] =
    useState<ScheduledPublication | null>(null);
  const [scheduleValue, setScheduleValue] = useState("");
  const [ready, setReady] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const activeRef = useRef<DraftRecord | null>(null);
  const { text, media, policy, selected, loadDraft, clearDraft } =
    useComposerStore();
  const post = useMemo(
    () => ({ text, media, policy, destinationAccountIds: selected }),
    [text, media, policy, selected],
  );
  const refresh = useCallback(async () => {
    if (props.workspace.mode === "BROWSER") {
      setReady(true);
      return;
    }
    try {
      const [nextDrafts, nextSchedules, nextHistory] = await Promise.all([
        publishingApi.drafts.list(),
        publishingApi.schedules.list(),
        publishingApi.history(),
      ]);
      setDrafts(nextDrafts);
      setSchedules(nextSchedules);
      setHistory(nextHistory);
      return nextDrafts;
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setReady(true);
    }
  }, [props.workspace.mode]);
  useEffect(() => {
    let current = true;
    void refresh().then((nextDrafts) => {
      if (
        !current ||
        !nextDrafts ||
        text ||
        media.length > 0 ||
        activeRef.current
      )
        return;
      const latest = [...nextDrafts].sort(
        (left, right) => right.updatedAtEpochMs - left.updatedAtEpochMs,
      )[0];
      if (latest) {
        activeRef.current = latest;
        draftWriter.seed(latest);
        setActiveDraft(latest);
        loadDraft(latest.post);
      }
    });
    return () => {
      current = false;
    };
  }, [refresh, loadDraft, media.length, text]);
  useEffect(() => {
    activeRef.current = activeDraft;
  }, [activeDraft]);
  const persistDraft = useCallback(
    (nextPost: typeof post): Promise<DraftRecord> => {
      const operation = draftWriter
        .save(nextPost, activeRef.current)
        .then((saved) => {
          activeRef.current = saved;
          setActiveDraft(saved);
          setDrafts((items) => [
            saved,
            ...items.filter((item) => item.id !== saved.id),
          ]);
          return saved;
        });
      void operation.catch((cause) => {
        setError(cause instanceof Error ? cause.message : String(cause));
      });
      return operation;
    },
    [],
  );
  const postKey = JSON.stringify(post);
  useEffect(() => {
    if (
      !ready ||
      props.workspace.mode === "BROWSER" ||
      (!text.trim() && media.length === 0 && !activeRef.current)
    )
      return;
    const timer = window.setTimeout(() => {
      void persistDraft(post);
    }, 700);
    return () => window.clearTimeout(timer);
  }, [postKey, ready, props.workspace.mode, persistDraft, post, media.length, text]);
  const openDraft = (draft: DraftRecord) => {
    activeRef.current = draft;
    draftWriter.seed(draft);
    setActiveDraft(draft);
    loadDraft(draft.post);
    setTab("NEW");
  };
  const newDraft = () => {
    activeRef.current = null;
    draftWriter.seed(null);
    setActiveDraft(null);
    clearDraft();
    setTab("NEW");
  };
  const updateSchedule = async (action: "SEND" | "CANCEL" | "RESCHEDULE") => {
    if (!selectedSchedule) return;
    setError(null);
    try {
      const updated =
        action === "SEND"
          ? await publishingApi.schedules.sendNow(
              selectedSchedule.id,
              selectedSchedule.revision,
            )
          : action === "CANCEL"
            ? await publishingApi.schedules.cancel(
                selectedSchedule.id,
                selectedSchedule.revision,
              )
            : await publishingApi.schedules.reschedule(
                selectedSchedule.id,
                selectedSchedule.revision,
                new Date(scheduleValue).getTime(),
                Intl.DateTimeFormat().resolvedOptions().timeZone,
              );
      setSelectedSchedule(updated);
      await refresh();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  };
  return (
    <div className="publishing-workspace">
      <div
        className="publishing-tabs"
        role="tablist"
        aria-label="Composer queues"
      >
        {tabs.map((item) => (
          <button
            role="tab"
            aria-selected={tab === item.id}
            className={`nav-item ${tab === item.id ? "active" : ""}`}
            key={item.id}
            onClick={() => setTab(item.id)}
          >
            <item.icon size={17} />
            {item.label}
            {item.id === "DRAFTS" && <small>{drafts.length}</small>}
            {item.id === "SCHEDULED" && <small>{schedules.length}</small>}
          </button>
        ))}
      </div>
      {error && (
        <div className="page">
          <Notice error>{error}</Notice>
        </div>
      )}
      {tab === "NEW" && (
        <ComposerPanel
          workspace={props.workspace}
          refreshing={props.refreshing || !ready}
          onAccounts={props.onAccounts}
          draft={activeDraft}
          onDraftSaved={(draft) => {
            activeRef.current = draft;
            setActiveDraft(draft);
          }}
          onQueueChanged={() => void refresh()}
        />
      )}
      {tab === "DRAFTS" && (
        <div className="page queue-page">
          <header className="unified-header">
            <div className="unified-title">
              <FileText />
              <div>
                <h1>Drafts</h1>
                <p>Posts saved on this device.</p>
              </div>
            </div>
            <button className="button button-blue" onClick={newDraft}>
              New post
            </button>
          </header>
          <section className="panel queue-list">
            {drafts.map((draft) => (
              <div className="queue-row" key={draft.id}>
                <button
                  className="quiet queue-copy"
                  onClick={() => openDraft(draft)}
                >
                  <strong>
                    {draft.post.text ||
                      `${draft.post.media.length} media attachment(s)`}
                  </strong>
                  <small>
                    Updated {new Date(draft.updatedAtEpochMs).toLocaleString()}{" "}
                    · {draft.post.destinationAccountIds.length} destinations
                  </small>
                </button>
                <span className="status-pill">Revision {draft.revision}</span>
                <button
                  className="button"
                  aria-label={`Delete draft ${draft.post.text.slice(0, 30)}`}
                  onClick={() =>
                    void publishingApi.drafts
                      .delete(draft.id)
                      .then(refresh)
                      .catch((cause) =>
                        setError(
                          cause instanceof Error
                            ? cause.message
                            : String(cause),
                        ),
                      )
                  }
                >
                  <Trash2 size={15} />
                  Delete
                </button>
              </div>
            ))}
            {drafts.length === 0 && (
              <div className="empty-box">No saved drafts.</div>
            )}
          </section>
        </div>
      )}
      {tab === "SCHEDULED" && (
        <div className="page queue-page">
          <header className="unified-header">
            <div className="unified-title">
              <Calendar />
              <div>
                <h1>Scheduled</h1>
                <p>
                  Local schedules that require this device and Threadline to be
                  running.
                </p>
              </div>
            </div>
          </header>
          <div className="unified-layout">
            <section className="panel queue-list">
              {schedules.map((schedule) => (
                <button
                  className={`queue-row click-row ${selectedSchedule?.id === schedule.id ? "selected" : ""}`}
                  key={schedule.id}
                  onClick={() => {
                    setSelectedSchedule(schedule);
                    setScheduleValue(
                      new Date(schedule.scheduledForEpochMs)
                        .toISOString()
                        .slice(0, 16),
                    );
                  }}
                >
                  <span>
                    <strong>
                      {schedule.post.text || `${schedule.post.media.length} media attachment(s)`}
                    </strong>
                    <small>
                      {new Date(schedule.scheduledForEpochMs).toLocaleString()}{" "}
                      · {schedule.timeZone}
                    </small>
                  </span>
                  <span
                    className={`status-pill ${schedule.status === "NEEDS_ATTENTION" ? "warning" : ""}`}
                  >
                    {schedule.status.replaceAll("_", " ")}
                  </span>
                </button>
              ))}
              {schedules.length === 0 && (
                <div className="empty-box">No scheduled posts.</div>
              )}
            </section>
            <aside className="panel queue-details">
              {selectedSchedule ? (
                <>
                  <h2>Schedule details</h2>
                  {selectedSchedule.attentionReason && (
                    <Notice error>
                      <strong>Needs attention</strong>
                      <span>{selectedSchedule.attentionReason}</span>
                    </Notice>
                  )}
                  <label>
                    New time on this device (
                    {Intl.DateTimeFormat().resolvedOptions().timeZone})
                    <input
                      type="datetime-local"
                      value={scheduleValue}
                      onChange={(event) => setScheduleValue(event.target.value)}
                    />
                  </label>
                  <button
                    className="button"
                    disabled={!scheduleValue}
                    onClick={() => void updateSchedule("RESCHEDULE")}
                  >
                    Reschedule
                  </button>
                  <button
                    className="button button-blue"
                    onClick={() => void updateSchedule("SEND")}
                  >
                    <Send size={15} />
                    Send now
                  </button>
                  <button
                    className="button"
                    onClick={() => void updateSchedule("CANCEL")}
                  >
                    Cancel schedule
                  </button>
                </>
              ) : (
                <p className="aside-copy">
                  Select a schedule to reschedule, send now, or cancel it.
                </p>
              )}
            </aside>
          </div>
        </div>
      )}
      {tab === "PUBLISHED" && (
        <div className="page queue-page">
          <header className="unified-header">
            <div className="unified-title">
              <Clock />
              <div>
                <h1>Published</h1>
                <p>
                  Durable publication results, including uncertain outcomes.
                </p>
              </div>
            </div>
          </header>
          <section className="panel queue-list">
            {history.map((publication) => (
              <div className="queue-row" key={publication.id}>
                <span>
                  <strong>
                    {new Date(publication.createdAtEpochMs).toLocaleString()}
                  </strong>
                  <small>
                    {publication.destinations.length} destination results
                  </small>
                </span>
                <span>
                  {publication.destinations
                    .map((item) => item.status)
                    .join(" · ")}
                </span>
                <span
                  className={`status-pill ${publication.destinations.some((item) => item.status === "UNCERTAIN") ? "warning" : ""}`}
                >
                  {publication.destinations.some(
                    (item) => item.status === "UNCERTAIN",
                  )
                    ? "Check provider before retry"
                    : "Recorded"}
                </span>
              </div>
            ))}
            {history.length === 0 && (
              <div className="empty-box">No publication history yet.</div>
            )}
          </section>
        </div>
      )}
    </div>
  );
}
