import { useEffect, useLayoutEffect, useRef, useState } from "react";
export function useInterval(
  callback: () => void,
  delay: number | null,
  immediate = false,
) {
  const latest = useRef(callback);
  useLayoutEffect(() => {
    latest.current = callback;
  });
  useEffect(() => {
    if (delay === null) return;
    if (immediate) latest.current();
    const timer = setInterval(() => latest.current(), delay);
    return () => clearInterval(timer);
  }, [delay, immediate]);
}
export function useNow() {
  const [now, setNow] = useState(Date.now);
  useInterval(() => setNow(Date.now()), 60000);
  return now;
}
export function useMedia(query: string) {
  const [matches, setMatches] = useState(() => matchMedia(query).matches);
  useEffect(() => {
    const media = matchMedia(query);
    const update = () => setMatches(media.matches);
    update();
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  }, [query]);
  return matches;
}
export function useWindowSize() {
  const [size, setSize] = useState({ width: innerWidth, height: innerHeight });
  useEffect(() => {
    const update = () => setSize({ width: innerWidth, height: innerHeight });
    window.addEventListener("resize", update);
    return () => window.removeEventListener("resize", update);
  }, []);
  return size;
}
