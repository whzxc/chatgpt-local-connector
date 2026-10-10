import { animate, mix, type AnimationPlaybackControlsWithThen } from "motion";

export const surfaceDuration = 0.2;

export type Surface = Record<string, string | number>;
export const reducedMotion = () => matchMedia("(prefers-reduced-motion: reduce)").matches;

// Morph the shell's real dimensions; its independently laid-out content is clipped,
// never scaled. A shared monotonic curve keeps every edge between its endpoints.
export function surfaceMotion(node: HTMLElement, naturalStyle: () => Surface = () => ({})) {
  const base: Record<string, string> = {};
  const content = node.firstElementChild as HTMLElement | null;
  const contentTranslate = content?.style.translate ?? "";
  let controls: AnimationPlaybackControlsWithThen | undefined;
  let startFrame: number | undefined;
  let generation = 0;
  let goal: Surface = {};
  let rendered: Surface = {};
  let active = false;
  const reduced = matchMedia("(prefers-reduced-motion: reduce)");
  const save = (surface: Surface) => {
    for (const key of Object.keys(surface)) {
      if (!(key in base)) base[key] = (node.style as unknown as Record<string, string>)[key] ?? "";
    }
  };
  const restore = () => {
    Object.assign(node.style, base, naturalStyle());
    if (content) content.style.translate = contentTranslate;
  };
  const settle = () => {
    if (reduced.matches) {
      cancelAnimationFrame(startFrame ?? 0);
      startFrame = undefined;
      controls?.complete();
    }
  };
  reduced.addEventListener("change", settle);
  return {
    get running() { return active; },
    to(target: Surface, { from, retain = false }: {
      from?: Surface; retain?: boolean;
    } = {}) {
      const id = ++generation;
      save(target);
      goal = { ...goal, ...target };
      cancelAnimationFrame(startFrame ?? 0);
      controls?.stop();
      // Opening content belongs at its destination; dismissal keeps it where
      // it is. Only the shell moves, revealing/clipping this stationary layer.
      let contentRect = content?.getBoundingClientRect();
      if (content && !retain) {
        const shellStyle = node.style.cssText;
        const previousTranslate = content.style.translate;
        Object.assign(node.style, goal);
        content.style.translate = contentTranslate;
        contentRect = content.getBoundingClientRect();
        node.style.cssText = shellStyle;
        content.style.translate = previousTranslate;
      }
      const pinContent = () => {
        if (!content || !contentRect) return;
        content.style.translate = contentTranslate;
        const rect = content.getBoundingClientRect();
        content.style.translate = `${contentRect.x - rect.x}px ${contentRect.y - rect.y}px`;
      };
      if (from) {
        save(from);
        rendered = { ...rendered, ...from };
        Object.assign(node.style, from);
      }
      pinContent();
      const computed = getComputedStyle(node);
      const start = Object.fromEntries(Object.keys(goal).map((key) => [
        key, rendered[key] ?? computed[key as keyof CSSStyleDeclaration],
      ])) as Surface;
      const interpolate = mix(start, goal);
      active = true;
      // One progress value writes the whole surface atomically. Mixing DOM
      // transforms (WAAPI) with layout sizes (JS) can put edges a frame apart.
      controls = animate(0, 1, {
        type: "tween",
        autoplay: reduced.matches,
        duration: reduced.matches ? 0 : surfaceDuration,
        ease: [0.2, 0.8, 0.3, 1],
        onUpdate: (progress) => {
          rendered = { ...interpolate(progress) };
          Object.assign(node.style, rendered);
          pinContent();
        },
      });
      if (!reduced.matches) {
        // Keep the origin intact through preparation. Start the clock at the
        // next paint, after React layout and the visual copy are ready.
        const animation = controls;
        startFrame = requestAnimationFrame(() => {
          startFrame = undefined;
          animation.play();
        });
      }
      void controls.then(() => {
        if (id !== generation) return;
        active = false;
        if (!retain) restore();
      });
      return controls;
    },
    dispose() {
      ++generation;
      active = false;
      cancelAnimationFrame(startFrame ?? 0);
      controls?.stop();
      restore();
      reduced.removeEventListener("change", settle);
    },
  };
}
