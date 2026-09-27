import { controlSource } from "../controlSources";
import { Plus, Plug } from "lucide-react";
import { sourceVectors } from "../assets/brand-reserve/control-sources";
import { Icon } from "./ui";
export default function SourceIcon({
  platform,
  add = false,
  panelVisual = false,
}: {
  platform: string;
  add?: boolean;
  panelVisual?: boolean;
}) {
  const raw = sourceVectors[controlSource(platform)?.icon || platform];
  return (
    <span className="source-icon" data-panel-visual={panelVisual || undefined} aria-hidden="true">
      {add ? (
        <Icon icon={Plus} size={20} />
      ) : raw ? (
        <span dangerouslySetInnerHTML={{ __html: raw }} />
      ) : (
        <Icon icon={Plug} size={20} />
      )}
    </span>
  );
}
