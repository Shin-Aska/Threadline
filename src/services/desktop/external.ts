import { invoke } from "@tauri-apps/api/core";

/** Launches a website in the operating system's browser rather than a new WebView. */
export const openExternalUrl = (url: string): Promise<void> => invoke<void>("open_external_url", { url });
