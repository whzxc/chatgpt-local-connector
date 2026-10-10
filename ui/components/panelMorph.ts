import { animate } from "motion";
import { surfaceDuration, type Surface } from "../motion/surface";
// A short-lived visual copy keeps trigger content crisp while its surface grows.
// The real trigger and Radix content retain ownership of all interaction.
export type PanelOrigin = {
  trigger: HTMLElement;
  visual: HTMLElement;
  rect: DOMRect;
  surface: Surface;
  copy: HTMLElement;
  parent?: HTMLElement;
};


export function surfaceOf(node: Element): Surface {
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
  // The capsule owns its descendant selectors, so its clone can reuse them.
  // Only inherited values need freezing; copying every computed declaration
  // on every icon delays the very first opening frame.
  const capsule = node.classList.contains("navigation-capsule");
  if (capsule) {
    const computed = getComputedStyle(node);
    copy.style.font = computed.font;
    copy.style.color = computed.color;
    for (const key of computed)
      if (key.startsWith("--")) copy.style.setProperty(key, computed.getPropertyValue(key));
  }
  const ids = new Map<string, string>();
  const prefix = `morph-${crypto.randomUUID()}-`;
  originals.forEach((original, i) => {
    const clone = copies[i] as HTMLElement | SVGElement;
    if (!capsule) {
      const computed = getComputedStyle(original);
      // Freeze the visual in one style write rather than reparsing hundreds of
      // declarations individually on the click path.
      clone.style.cssText = Array.from(computed, (key) =>
        `${key}:${computed.getPropertyValue(key)};`,
      ).join("");
    }
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
  origin: PanelOrigin, zIndex: number, closing = false,
) {
  const host = document.createElement("div");
  host.className = "panel-morph-content";
  host.setAttribute("aria-hidden", "true");
  host.inert = true;
  Object.assign(host.style, {
    position: "fixed", left: `${origin.rect.x}px`, top: `${origin.rect.y}px`, pointerEvents: "none",
    width: `${origin.rect.width}px`, height: `${origin.rect.height}px`,
    zIndex: String(zIndex),
  });
  host.append(origin.copy);
  document.body.append(host);
  // The anchor's content stays at its own screen position throughout the morph.
  const animation = animate(host, {
    opacity: closing ? [0, 0, 1] : [1, 0, 0],
  }, { duration: surfaceDuration, ease: [0.2, 0.8, 0.3, 1],
    opacity: { times: closing ? [0, 0.45, 1] : [0, 0.55, 1] } });
  const cleanup = () => { animation.stop(); host.remove(); };
  // Keep the dismissal copy until its owner restores the real anchor.
  if (!closing) void animation.then(cleanup);
  return cleanup;
}
