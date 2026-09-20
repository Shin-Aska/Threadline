import { useRef, useState } from "react";
import type { ReactNode } from "react";
import { ImagePlus, Video, X } from "lucide-react";
import type { Account, MediaAttachment } from "../types";
import { Notice } from "./ui";

interface ImageAttachmentsProps {
  readonly media: MediaAttachment[];
  readonly accounts: readonly Account[];
  readonly disabled: boolean;
  readonly onChange: (media: MediaAttachment[]) => void;
  readonly onReading: (reading: boolean) => void;
  readonly counter: ReactNode;
}
const acceptedTypes = ["image/jpeg", "image/png", "image/webp"];
const acceptedVideoType = "video/mp4";
function videoDuration(file: File): Promise<number> {
  return new Promise((resolve, reject) => {
    const url = URL.createObjectURL(file);
    const video = document.createElement("video");
    const finish = () => URL.revokeObjectURL(url);
    video.onerror = () => { finish(); reject(new Error(`${file.name} could not be opened as an MP4 video.`)); };
    video.onloadedmetadata = () => {
      const durationMs = Math.round(video.duration * 1_000);
      finish();
      if (!Number.isFinite(durationMs) || durationMs <= 0) { reject(new Error(`${file.name} has no playable duration.`)); return; }
      resolve(durationMs);
    };
    video.preload = "metadata";
    video.src = url;
  });
}
function validateImage(file: File): Promise<void> {
  return new Promise((resolve, reject) => {
    const url = URL.createObjectURL(file);
    const image = new Image();
    const finish = () => URL.revokeObjectURL(url);
    image.onerror = () => { finish(); reject(new Error(`${file.name} could not be opened as an image.`)); };
    image.onload = () => { finish(); resolve(); };
    image.src = url;
  });
}
async function readMedia(file: File): Promise<MediaAttachment> {
  const durationMs = file.type === acceptedVideoType ? await videoDuration(file) : (await validateImage(file), null);
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject(new Error(`Could not read ${file.name}. Try choosing it again.`));
    reader.onload = () => {
      if (typeof reader.result !== "string") { reject(new Error("Could not read media.")); return; }
      const url = reader.result;
      resolve({ id: crypto.randomUUID(), name: file.name, mimeType: file.type, sizeBytes: file.size, altText: "", dataBase64: url.slice(url.indexOf(",") + 1), durationMs });
    };
    reader.readAsDataURL(file);
  });
}
export function ImageAttachments({ media, accounts, disabled, onChange, onReading, counter }: ImageAttachmentsProps) {
  const imageInput = useRef<HTMLInputElement>(null);
  const videoInput = useRef<HTMLInputElement>(null);
  const pending = useRef(false);
  const [reading, setReading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const locked = disabled || reading;
  const hasVideo = media.some(item => item.mimeType === acceptedVideoType);
  async function add(files: File[]) {
    if (!files.length || disabled || pending.current) return;
    setError(null);
    const addsVideo = files.some(file => file.type === acceptedVideoType);
    if (addsVideo) {
      if (files.length !== 1 || media.length > 0) { setError("Attach either one video or up to four images."); return; }
      const file = files[0];
      if (!file || file.size === 0) { setError("Choose a non-empty MP4 video."); return; }
      const unsupported = accounts.find(account => !account.capabilities.supportedMediaTypes.includes(acceptedVideoType) || account.capabilities.maxVideoBytes == null);
      if (unsupported) { setError(`${unsupported.handle}: video limits are unavailable. Reconnect this account before adding video.`); return; }
      const maxBytes = accounts.reduce<number | null>((limit, account) => account.capabilities.maxVideoBytes == null ? limit : Math.min(limit ?? account.capabilities.maxVideoBytes, account.capabilities.maxVideoBytes), null);
      if (maxBytes !== null && file.size > maxBytes) { setError(`${file.name} exceeds the selected destinations’ ${maxBytes.toLocaleString()} byte video limit.`); return; }
    } else {
      if (hasVideo) { setError("Remove the video before adding images."); return; }
      if (media.length + files.length > 4) { setError("Choose up to four images per post."); return; }
      const invalid = files.find(file => !acceptedTypes.includes(file.type) || file.size === 0 || file.size > 2_000_000);
      if (invalid) { setError(`${invalid.name}: use a JPEG, PNG, or WebP image up to 2 MB.`); return; }
    }
    pending.current = true; setReading(true); onReading(true);
    try {
      const next = await Promise.all(files.map(readMedia));
      const maxDurationMs = accounts.reduce<number | null>((limit, account) => account.capabilities.maxVideoDurationMs == null ? limit : Math.min(limit ?? account.capabilities.maxVideoDurationMs, account.capabilities.maxVideoDurationMs), null);
      const tooLong = next.find(item => item.durationMs != null && maxDurationMs !== null && item.durationMs > maxDurationMs);
      if (tooLong) { setError(`${tooLong.name} exceeds the selected destinations’ video duration limit.`); return; }
      onChange([...media, ...next]);
    }
    catch (cause) { setError(cause instanceof Error ? cause.message : "Could not read media."); }
    finally { pending.current = false; setReading(false); onReading(false); }
  }
  return <section className="panel images-panel" aria-labelledby="media-heading">
    <h2 id="media-heading" className="sr-only">Media</h2>
    <div className="attachment-toolbar"><button type="button" className="button" title="JPEG, PNG, or WebP · up to 2 MB each" disabled={locked || hasVideo || media.length === 4} onClick={() => imageInput.current?.click()}><ImagePlus size={20} />Add images</button><button type="button" className="button" title="One MP4 video · destination limits apply" disabled={locked || media.length > 0} onClick={() => videoInput.current?.click()}><Video size={20} />Add video</button><span className="attachment-count" aria-label={hasVideo ? "One video attached" : `${media.length} of 4 images`}>{hasVideo ? "1 video" : `${media.length} / 4`}</span>{counter}</div>
    <input ref={imageInput} className="sr-only" type="file" tabIndex={-1} aria-label="Choose images" accept={acceptedTypes.join(",")} multiple disabled={locked} onChange={event => { const files = Array.from(event.target.files ?? []); event.target.value = ""; void add(files); }} />
    <input ref={videoInput} className="sr-only" type="file" tabIndex={-1} aria-label="Choose an MP4 video" accept={acceptedVideoType} disabled={locked} onChange={event => { const files = Array.from(event.target.files ?? []); event.target.value = ""; void add(files); }} />
    {media.length > 0 && <p className="muted media-help">{hasVideo ? "MP4 video · selected destination limits apply." : "JPEG, PNG, or WebP · up to 2 MB each."} Media attaches to the first post of a thread.</p>}
    {reading && <p role="status">Reading media…</p>}
    {error && <Notice error>{error}</Notice>}
    <div className="attachment-list">{media.map((item, index) => <div className="attachment-card" key={item.id}>
      {item.mimeType === acceptedVideoType ? <video className="attachment-thumbnail" src={`data:${item.mimeType};base64,${item.dataBase64}`} aria-label={item.altText || "Attached video"} controls preload="metadata" /> : <img className="attachment-thumbnail" src={`data:${item.mimeType};base64,${item.dataBase64}`} alt={item.altText || `Attached image ${index + 1}`} />}
      <div className="attachment-details"><div className="attachment-title"><strong title={item.name}>{item.name}</strong><button type="button" className="button" aria-label={`Remove ${item.name}`} disabled={locked} onClick={() => { onChange(media.filter(image => image.id !== item.id)); setError(null); }}><X size={16} /></button></div>
        <label htmlFor={`alt-${item.id}`}>Alt text for {item.mimeType === acceptedVideoType ? "video" : `image ${index + 1}`}</label>
        <textarea id={`alt-${item.id}`} className="media-alt" value={item.altText} disabled={locked} placeholder={`Describe what’s in this ${item.mimeType === acceptedVideoType ? "video" : "image"}…`} onChange={event => onChange(media.map(image => image.id === item.id ? { ...image, altText: [...event.target.value].slice(0, 1500).join("") } : image))} aria-describedby={`alt-count-${item.id}`} />
        <span className="muted alt-count" id={`alt-count-${item.id}`}>{[...item.altText].length}/1,500 · Helps people using screen readers</span>
      </div>
    </div>)}</div>
  </section>;
}
