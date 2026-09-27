import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
  type CSSProperties,
} from "react";
import { Dialog as D } from "radix-ui";
import { X } from "lucide-react";
import { t } from "../i18n";
import {
  panelLayers,
  panelHandoff,
  panelAnchorAt,
  rememberPanel,
} from "../state/panels";
import {
  takePanelOrigin, readOrigin, hideOrigin, animateOriginContent, surfaceOf,
  reducedMotion as reduced, type PanelOrigin,
} from "./panelMorph";
const easing = "cubic-bezier(.2,.8,.3,1)";
const duration = 480;
const frameAt = (rect: DOMRect, opacity: number, borderRadius: string): Keyframe => ({
  transform: `translate(calc(-50% + ${rect.x + rect.width / 2 - innerWidth / 2}px),calc(-50% + ${rect.y + rect.height / 2 - innerHeight / 2}px))`,
  width: `${rect.width}px`,
  height: `${rect.height}px`,
  opacity,
  borderRadius,
});
export default function ElasticPanel({
  open = true,
  onClose,
  title,
  children,
  footer,
  actions,
  wide = false,
  busy = false,
  unframed = false,
  headerless = false,
  width,
  depthOffset = 0,
}: {
  open?: boolean;
  onClose: () => void;
  title: ReactNode;
  children: ReactNode;
  footer?: ReactNode;
  actions?: ReactNode;
  wide?: boolean;
  busy?: boolean;
  unframed?: boolean;
  headerless?: boolean;
  width?: number;
  depthOffset?: number;
}) {
  const [node, setNode] = useState<HTMLDivElement | null>(null),
    [position, setPosition] = useState({ x: 0, y: 0 });
  const panelRef = useRef<HTMLDivElement | null>(null);
  const attach = useCallback((el: HTMLDivElement | null) => {
    panelRef.current = el;
    setNode(el);
  }, []);
  const positionRef = useRef(position);
  positionRef.current = position;
  const visualOrigin = useRef<PanelOrigin | undefined>(undefined);
  const restoreOrigin = useRef<(() => void) | undefined>(undefined);
  const clearGhost = useRef<(() => void) | undefined>(undefined);
  const trigger = useRef<HTMLElement | null>(null);
  const origin = useRef<DOMRect | undefined>(undefined);
  const parentPanel = useRef<HTMLElement | undefined>(undefined);
  const [baseDepth, setBaseDepth] = useState(1);
  const closing = useRef(false),
    motion = useRef<Animation | null>(null),
    backdropPressed = useRef(false);
  const drag = useRef<
    { id: number; x: number; y: number; left: number; top: number } | undefined
  >(undefined);
  const layers = panelLayers.use(),
    recessed = !!node && layers.includes(node) && layers.at(-1) !== node;
  const previousView = useRef({ recessed, depthOffset });
  const latest = useRef({ onClose, busy, depthOffset });
  latest.current = { onClose, busy, depthOffset };
  const moveTo = (x: number, y: number) => {
    if (!node) return;
    const maxX = Math.max(0, (innerWidth - node.offsetWidth) / 2 - 20),
      maxY = Math.max(0, (innerHeight - node.offsetHeight) / 2 - 20);
    setPosition({
      x: Math.max(-maxX, Math.min(maxX, x)),
      y: Math.max(-maxY, Math.min(maxY, y)),
    });
  };
  useLayoutEffect(() => {
    if (!open) return;
    const active = document.activeElement;
    const visual = takePanelOrigin() ?? (
      active instanceof HTMLElement && active.matches("button, [data-panel-anchor]")
        ? readOrigin(active) : undefined
    );
    visualOrigin.current = visual;
    trigger.current = visual?.trigger ?? null;
    origin.current = visual?.rect;
    parentPanel.current = visual?.parent;
    closing.current = false;
    setPosition({ x: 0, y: 0 });
  }, [open]);
  useLayoutEffect(() => {
    if (!node || !open) return;
    const previous = panelLayers.get().at(-1);
    const parent = parentPanel.current ?? previous;
    parentPanel.current = parent;
    setBaseDepth(parent ? Number(parent.dataset.panelDepth || 1) + 1 : 1);
    const handoff =
      (panelHandoff.until ?? 0) > performance.now()
        ? panelHandoff.from
        : undefined;
    const source =
      parent?.getBoundingClientRect() ?? handoff ?? visualOrigin.current?.rect ?? origin.current;
    if (handoff && panelHandoff.trigger) trigger.current = panelHandoff.trigger;
    panelHandoff.from = undefined;
    panelHandoff.trigger = undefined;
    panelLayers.set((old) => [...old, node]);
    const focusFrame = requestAnimationFrame(() =>
      node.focus({ preventScroll: true }),
    );
    let rect = node.getBoundingClientRect();
    if (parent && source) {
      const x = Math.max(-(innerWidth - rect.width) / 2 + 20, Math.min((innerWidth - rect.width) / 2 - 20, source.x + source.width / 2 - innerWidth / 2));
      const y = Math.max(-(innerHeight - rect.height) / 2 + 20, Math.min((innerHeight - rect.height) / 2 - 20, source.y + source.height / 2 - innerHeight / 2));
      setPosition({ x, y });
      rect = new DOMRect((innerWidth - rect.width) / 2 + x, (innerHeight - rect.height) / 2 + y, rect.width, rect.height);
    }
    if (!reduced() && source && rect.width) {
      const visual = !parent && !handoff ? visualOrigin.current : undefined;
      const timing = parent || handoff ? 320 : duration;
      motion.current = node.animate(
        [
          { ...frameAt(source, visual || parent || handoff ? 1 : 0, "28px"), ...(parent ? surfaceOf(parent) : visual?.surface ?? {}) },
          { ...frameAt(rect, 1, "28px"), ...surfaceOf(node) },
        ],
        { duration: timing, easing },
      );
      if (visual) {
        restoreOrigin.current = hideOrigin(visual);
        clearGhost.current = animateOriginContent(visual, rect, timing, Number(node.style.zIndex) + 1);
      }
      Array.from(node.children).forEach((child) =>
        child.animate([{ opacity: 0 }, { opacity: 1 }], {
          duration: 280,
          delay: parent ? 40 : 130,
          fill: "backwards",
        }),
      );
    }
    // Keep the natural target height, not the opening animation's trigger height.
    // Otherwise ResizeObserver replays the expansion as soon as opening finishes.
    let height = rect.height,
      layoutWidth = rect.width,
      resizing: Animation | undefined,
      resizeFrame = 0;
    // Async content can change the natural target while the opening animation
    // still fixes the panel's height. Measure without its effect in the same
    // frame, then retarget it before the browser paints.
    const openingContent = new MutationObserver(() => {
      const animation = motion.current;
      if (closing.current || animation?.playState !== "running") return;
      const effect = animation.effect as KeyframeEffect | null;
      if (!effect) return;
      animation.effect = null;
      const target = node.getBoundingClientRect();
      animation.effect = effect;
      const frames = effect.getKeyframes();
      frames[frames.length - 1].height = `${target.height}px`;
      frames[frames.length - 1].width = `${target.width}px`;
      effect.setKeyframes(frames);
      height = target.height;
      layoutWidth = target.width;
    });
    openingContent.observe(node, { childList: true, subtree: true, characterData: true });
    const resize = () => {
      if (
        closing.current ||
        motion.current?.playState === "running" ||
        motion.current?.playState === "paused" ||
        resizing?.playState === "running"
      )
        return;
      const next = node.offsetHeight;
      const nextWidth = node.offsetWidth;
      if ((Math.abs(next - height) > 1 || Math.abs(nextWidth - layoutWidth) > 1) && !reduced()) {
        resizing = node.animate(
          [{ width: `${layoutWidth}px`, height: `${height}px` }, { width: `${nextWidth}px`, height: `${next}px` }],
          { duration: 320, easing },
        );
      }
      height = next;
      layoutWidth = nextWidth;
    };
    const observer = new ResizeObserver(() => {
      cancelAnimationFrame(resizeFrame);
      resizeFrame = requestAnimationFrame(resize);
    });
    observer.observe(node);
    return () => {
      cancelAnimationFrame(focusFrame);
      cancelAnimationFrame(resizeFrame);
      restoreOrigin.current?.();
      clearGhost.current?.();
      if (!closing.current) rememberPanel(node, trigger.current ?? undefined);
      panelLayers.set((old) => old.filter((el) => el !== node));
      observer.disconnect();
      openingContent.disconnect();
      resizing?.cancel();
      motion.current?.cancel();
    };
  }, [node, open]);
  useEffect(() => {
    const resize = () => moveTo(positionRef.current.x, positionRef.current.y);
    window.addEventListener("resize", resize);
    return () => window.removeEventListener("resize", resize);
  }, [node]);
  useLayoutEffect(() => {
    const returned = previousView.current.recessed && !recessed;
    const stepped = previousView.current.depthOffset !== depthOffset;
    previousView.current = { recessed, depthOffset };
    if (node && !recessed && (returned || stepped) && !reduced()) {
      // A save can remove the child directly. Continue its surface transition
      // when returning, just as an animated Close does before removing it.
      const from = returned && (panelHandoff.until ?? 0) > performance.now()
        ? panelHandoff.from : undefined;
      const surface = from ? node.animate(
        [frameAt(from, 1, "28px"), frameAt(node.getBoundingClientRect(), 1, "28px")],
        { duration: 320, easing },
      ) : undefined;
      if (from) panelHandoff.from = undefined;
      const animation = node.firstElementChild?.animate(
        [{ opacity: 0 }, { opacity: 1 }], { duration: 180, easing },
      );
      return () => { animation?.cancel(); surface?.cancel(); };
    }
  }, [node, recessed, depthOffset]);
  async function close(replacement?: HTMLElement) {
    if (latest.current.busy || closing.current || !node) return;
    if (latest.current.depthOffset > 0) {
      latest.current.onClose();
      return;
    }
    closing.current = true;
    restoreOrigin.current?.();
    const parent = parentPanel.current?.isConnected ? parentPanel.current : undefined;
    if (parent) replacement = undefined;
    const saved = visualOrigin.current;
    const visual = !parent && saved?.visual.isConnected
      ? readOrigin(saved.trigger, saved.visual)
      : undefined;
    clearGhost.current?.();
    const from = node.getBoundingClientRect(),
      to = parent?.getBoundingClientRect() ?? visual?.rect ?? origin.current;
    if (replacement) {
      rememberPanel(node, replacement);
    } else if (to && !reduced()) {
      const style = getComputedStyle(node);
      const current = { ...frameAt(from, Number(style.opacity), style.borderRadius), ...surfaceOf(node) };
      const closeDuration = parent ? 320 : duration;
      motion.current?.cancel();
      motion.current = node.animate(
        [
          current,
          { ...frameAt(to, visual || parent ? 1 : 0, parent ? "28px" : "50%"), ...(parent ? surfaceOf(parent) : visual?.surface ?? {}) },
        ],
        { duration: closeDuration, easing, fill: "forwards" },
      );
      if (visual) {
        restoreOrigin.current = hideOrigin(visual);
        clearGhost.current = animateOriginContent(visual, from, closeDuration, Number(node.style.zIndex) + 1, true);
      }
      Array.from(node.children).forEach((child) => {
        const opacity = getComputedStyle(child).opacity;
        child.getAnimations().forEach((animation) => animation.cancel());
        child.animate([{ opacity: 0 }, { opacity }], {
          duration: 150,
          delay: 0,
          direction: "reverse",
          fill: "both",
        });
      });
      await motion.current.finished.catch(() => {});
    }
    latest.current.onClose();
    if (replacement)
      requestAnimationFrame(() => {
        panelHandoff.trigger = replacement;
        panelHandoff.from = from;
        panelHandoff.until = performance.now() + 100;
        replacement.focus({ preventScroll: true });
        replacement.click();
      });
  }
  return (
    <D.Root
      open={open}
      onOpenChange={(next) => {
        if (!next) void close();
      }}
    >
      <D.Portal>
        <D.Overlay
          className="elastic-backdrop"
          style={{ zIndex: 100 + Math.max(0, layers.indexOf(node!)) * 2 }}
          onPointerDown={(e) => {
            backdropPressed.current =
              e.button === 0 && e.target === e.currentTarget;
          }}
          onPointerCancel={() => (backdropPressed.current = false)}
          onClick={(e) => {
            if (backdropPressed.current && e.target === e.currentTarget) {
              backdropPressed.current = false;
              void close(
                panelAnchorAt(
                  e.clientX,
                  e.clientY,
                  trigger.current ?? undefined,
                ),
              );
            }
          }}
        />
        <D.Content
          ref={attach}
          className={`dialog elastic-panel ${wide ? "dialog-wide" : ""} ${unframed ? "unframed" : ""} ${headerless ? "headerless" : ""}`}
          data-recessed={recessed || undefined}
          data-panel-depth={baseDepth + depthOffset}
          aria-describedby={undefined}
          style={
            {
              "--preferred-width": `${width ?? (wide ? 880 : 800)}px`,
              transform: `translate(calc(-50% + ${position.x}px),calc(-50% + ${position.y}px))`,
              zIndex: 101 + Math.max(0, layers.indexOf(node!)) * 2,
            } as CSSProperties
          }
          onOpenAutoFocus={(e) => {
            e.preventDefault();
            panelRef.current?.focus({ preventScroll: true });
          }}
          onCloseAutoFocus={(e) => {
            e.preventDefault();
            if (trigger.current?.isConnected)
              trigger.current.focus({ preventScroll: true });
          }}
          onEscapeKeyDown={(e) => {
            e.preventDefault();
            void close();
          }}
          onPointerDownOutside={(e) => e.preventDefault()}
        >
          <div className="elastic-panel-content">
            {unframed || headerless ? (
              <D.Title className="sr-only">{title}</D.Title>
            ) : (
              <header
                className="dialog-header"
                onPointerDown={(e) => {
                  if (
                    e.button !== 0 ||
                    !e.isPrimary ||
                    (e.target as Element).closest("button") ||
                    motion.current?.playState === "running"
                  )
                    return;
                  drag.current = {
                    id: e.pointerId,
                    x: e.clientX,
                    y: e.clientY,
                    left: position.x,
                    top: position.y,
                  };
                  e.currentTarget.setPointerCapture(e.pointerId);
                  e.preventDefault();
                }}
                onPointerMove={(e) => {
                  const d = drag.current;
                  if (d && d.id === e.pointerId)
                    moveTo(d.left + e.clientX - d.x, d.top + e.clientY - d.y);
                }}
                onPointerUp={(e) => {
                  drag.current = undefined;
                  if (e.currentTarget.hasPointerCapture(e.pointerId))
                    e.currentTarget.releasePointerCapture(e.pointerId);
                }}
                onPointerCancel={() => (drag.current = undefined)}
              >
                <D.Title>{title}</D.Title>
                <div className="actions">
                  {actions}
                  <button
                    type="button"
                    className="button ghost icon-button"
                    aria-label={t("close")}
                    disabled={busy}
                    onClick={() => void close()}
                  >
                    <X size={20} strokeWidth={1.75} />
                  </button>
                </div>
              </header>
            )}
            <div className="dialog-body">{children}</div>
            {footer && <footer className="dialog-footer">{footer}</footer>}
          </div>
        </D.Content>
      </D.Portal>
    </D.Root>
  );
}
