import { ArrowLeft } from "lucide-react";

/** Shows the origin and label for a detail route with its back action. */
export function DetailHeader({ origin, label, onBack }: { readonly origin: string; readonly label: string; readonly onBack: () => void }) {
  return <div className="crumb"><button className="quiet" onClick={onBack}><ArrowLeft size={15} />Back</button><span>{origin} / {label}</span></div>;
}
