import { create } from "zustand";
import type { CanonicalPost, MediaAttachment, PublishingPolicy } from "../types";
interface ComposerState {
  publishing: boolean; setPublishing(publishing: boolean): void;
  text: string; media: MediaAttachment[]; policy: PublishingPolicy; selected: string[];
  setText(text: string): void; setMedia(media: MediaAttachment[]): void; clearDraft(): void;
  loadDraft(post: CanonicalPost): void;
  setPolicy(policy: PublishingPolicy): void; toggle(id: string): void; setSelected(ids: string[]): void;
}
export const useComposerStore = create<ComposerState>((set) => ({
  publishing: false, setPublishing: (publishing) => set({ publishing }),
  text: "", media: [], policy: "COMMON_LIMIT", selected: [],
  setText: (text) => set({ text }), setMedia: (media) => set({ media }),
  clearDraft: () => set({ text: "", media: [] }), setPolicy: (policy) => set({ policy }),
  loadDraft: (post) => set({ text: post.text, media: [...post.media], policy: post.policy, selected: [...post.destinationAccountIds] }),
  toggle: (id) => set((state) => ({ selected: state.selected.includes(id) ? state.selected.filter(value => value !== id) : [...state.selected, id] })),
  setSelected: (selected) => set({ selected }),
}));
