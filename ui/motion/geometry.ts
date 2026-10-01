import { useEffect, useMemo, useSyncExternalStore } from "react";
import { animate, motionValue, frame, cancelFrame } from "motion";
import { createStore } from "../state/store";

// Convert the product's response/damping ratio to Motion's physical spring.
export function springTransition(response: number, damping: number) {
  const frequency = 2 * Math.PI / response;
  return { type: "spring" as const, stiffness: frequency ** 2,
    damping: 2 * damping * frequency, mass: 1, restDelta: 0.001, restSpeed: 0.005 };
}

// SVG contours and native hit regions need numeric snapshots of the same frame.
// Motion owns integration, velocity and interruption; publish once per frame.
export function useGeometrySpring(initial: number[], response: number, damping: number) {
  const spring = useMemo(() => {
    const values = initial.map((value) => motionValue(value));
    const state = createStore([...initial]);
    const reduced = matchMedia("(prefers-reduced-motion: reduce)");
    let goal = [...initial];
    const publish = () => state.set(values.map((value) => value.get()));
    const changed = () => frame.preRender(publish);
    const jump = (next: number[]) => {
      goal = [...next];
      values.forEach((value, i) => value.jump(next[i]!));
      publish();
    };
    const to = (next: number[]) => {
      if (next.every((value, i) => value === goal[i])) return;
      if (reduced.matches) { jump(next); return; }
      goal = [...next];
      values.forEach((value, i) => {
        if (value.get() !== next[i] || value.isAnimating())
          animate(value, next[i]!, springTransition(response, damping));
      });
    };
    const reduce = () => { if (reduced.matches) jump(goal); };
    return { state, to, jump, start: () => {
      const stops = values.map((value) => value.on("change", changed));
      reduced.addEventListener("change", reduce);
      return () => {
        stops.forEach((stop) => stop());
        values.forEach((value) => value.stop());
        cancelFrame(publish);
        reduced.removeEventListener("change", reduce);
      };
    } };
  }, [response, damping]);
  useEffect(spring.start, [spring]);
  return { value: useSyncExternalStore(spring.state.subscribe, spring.state.get), to: spring.to, jump: spring.jump };
}
