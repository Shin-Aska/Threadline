import { create } from "zustand";
import type { PublishingPolicy } from "../types";
interface ComposerState { text: string; policy: PublishingPolicy; selected: string[]; setText(text:string):void; setPolicy(policy:PublishingPolicy):void; toggle(id:string):void; setSelected(ids:string[]):void }
export const useComposerStore = create<ComposerState>((set) => ({ text:"", policy:"COMMON_LIMIT", selected:[], setText:(text)=>set({text}), setPolicy:(policy)=>set({policy}), toggle:(id)=>set((s)=>({selected:s.selected.includes(id)?s.selected.filter(x=>x!==id):[...s.selected,id]})), setSelected:(selected)=>set({selected}) }));
