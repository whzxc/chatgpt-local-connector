import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
  type CSSProperties,
} from "react";
import { Popover } from "radix-ui";
import { useSpring } from "../usage-rail/spring";
import m from "../../shared/usage-panel.json";
import { isDesktop } from "../platform";
export type BubbleControls = {
  register: (key: string, node: HTMLElement | null) => void;
  hover: (key: string) => void;
  focus: (key: string) => void;
  expanded: boolean;
  displayed: string;
};
export default function UsageHistory({
  children,
  detail,
  rail = false,
  side = "left",
  onBounds,
}: {
  children: (controls: BubbleControls) => ReactNode;
  detail: (key: string) => ReactNode;
  rail?: boolean;
  side?: "left" | "right" | "bottom";
  onBounds?: (points: [number, number][]) => void;
}) {
  const [expanded, setExpanded] = useState(false),
    [displayed, setDisplayed] = useState("");
  const triggers = useRef(new Map<string, HTMLElement>()),
    content = useRef<HTMLDivElement>(null),
    card = useRef<HTMLDivElement>(null);
  const [contentNode, setContentNode] = useState<HTMLDivElement | null>(null);
  const attachContent = useCallback((node: HTMLDivElement | null) => {
    content.current = node;
    setContentNode(node);
  }, []);
  const measuredContent = useRef<HTMLDivElement | null>(null);
  const live = useRef({ expanded, displayed });
  live.current = { expanded, displayed };
  const hoverTarget = useRef(""),
    timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const motion = useSpring([0, 0, 80], m.cardResponse, m.cardDamping),
    fade = useSpring([1], 0.18, 0.9);
  const refs = useRef({ motion, fade, onBounds, side });
  refs.current = { motion, fade, onBounds, side };
  const register = (key: string, node: HTMLElement | null) => {
    if (node) triggers.current.set(key, node);
    else triggers.current.delete(key);
  };
  const geometry = (key: string) => {
    const trigger = triggers.current.get(key);
    if (!trigger) return;
    const rect = trigger.getBoundingClientRect();
    const currentSide = refs.current.side;
    return [
      currentSide === "left"
        ? rect.left
        : currentSide === "right"
          ? rect.right
          : rect.left + rect.width / 2,
      rect.top + rect.height / 2,
      Math.min(
        content.current?.scrollHeight || 80,
        Math.min(360, innerHeight * 0.6),
      ),
    ];
  };
  function schedule(key: string) {
    if (hoverTarget.current === key) return;
    hoverTarget.current = key;
    clearTimeout(timer.current);
    timer.current = setTimeout(
      () => {
        if (!key) {
          setExpanded(false);
          return;
        }
        const next = geometry(key);
        if (!next) return;
        if (!live.current.expanded) refs.current.motion.jump(next);
        else refs.current.motion.to(next);
        if (live.current.displayed !== key && live.current.expanded) {
          refs.current.fade.jump([0.35]);
          refs.current.fade.to([1]);
        }
        setDisplayed(key);
        setExpanded(true);
      },
      key ? (live.current.expanded ? 0 : 120) : 180,
    );
  }
  const dismiss = () => {
    clearTimeout(timer.current);
    hoverTarget.current = "";
    setExpanded(false);
  };
  useLayoutEffect(() => {
    if (!expanded || !contentNode) return;
    const retarget = () => {
      const next = geometry(displayed);
      if (next) motion.to(next);
    };
    const initial = geometry(displayed);
    if (initial && measuredContent.current !== contentNode)
      motion.jump(initial);
    else retarget();
    measuredContent.current = contentNode;
    const observer = new ResizeObserver(retarget);
    observer.observe(contentNode);
    window.addEventListener("resize", retarget);
    window.addEventListener("scroll", retarget, true);
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", retarget);
      window.removeEventListener("scroll", retarget, true);
    };
  }, [expanded, displayed, contentNode]);
  useLayoutEffect(() => {
    if (!expanded || !card.current) {
      onBounds?.([]);
      return;
    }
    let frame = 0;
    const report = () => {
      const r = card.current?.getBoundingClientRect();
      if (r) {
        // Reserve the incoming content's height while the spring is growing.
        // Native clipping must not trail the visible bubble's animation.
        const targetHeight = Math.min(content.current?.scrollHeight || 0, Math.min(360, innerHeight * 0.6)) + 36;
        const growth = Math.max(0, targetHeight - r.height);
        onBounds?.([
          [r.left - 24, r.top - 24 - growth],
          [r.right + 24, r.top - 24 - growth],
          [r.right + 24, r.bottom + 24 + growth],
          [r.left - 24, r.bottom + 24 + growth],
        ]);
      }
      frame = requestAnimationFrame(report);
    };
    report();
    return () => cancelAnimationFrame(frame);
  }, [expanded, contentNode, onBounds]);
  useEffect(() => {
    let disposed = false,
      off: (() => void) | undefined;
    if (rail && isDesktop)
      void import("@tauri-apps/api/event").then(async ({ listen }) => {
        const stop = await listen<{ x: number; y: number }>(
          "usage-panel:hover",
          ({ payload: { x, y } }) => {
            const contains = (node: HTMLElement | null) => {
              const r = node?.getBoundingClientRect();
              return (
                r && x >= r.left && x <= r.right && y >= r.top && y <= r.bottom
              );
            };
            const key = [...triggers.current].find(([, node]) =>
              contains(node),
            )?.[0];
            schedule(
              key ??
                (live.current.expanded && contains(card.current)
                  ? live.current.displayed
                  : ""),
            );
          },
        );
        if (disposed) stop();
        else off = stop;
      });
    return () => {
      disposed = true;
      off?.();
      clearTimeout(timer.current);
      refs.current.onBounds?.([]);
    };
  }, [rail]);
  const anchor = useRef({ getBoundingClientRect: () => new DOMRect() });
  anchor.current.getBoundingClientRect = () =>
    new DOMRect(motion.value[0], motion.value[1], 0, 0);
  return (
    <>
      {children({
        register,
        hover: (key) => {
          if (!(rail && isDesktop)) schedule(key);
        },
        focus: schedule,
        expanded,
        displayed,
      })}
      <Popover.Root
        open={expanded}
        onOpenChange={(next) => {
          if (!next) dismiss();
        }}
      >
        <Popover.Anchor virtualRef={anchor} />
        <Popover.Portal>
          <Popover.Content
            ref={card}
            className={`history-popover ${rail ? "rail-popover" : ""}`}
            side={side}
            align="center"
            sideOffset={28}
            collisionPadding={12}
            updatePositionStrategy="always"
            style={
              {
                height: motion.value[2]! + 36,
                "--tail-span": `${Math.max(12, Math.min(80, motion.value[2]! - 4))}px`,
              } as CSSProperties
            }
            onOpenAutoFocus={(e) => e.preventDefault()}
            onCloseAutoFocus={(e) => e.preventDefault()}
            onEscapeKeyDown={(e) => {
              e.preventDefault();
              e.stopPropagation();
              triggers.current.get(displayed)?.focus({ preventScroll: true });
              dismiss();
            }}
            onMouseEnter={() => schedule(displayed)}
            onMouseLeave={() => schedule("")}
          >
            <div className="history-scroll">
              <div
                ref={attachContent}
                className="history-detail"
                style={{ opacity: fade.value[0] }}
              >
                {detail(displayed)}
              </div>
            </div>
            <svg
              className="usage-detail-tail"
              viewBox="0 0 20 80"
              preserveAspectRatio="none"
              aria-hidden="true"
            >
              <path d="M20 0 C20 28.8 8.4 31.2 0 40 C8.4 48.8 20 51.2 20 80 Z" />
              <path className="usage-detail-tail-outline" d="M20 0 C20 28.8 8.4 31.2 0 40 C8.4 48.8 20 51.2 20 80" vectorEffect="non-scaling-stroke" />
            </svg>
          </Popover.Content>
        </Popover.Portal>
      </Popover.Root>
    </>
  );
}
