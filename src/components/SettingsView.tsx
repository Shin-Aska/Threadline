import { Info, Settings2 } from "lucide-react";
import { useState } from "react";
import { POST_CACHE_LIMITS, type PostCacheLimit } from "../services/desktop/post-cache-settings";
import { getPostCacheLimit, setPostCacheLimit } from "../services/desktop/social";
import { ExternalLinkWarningSetting } from "./unified/ExternalLinks";
import { ViewHeader } from "./unified/ViewHeader";

/** Presents local application preferences that do not belong to one provider. */
export function SettingsView() {
  const [limit, setLimit] = useState(getPostCacheLimit);
  const [error, setError] = useState<string | null>(null);

  const chooseLimit = (value: PostCacheLimit) => {
    try {
      setPostCacheLimit(value);
      setLimit(value);
      setError(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Could not save the cache preference.");
    }
  };

  return <div className="unified-page settings-page">
    <ViewHeader icon={<Settings2 />} title="Settings" subtitle="Browsing and privacy preferences for this device." controls={null} />
    <div className="settings-stack">
      <section className="panel settings-panel" aria-labelledby="settings-memory-heading">
        <div className="settings-panel-heading"><h2 id="settings-memory-heading">Memory cache</h2><p>Choose how many fetched posts Threadline keeps for quick reloads.</p></div>
        <fieldset className="settings-cache-fieldset">
          <legend>Maximum cached posts</legend>
          <div className="settings-cache-options">
            {POST_CACHE_LIMITS.map(value => <label className="settings-cache-option" key={value}>
              <input type="radio" name="post-cache-limit" value={value} checked={limit === value} onChange={() => chooseLimit(value)} />
              <span>{value === 0 ? "Off" : value}</span>
              <small>{value === 0 ? "No post read cache" : value === 250 ? "Default" : "Posts"}</small>
            </label>)}
          </div>
        </fieldset>
        <p className="settings-help">This limits cached provider responses. Posts already displayed in an open view remain visible while you browse.</p>
        {error && <p className="inline-error" role="alert">{error}</p>}
      </section>
      <section className="panel settings-panel" aria-labelledby="settings-links-heading">
        <div className="settings-panel-heading"><h2 id="settings-links-heading">External links</h2><p>Choose whether Threadline asks before opening a link in your browser.</p></div>
        <ExternalLinkWarningSetting />
      </section>
      <div className="notice settings-privacy-note" role="note"><Info size={18} aria-hidden="true" /><p><strong>Privacy FYI</strong> Fetched posts use a memory-only read cache. Threadline does not save that cache to disk, to limit stored browsing data. Drafts and publishing history are stored separately.</p></div>
    </div>
  </div>;
}
