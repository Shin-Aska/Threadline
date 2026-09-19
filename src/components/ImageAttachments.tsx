import { useRef, useState } from "react";
import type { ReactNode } from "react";
import { ImagePlus, X } from "lucide-react";
import type { MediaAttachment } from "../types";
import { Notice } from "./ui";

interface ImageAttachmentsProps {
  readonly media: MediaAttachment[];
  readonly disabled: boolean;
  readonly onChange: (media: MediaAttachment[]) => void;
  readonly onReading: (reading: boolean) => void;
  readonly counter: ReactNode;
}
const acceptedTypes = ["image/jpeg", "image/png", "image/webp"];
function readImage(file: File): Promise<MediaAttachment> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject(new Error(`Could not read ${file.name}. Try choosing it again.`));
    reader.onload = () => {
      if (typeof reader.result !== "string") { reject(new Error("Could not read image.")); return; }
      const url = reader.result;
      const image = new Image();
      image.onerror = () => reject(new Error(`${file.name} could not be opened as an image.`));
      image.onload = () => resolve({ id: crypto.randomUUID(), name: file.name, mimeType: file.type, sizeBytes: file.size, altText: "", dataBase64: url.slice(url.indexOf(",") + 1) });
      image.src = url;
    };
    reader.readAsDataURL(file);
  });
}
export function ImageAttachments({ media, disabled, onChange, onReading, counter }: ImageAttachmentsProps) {
  const input = useRef<HTMLInputElement>(null);
  const pending = useRef(false);
  const [reading, setReading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const locked = disabled || reading;
  async function add(files: File[]) {
    if (!files.length || disabled || pending.current) return;
    setError(null);
    if (media.length + files.length > 4) { setError("Choose up to four images per post."); return; }
    const invalid = files.find(file => !acceptedTypes.includes(file.type) || file.size === 0 || file.size > 2_000_000);
    if (invalid) { setError(`${invalid.name}: use a JPEG, PNG, or WebP image up to 2 MB.`); return; }
    pending.current = true; setReading(true); onReading(true);
    try { onChange([...media, ...await Promise.all(files.map(readImage))]); }
    catch (cause) { setError(cause instanceof Error ? cause.message : "Could not read images."); }
    finally { pending.current = false; setReading(false); onReading(false); }
  }
  return <section className="panel images-panel" aria-labelledby="images-heading">
    <h2 id="images-heading" className="sr-only">Images</h2>
    <div className="attachment-toolbar"><button type="button" className="button" title="JPEG, PNG, or WebP · up to 2 MB each" disabled={locked || media.length === 4} onClick={() => input.current?.click()}><ImagePlus size={20} />Add images</button><span className="attachment-count" aria-label={`${media.length} of 4 images`}>{media.length} / 4</span>{counter}</div>
    <input ref={input} className="sr-only" type="file" tabIndex={-1} aria-label="Choose images" accept={acceptedTypes.join(",")} multiple disabled={locked} onChange={event => { const files = Array.from(event.target.files ?? []); event.target.value = ""; void add(files); }} />
    {media.length > 0 && <p className="muted media-help">JPEG, PNG, or WebP · up to 2 MB each. Images attach to the first post of a thread.</p>}
    {reading && <p role="status">Reading images…</p>}
    {error && <Notice error>{error}</Notice>}
    <div className="attachment-list">{media.map((item, index) => <div className="attachment-card" key={item.id}>
      <img className="attachment-thumbnail" src={`data:${item.mimeType};base64,${item.dataBase64}`} alt={item.altText || `Attached image ${index + 1}`} />
      <div className="attachment-details"><div className="attachment-title"><strong title={item.name}>{item.name}</strong><button type="button" className="button" aria-label={`Remove ${item.name}`} disabled={locked} onClick={() => { onChange(media.filter(image => image.id !== item.id)); setError(null); }}><X size={16} /></button></div>
        <label htmlFor={`alt-${item.id}`}>Alt text for image {index + 1}</label>
        <textarea id={`alt-${item.id}`} className="media-alt" value={item.altText} disabled={locked} placeholder="Describe what’s in this image…" onChange={event => onChange(media.map(image => image.id === item.id ? { ...image, altText: [...event.target.value].slice(0, 1500).join("") } : image))} aria-describedby={`alt-count-${item.id}`} />
        <span className="muted alt-count" id={`alt-count-${item.id}`}>{[...item.altText].length}/1,500 · Helps people using screen readers</span>
      </div>
    </div>)}</div>
  </section>;
}
