import { Maximize2, Minus, Mouse, MousePointer2, Move, Plus, X } from "lucide-react";
import { useCallback, useEffect, useId, useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import { createPortal } from "react-dom";
import "../../styles/image-viewer.css";

type Point = { readonly x: number; readonly y: number };
type Size = { readonly width: number; readonly height: number };

export type ImageViewerProps = {
  readonly src: string;
  readonly alt: string;
  readonly onClose: () => void;
};

const MIN_ZOOM = 50;
const MAX_ZOOM = 400;
const ZOOM_STEP = 25;
const PAN_STEP = 48;

export function ImageViewer({ src, alt, onClose }: ImageViewerProps) {
  const titleId = useId();
  const dialogRef = useRef<HTMLDivElement>(null);
  const stageRef = useRef<HTMLDivElement>(null);
  const zoomRef = useRef(100);
  const offsetRef = useRef<Point>({ x: 0, y: 0 });
  const dragRef = useRef<{ pointerId: number; x: number; y: number } | null>(null);
  const [zoom, setZoom] = useState(100);
  const [offset, setOffset] = useState<Point>({ x: 0, y: 0 });
  const [stageSize, setStageSize] = useState<Size | null>(null);
  const [imageSize, setImageSize] = useState<Size | null>(null);
  const [loadFailed, setLoadFailed] = useState(false);

  const moveTo = useCallback((next: Point) => {
    offsetRef.current = next;
    setOffset(next);
  }, []);

  const zoomTo = useCallback((requested: number, anchor: Point = { x: 0, y: 0 }) => {
    const next = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, requested));
    const previous = zoomRef.current;
    if (next === previous) return;
    const ratio = next / previous;
    moveTo({
      x: anchor.x - ratio * (anchor.x - offsetRef.current.x),
      y: anchor.y - ratio * (anchor.y - offsetRef.current.y),
    });
    zoomRef.current = next;
    setZoom(next);
  }, [moveTo]);

  useEffect(() => {
    const priorFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const priorOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    dialogRef.current?.focus();
    return () => {
      document.body.style.overflow = priorOverflow;
      priorFocus?.focus();
    };
  }, []);

  useEffect(() => {
    const stage = stageRef.current;
    if (!stage) return;
    const measure = () => setStageSize({ width: stage.clientWidth, height: stage.clientHeight });
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(stage);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    zoomRef.current = 100;
    offsetRef.current = { x: 0, y: 0 };
    setZoom(100);
    setOffset({ x: 0, y: 0 });
    setImageSize(null);
    setLoadFailed(false);
  }, [src]);

  useEffect(() => {
    const stage = stageRef.current;
    if (!stage) return;
    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      if (!imageSize || event.deltaY === 0) return;
      const bounds = stage.getBoundingClientRect();
      zoomTo(zoomRef.current + (event.deltaY < 0 ? ZOOM_STEP : -ZOOM_STEP), {
        x: event.clientX - bounds.left - bounds.width / 2,
        y: event.clientY - bounds.top - bounds.height / 2,
      });
    };
    stage.addEventListener("wheel", onWheel, { passive: false });
    return () => stage.removeEventListener("wheel", onWheel);
  }, [imageSize, zoomTo]);

  const onPointerDown = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (event.button !== 0 || !imageSize) return;
    event.currentTarget.focus();
    event.currentTarget.setPointerCapture(event.pointerId);
    dragRef.current = { pointerId: event.pointerId, x: event.clientX, y: event.clientY };
  };

  const onPointerMove = (event: ReactPointerEvent<HTMLDivElement>) => {
    const drag = dragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    moveTo({ x: offsetRef.current.x + event.clientX - drag.x, y: offsetRef.current.y + event.clientY - drag.y });
    dragRef.current = { pointerId: drag.pointerId, x: event.clientX, y: event.clientY };
  };

  const stopDragging = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (dragRef.current?.pointerId !== event.pointerId) return;
    dragRef.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
  };

  const onKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
      return;
    }
    if (event.key === "Tab") {
      const controls = Array.from(dialogRef.current?.querySelectorAll<HTMLElement>("button:not([disabled]), input:not([disabled])") ?? []);
      const first = controls[0];
      const last = controls[controls.length - 1];
      if (event.shiftKey && (document.activeElement === first || document.activeElement === dialogRef.current)) {
        event.preventDefault();
        last?.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first?.focus();
      }
      return;
    }
    if (event.altKey || event.ctrlKey || event.metaKey) return;
    if (event.code === "NumpadAdd" || event.key === "+" || event.key === "=") {
      event.preventDefault();
      zoomTo(zoomRef.current + ZOOM_STEP);
      return;
    }
    if (event.code === "NumpadSubtract" || event.key === "-") {
      event.preventDefault();
      zoomTo(zoomRef.current - ZOOM_STEP);
      return;
    }
    const direction: Record<string, Point> = {
      ArrowLeft: { x: -1, y: 0 },
      ArrowRight: { x: 1, y: 0 },
      ArrowUp: { x: 0, y: -1 },
      ArrowDown: { x: 0, y: 1 },
    };
    const delta = direction[event.key];
    if (delta && imageSize) {
      event.preventDefault();
      const distance = PAN_STEP * (event.shiftKey ? 2 : 1);
      moveTo({ x: offsetRef.current.x + delta.x * distance, y: offsetRef.current.y + delta.y * distance });
    }
  };

  const fit = imageSize && stageSize
    ? Math.min(stageSize.width * 0.9 / imageSize.width, stageSize.height * 0.9 / imageSize.height)
    : null;

  return createPortal(
    <div className="image-viewer">
      <div ref={dialogRef} className="image-viewer__dialog" role="dialog" aria-modal="true" aria-labelledby={titleId} tabIndex={-1} onKeyDown={onKeyDown}>
        <header className="image-viewer__header">
          <div className="image-viewer__heading"><h2 id={titleId}>Image viewer</h2>{alt && <p title={alt}>{alt}</p>}</div>
          <button type="button" className="image-viewer__button" aria-label="Close image viewer" onClick={onClose}><X size={20} aria-hidden="true" /></button>
        </header>
        <div ref={stageRef} className="image-viewer__stage" role="group" tabIndex={0} aria-label="Image canvas. Drag to move, use arrow keys to pan, and scroll to zoom." onPointerDown={onPointerDown} onPointerMove={onPointerMove} onPointerUp={stopDragging} onPointerCancel={stopDragging}>
          {loadFailed && <p className="image-viewer__message" role="alert">Could not load this image.</p>}
          {!imageSize && !loadFailed && <p className="image-viewer__message" role="status">Loading image…</p>}
          <img src={src} alt={alt} draggable={false} onLoad={event => setImageSize({ width: event.currentTarget.naturalWidth, height: event.currentTarget.naturalHeight })} onError={() => setLoadFailed(true)} style={{
            width: fit && imageSize ? imageSize.width * fit : undefined,
            height: fit && imageSize ? imageSize.height * fit : undefined,
            transform: `translate(-50%, -50%) translate(${offset.x}px, ${offset.y}px) scale(${zoom / 100})`,
            visibility: fit ? "visible" : "hidden",
          }} />
        </div>
        <div className="image-viewer__toolbar" aria-label="Image zoom controls">
          <button type="button" className="image-viewer__button" aria-label="Zoom out" disabled={zoom <= MIN_ZOOM} onClick={() => zoomTo(zoomRef.current - ZOOM_STEP)}><Minus size={20} aria-hidden="true" /></button>
          <input type="range" min={MIN_ZOOM} max={MAX_ZOOM} step={5} value={zoom} aria-label="Zoom level" aria-valuetext={`${zoom}%`} onChange={event => zoomTo(Number(event.target.value))} />
          <button type="button" className="image-viewer__button" aria-label="Zoom in" disabled={zoom >= MAX_ZOOM} onClick={() => zoomTo(zoomRef.current + ZOOM_STEP)}><Plus size={20} aria-hidden="true" /></button>
          <output className="image-viewer__zoom">{zoom}%</output>
          <button type="button" className="image-viewer__button image-viewer__fit" aria-label="Fit image to view" onClick={() => { zoomTo(100); moveTo({ x: 0, y: 0 }); }}><Maximize2 size={18} aria-hidden="true" /><span>Fit</span></button>
          <div className="image-viewer__hint" aria-label="Scroll or use plus and minus keys to zoom. Drag or use arrow keys to move the image.">
            <span className="image-viewer__shortcut"><Mouse size={16} aria-hidden="true" />Scroll to zoom</span>
            <span className="image-viewer__shortcut"><kbd><Plus size={13} aria-hidden="true" /><span className="sr-only">Plus</span></kbd><kbd><Minus size={13} aria-hidden="true" /><span className="sr-only">Minus</span></kbd>Zoom keys</span>
            <span className="image-viewer__shortcut"><MousePointer2 size={16} aria-hidden="true" />Drag to move</span>
            <span className="image-viewer__shortcut"><Move size={16} aria-hidden="true" />Arrow keys to move</span>
          </div>
        </div>
      </div>
    </div>,
    document.body,
  );
}
