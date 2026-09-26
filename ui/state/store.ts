import { useSyncExternalStore } from "react";

// Small immutable stores share native snapshots and preferences across React roots.
export function createStore<T>(initial: T) {
  let value = initial;
  const listeners = new Set<() => void>();
  const get = () => value;
  const subscribe = (listener: () => void) => {
    listeners.add(listener);
    return () => {
      listeners.delete(listener);
    };
  };
  const set = (next: T | ((previous: T) => T)) => {
    const updated =
      typeof next === "function" ? (next as (previous: T) => T)(value) : next;
    if (Object.is(value, updated)) return;
    value = updated;
    listeners.forEach((listener) => listener());
  };
  return {
    get,
    set,
    subscribe,
    use: () => useSyncExternalStore(subscribe, get),
  };
}
export function preference<T>(
  key: string,
  fallback: T,
  parse: (raw: string) => T = JSON.parse,
  serialize: (value: T) => string = JSON.stringify,
) {
  let initial = fallback;
  try {
    const raw = localStorage.getItem(key);
    if (raw !== null) initial = parse(raw);
  } catch {
    /* Use the default if storage is unavailable. */
  }
  const store = createStore(initial);
  store.subscribe(() => {
    try {
      localStorage.setItem(key, serialize(store.get()));
    } catch {
      /* The session remains usable without storage. */
    }
  });
  window.addEventListener("storage", (event) => {
    if (event.key === key) {
      try {
        store.set(event.newValue === null ? fallback : parse(event.newValue));
      } catch {
        store.set(fallback);
      }
    }
  });
  return store;
}
