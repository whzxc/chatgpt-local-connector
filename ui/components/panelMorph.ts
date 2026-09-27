// A short-lived visual copy keeps trigger content crisp while its surface grows.
// The real trigger and Radix content retain ownership of all interaction.
export type PanelOrigin = {
  trigger: HTMLElement;
  visual: HTMLElement;
  rect: DOMRect;
  surface: Keyframe;
  copy: HTMLElement;
  parent?: HTMLElement;
};

export const morphEasing = "cubic-bezier(.2,.8,.3,1)";
export const reducedMotion = () =>
  matchMedia("(prefers-reduced-motion: reduce)").matches;

export function surfaceOf(node: Element): Keyframe {
  const s = getComputedStyle(node);
  const rect = node.getBoundingClientRect();
  const corners = [s.borderTopLeftRadius, s.borderTopRightRadius, s.borderBottomRightRadius, s.borderBottomLeftRadius]
    .map((corner) => {
      const values = corner.split(" ");
      return [values[0], values[1] ?? values[0]].map((v, i) =>
        v.endsWith("%") ? `${parseFloat(v) * (i ? rect.height : rect.width) / 100}px` : v,
      );
    });
  return {
    backgroundColor: s.backgroundColor,
    borderColor: s.borderColor,
    borderWidth: s.borderWidth,
    borderStyle: s.borderStyle,
    borderRadius: `${corners.map((c) => c[0]).join(" ")} / ${corners.map((c) => c[1]).join(" ")}`,
    boxShadow: s.boxShadow,
  };
}

function visualCopy(node: HTMLElement): HTMLElement {
  const copy = node.cloneNode(true) as HTMLElement;
  const originals = [node, ...node.querySelectorAll("*")];
  const copies = [copy, ...copy.querySelectorAll("*")];
  const ids = new Map<string, string>();
  const prefix = `morph-${crypto.randomUUID()}-`;
  originals.forEach((original, i) => {
    const clone = copies[i] as HTMLElement | SVGElement;
    const computed = getComputedStyle(original);
    for (const key of computed)
      clone.style.setProperty(key, computed.getPropertyValue(key));
    clone.style.animation = "none";
    clone.style.transition = "none";
    clone.style.pointerEvents = "none";
    clone.removeAttribute("autofocus");
    clone.removeAttribute("name");
    clone.removeAttribute("tabindex");
    if (original.id) {
      ids.set(original.id, prefix + original.id);
      clone.id = prefix + original.id;
    }
  });
  for (const clone of copies) {
    for (const attribute of Array.from(clone.attributes)) {
      let value = attribute.value;
      for (const [id, next] of ids) {
        value = value.replaceAll(`url(#${id})`, `url(#${next})`)
          .replaceAll(`url("#${id}")`, `url("#${next}")`);
        if (value === `#${id}`) value = `#${next}`;
      }
      if (value !== attribute.value) clone.setAttribute(attribute.name, value);
    }
  }
  copy.inert = true;
  copy.setAttribute("aria-hidden", "true");
  Object.assign(copy.style, {
    position: "absolute", inset: "0 auto auto 0", margin: "0",
    transform: "none", visibility: "visible", opacity: "1",
    background: "transparent", borderColor: "transparent", boxShadow: "none",
    outline: "none", minWidth: "0", minHeight: "0", maxWidth: "none", maxHeight: "none",
  });
  return copy;
}

export function readOrigin(trigger: HTMLElement, visual = trigger, surface = visual): PanelOrigin | undefined {
  const rect = visual.getBoundingClientRect();
  if (!rect.width || !rect.height || !visual.isConnected) return;
  const copy = visualCopy(visual);
  const width = visual.offsetWidth || rect.width;
  const height = visual.offsetHeight || rect.height;
  copy.style.width = `${width}px`;
  copy.style.height = `${height}px`;
  copy.style.transformOrigin = "0 0";
  copy.style.transform = `scale(${rect.width / width}, ${rect.height / height})`;
  return { trigger, visual, rect, surface: surfaceOf(surface), copy,
    parent: trigger.closest<HTMLElement>(".elastic-panel, .page-sheet") ?? undefined,
  };
}

let pending: { origin: PanelOrigin; at: number } | undefined;
export function capturePanelOrigin(target: EventTarget | null) {
  pending = undefined;
  if (!(target instanceof Element)) return;
  const trigger = target.closest<HTMLElement>("button, [data-panel-anchor]");
  if (!trigger) return;
  const capsule = trigger.closest<HTMLElement>(".navigation-capsule:not(.expanded)");
  const visual = capsule ?? trigger.querySelector<HTMLElement>("[data-panel-visual]") ?? trigger;
  const surface = capsule ? document.querySelector<HTMLElement>(".navigation-surface") ?? visual : visual;
  const origin = readOrigin(trigger, visual, surface);
  if (origin) pending = { origin, at: performance.now() };
}

export function takePanelOrigin() {
  const value = pending;
  pending = undefined;
  return value && performance.now() - value.at < 800 ? value.origin : undefined;
}

export function hideOrigin(origin: PanelOrigin) {
  const before = origin.visual.style.opacity;
  origin.visual.style.opacity = "0";
  return () => { origin.visual.style.opacity = before; };
}

export function animateOriginContent(
  origin: PanelOrigin, target: DOMRect, duration: number, zIndex: number, closing = false,
) {
  const host = document.createElement("div");
  host.className = "panel-morph-content";
  host.setAttribute("aria-hidden", "true");
  host.inert = true;
  Object.assign(host.style, {
    position: "fixed", left: "0", top: "0", pointerEvents: "none",
    width: `${origin.rect.width}px`, height: `${origin.rect.height}px`,
    zIndex: String(zIndex),
  });
  host.append(origin.copy.cloneNode(true));
  document.body.append(host);
  const start = `translate(${origin.rect.x}px, ${origin.rect.y}px)`;
  // Move the old content with the surface, without stretching text or SVGs.
  const end = `translate(${target.x + (target.width - origin.rect.width) / 2}px, ${target.y + Math.min(28, (target.height - origin.rect.height) / 2)}px)`;
  const frames: Keyframe[] = [
    { transform: start, opacity: 1, offset: 0 },
    { opacity: 0, offset: 0.55 },
    { transform: end, opacity: 0, offset: 1 },
  ];
  const animation = host.animate(closing
    ? [...frames].reverse().map((frame) => ({ ...frame, offset: 1 - Number(frame.offset) }))
    : frames, { duration, easing: morphEasing, fill: "both" });
  const cleanup = () => { animation.cancel(); host.remove(); };
  void animation.finished.then(cleanup, cleanup);
  return cleanup;
}
