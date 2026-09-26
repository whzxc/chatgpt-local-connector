import { createStore } from "../state/store";
import { api } from "../api";
import type { PanelPreferences } from "./layout";

export const panelPreferences = createStore<PanelPreferences>({
  visible: true,
  autoCollapse: true,
  notchFusion: true,
  size: "standard",
  spacing: "standard",
  horizontalPercentages: false,
  alertColor: true,
  warningAt: 75,
  placement: { dock: "right", display: "", x: 1, y: 0.5 },
});
export async function readPanelPreferences() {
  const result = await api<{ preferences: PanelPreferences }>("usage-panel");
  panelPreferences.set(result.preferences);
}
export async function savePanelPreferences(
  patch: Partial<PanelPreferences> | { resetDefaults: true },
) {
  const result = await api<{ preferences: PanelPreferences }>(
    "usage-panel",
    "PUT",
    patch,
  );
  panelPreferences.set(result.preferences);
}
