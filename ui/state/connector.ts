import { createStore } from "./store";
import { api } from "../api";
import { t } from "../i18n";
import { isDesktop, notifyNative } from "../platform";
import type { Status } from "./types";
export const connector = createStore<{
  status?: Status;
  busy: string;
  loading: boolean;
  error: string;
  feedback: { text: string; error: boolean };
}>({
  busy: "",
  loading: true,
  error: "",
  feedback: { text: "", error: false },
});
const patch = (next: Partial<ReturnType<typeof connector.get>>) =>
  connector.set((old) => ({ ...old, ...next }));
let feedbackTimer: ReturnType<typeof setTimeout> | undefined;
export function notify(text: string, error = false) {
  clearTimeout(feedbackTimer);
  patch({ feedback: { text, error } });
  if (text && !error)
    feedbackTimer = setTimeout(
      () => patch({ feedback: { text: "", error: false } }),
      4500,
    );
}
let reading: Promise<void> | undefined;
export function refreshConnector(): Promise<void> {
  if (reading) return reading;
  reading = api<Status>("status")
    .then(accept)
    .catch((e) => {
      patch({ error: String(e), loading: false });
      throw e;
    })
    .finally(() => {
      reading = undefined;
    });
  return reading;
}
function accept(status: Status) {
  const previous = connector.get().status?.core.appServer.state;
  if (
    previous &&
    previous !== "error" &&
    status.core.appServer.state === "error"
  )
    void notifyNative(
      t("localConnectionError"),
      t("checkDiagnosticsInSettings"),
    ).catch(() => {});
  patch({ status, error: "", loading: false });
}
export async function run(label: string, action: () => Promise<unknown>) {
  if (connector.get().busy) return;
  patch({ busy: label });
  try {
    await action();
  } catch (e) {
    notify(e instanceof Error ? e.message : String(e), true);
  } finally {
    patch({ busy: "" });
  }
}
export function startConnector() {
  let stopped = false;
  let stream: EventSource | undefined;
  const read = () => {
    if (!stopped) void refreshConnector().catch(() => {});
  };
  read();
  if (!isDesktop) {
    stream = new EventSource("/api/events");
    stream.addEventListener("snapshot", (event) => {
      try {
        accept(JSON.parse((event as MessageEvent).data));
      } catch {
        read();
      }
    });
    stream.onerror = read;
  }
  const timer = setInterval(() => {
    if (isDesktop || stream?.readyState !== EventSource.OPEN) read();
  }, 2000);
  return () => {
    stopped = true;
    clearInterval(timer);
    stream?.close();
    clearTimeout(feedbackTimer);
  };
}
