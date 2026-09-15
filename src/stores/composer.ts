import { create } from "zustand";
import type { MediaAttachment, PublishingPolicy } from "../types";
interface ComposerState {
  publishing: boolean; setPublishing(publishing: boolean): void;
  text: string; media: MediaAttachment[]; policy: PublishingPolicy; selected: string[];
  setText(text: string): void; setMedia(media: MediaAttachment[]): void; clearDraft(): void;
  setPolicy(policy: PublishingPolicy): void; toggle(id: string): void; setSelected(ids: string[]): void;
}
export const useComposerStore = create<ComposerState>((set) => ({
  publishing: false, setPublishing: (publishing) => set({ publishing }),
  text: "", media: [], policy: "COMMON_LIMIT", selected: [],
  setText: (text) => set({ text }), setMedia: (media) => set({ media }),
  clearDraft: () => set({ text: "", media: [] }), setPolicy: (policy) => set({ policy }),
  toggle: (id) => set((state) => ({ selected: state.selected.includes(id) ? state.selected.filter(value => value !== id) : [...state.selected, id] })),
  setSelected: (selected) => set({ selected }),
}));
