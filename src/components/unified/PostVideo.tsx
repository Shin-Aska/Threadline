import { useEffect, useRef, useState } from "react";
import { ExternalLink } from "./ExternalLinks";

interface PostVideoProps {
  readonly url: string;
  readonly alt: string;
  readonly thumbnail?: string | null;
  readonly mediaType: string;
  readonly originalUrl: string;
}

/** Plays provider video or HLS media and exposes the original post as fallback. */
export function PostVideo({ url, alt, thumbnail, mediaType, originalUrl }: PostVideoProps) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const [unsupported, setUnsupported] = useState(false);

  useEffect(() => {
    if (mediaType !== "video/hls" || unsupported) return;
    let cancelled = false;
    let player: { destroy: () => void } | null = null;
    void import("hls.js").then(({ default: Hls }) => {
      const video = videoRef.current;
      if (cancelled || !video) return;
      if (Hls.isSupported()) {
        const hls = new Hls();
        player = hls;
        hls.on(Hls.Events.ERROR, (_event, data) => { if (data.fatal) setUnsupported(true); });
        hls.loadSource(url);
        hls.attachMedia(video);
      } else if (video.canPlayType("application/vnd.apple.mpegurl")) {
        video.src = url;
      } else {
        setUnsupported(true);
      }
    }).catch(() => { if (!cancelled) setUnsupported(true); });
    return () => { cancelled = true; player?.destroy(); };
  }, [mediaType, unsupported, url]);

  if (unsupported) return <ExternalLink className="post-video-fallback" href={originalUrl}>Video playback unavailable. Open original post</ExternalLink>;
  return <video ref={videoRef} src={mediaType === "video/hls" ? undefined : url} poster={thumbnail ?? undefined} controls preload="metadata" aria-label={alt || "Post video"} onError={() => setUnsupported(true)} />;
}
