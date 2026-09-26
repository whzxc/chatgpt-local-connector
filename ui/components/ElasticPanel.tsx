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
const reduced = () => matchMedia("(prefers-reduced-motion: reduce)").matches;
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
  const trigger = useRef<HTMLElement | null>(
    document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null,
  );
  const origin = useRef<DOMRect | undefined>(
    trigger.current?.getBoundingClientRect(),
  );
  const closing = useRef(false),
    motion = useRef<Animation | null>(null),
    backdropPressed = useRef(false);
  const drag = useRef<
    { id: number; x: number; y: number; left: number; top: number } | undefined
  >(undefined);
  const layers = panelLayers.use(),
    recessed = !!node && layers.includes(node) && layers.at(-1) !== node;
  const latest = useRef({ onClose, busy });
  latest.current = { onClose, busy };
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
    if (!node || !open) return;
    const previous = panelLayers.get().at(-1);
    const handoff =
      (panelHandoff.until ?? 0) > performance.now()
        ? panelHandoff.from
        : undefined;
    const source =
      handoff ?? previous?.getBoundingClientRect() ?? origin.current;
    if (handoff && panelHandoff.trigger) trigger.current = panelHandoff.trigger;
    panelHandoff.from = undefined;
    panelHandoff.trigger = undefined;
    panelLayers.set((old) => [...old, node]);
    const focusFrame = requestAnimationFrame(() =>
      node.focus({ preventScroll: true }),
    );
    const rect = node.getBoundingClientRect();
    if (!reduced() && source && rect.width) {
      motion.current = node.animate(
        [
          frameAt(source, previous || handoff ? 1 : 0, previous || handoff ? "28px" : "50%"),
          frameAt(rect, 1, "28px"),
        ],
        { duration: previous || handoff ? 320 : duration, easing },
      );
      Array.from(node.children).forEach((child) =>
        child.animate([{ opacity: 0 }, { opacity: 1 }], {
          duration: 280,
          delay: 80,
          fill: "backwards",
        }),
      );
    }
    // Keep the natural target height, not the opening animation's trigger height.
    // Otherwise ResizeObserver replays the expansion as soon as opening finishes.
    let height = rect.height,
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
      effect.setKeyframes(frames);
      height = target.height;
    });
    openingContent.observe(node, { childList: true, subtree: true, characterData: true });
    const resize = () => {
      if (
        closing.current ||
        motion.current?.playState === "running" ||
        resizing?.playState === "running"
      )
        return;
      const next = node.offsetHeight;
      if (Math.abs(next - height) > 1 && !reduced()) {
        resizing = node.animate(
          [{ height: `${height}px` }, { height: `${next}px` }],
          { duration: 320, easing },
        );
      }
      height = next;
    };
    const observer = new ResizeObserver(() => {
      cancelAnimationFrame(resizeFrame);
      resizeFrame = requestAnimationFrame(resize);
    });
    observer.observe(node);
    return () => {
      cancelAnimationFrame(focusFrame);
      cancelAnimationFrame(resizeFrame);
      rememberPanel(node, trigger.current ?? undefined);
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
  async function close(replacement?: HTMLElement) {
    if (latest.current.busy || closing.current || !node) return;
    closing.current = true;
    const previous = panelLayers.get().at(-2),
      from = node.getBoundingClientRect(),
      to =
        previous?.getBoundingClientRect() ??
        (trigger.current?.isConnected
          ? trigger.current.getBoundingClientRect()
          : origin.current);
    if (replacement) {
      rememberPanel(node, replacement);
    } else if (to && !reduced()) {
      const style = getComputedStyle(node);
      const current = frameAt(from, Number(style.opacity), style.borderRadius);
      const closeDuration = previous ? 320 : duration;
      motion.current?.cancel();
      motion.current = node.animate(
        [
          current,
          frameAt(to, previous ? 1 : 0, previous ? "28px" : "50%"),
        ],
        { duration: closeDuration, easing, fill: "forwards" },
      );
      Array.from(node.children).forEach((child) => {
        const opacity = getComputedStyle(child).opacity;
        child.getAnimations().forEach((animation) => animation.cancel());
        child.animate([{ opacity: 0 }, { opacity }], {
          duration: 280,
          delay: Math.max(0, closeDuration - 360),
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
        </D.Content>
      </D.Portal>
    </D.Root>
  );
}
