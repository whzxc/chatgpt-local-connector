export const isDesktop = "__TAURI_INTERNALS__" in window;
export const isDevelopment = import.meta.env.DEV;
export async function desktopRequest<T>(
  path: string,
  method: string,
  body?: unknown,
): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>("service_request", {
    route: path,
    method,
    body: body ?? {},
  });
}
export async function openUrl(url: string) {
  if (!isDesktop) {
    window.open(url, "_blank", "noopener,noreferrer");
    return;
  }
  const { openUrl } = await import("@tauri-apps/plugin-opener");
  await openUrl(url);
}
export async function notifyNative(title: string, body: string) {
  if (!isDesktop || localStorage.getItem("notifications") === "off") return;
  const { isPermissionGranted, requestPermission, sendNotification } =
    await import("@tauri-apps/plugin-notification");
  if (
    (await isPermissionGranted()) ||
    (await requestPermission()) === "granted"
  )
    sendNotification({ title, body });
}
