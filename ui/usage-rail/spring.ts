import { useEffect, useMemo, useSyncExternalStore } from "react";
import { createStore } from "../state/store";
// The same damped motion drives both the native rail and its browser adapter.
export function useSpring(
  initial: number[],
  response: number,
  damping: number,
) {
  const spring = useMemo(() => {
    const state = createStore([...initial]);
    let goal = [...initial],
      velocity = initial.map(() => 0),
      frame = 0,
      last = 0;
    const reduced = matchMedia("(prefers-reduced-motion: reduce)");
    const equal = (a: number[], b: number[]) =>
      a.length === b.length && a.every((v, i) => v === b[i]);
    function step(now: number) {
      const dt = Math.min((now - last) / 1000, 0.032);
      last = now;
      const w = (2 * Math.PI) / response,
        wd = w * Math.sqrt(1 - damping * damping),
        e = Math.exp(-damping * w * dt);
      let settled = true;
      const next = state.get().map((x, i) => {
        const target = goal[i]!,
          v = velocity[i]!,
          y = x - target,
          b = (v + damping * w * y) / wd,
          c = Math.cos(wd * dt),
          s = Math.sin(wd * dt);
        const distance = e * (y * c + b * s),
          speed = e * (-damping * w * (y * c + b * s) + wd * (-y * s + b * c));
        velocity[i] = speed;
        if (Math.abs(distance) > 0.001 || Math.abs(speed) > 0.005)
          settled = false;
        return target + distance;
      });
      if (settled) {
        state.set([...goal]);
        velocity.fill(0);
        frame = 0;
      } else {
        state.set(next);
        frame = requestAnimationFrame(step);
      }
    }
    function jump(next: number[]) {
      cancelAnimationFrame(frame);
      frame = 0;
      goal = [...next];
      velocity = next.map(() => 0);
      if (!equal(state.get(), next)) state.set([...next]);
    }
    function to(next: number[]) {
      if (equal(goal, next)) return;
      goal = [...next];
      if (reduced.matches) {
        jump(next);
        return;
      }
      if (!frame) {
        last = performance.now();
        frame = requestAnimationFrame(step);
      }
    }
    const reduce = () => {
      if (reduced.matches) jump(goal);
    };
    return {
      state,
      to,
      jump,
      start: () => {
        reduced.addEventListener("change", reduce);
        return () => {
          cancelAnimationFrame(frame);
          frame = 0;
          reduced.removeEventListener("change", reduce);
        };
      },
    };
  }, []);
  useEffect(spring.start, [spring]);
  return {
    value: useSyncExternalStore(spring.state.subscribe, spring.state.get),
    to: spring.to,
    jump: spring.jump,
  };
}
