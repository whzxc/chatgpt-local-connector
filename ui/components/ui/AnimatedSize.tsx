import { forwardRef, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { AnimatePresence, motion, useIsPresent, useReducedMotion, type HTMLMotionProps } from "motion/react";
import { Collapsible } from "radix-ui";
import "./AnimatedSize.css";

// Animate real layout dimensions, so text and timeline nodes are never scaled.
export const AnimatedSize = forwardRef<HTMLDivElement, Omit<HTMLMotionProps<"div">, "children"> & {
  children: ReactNode;
  contentClassName?: string;
  expand?: boolean;
  onSettled?: () => void;
}>(function AnimatedSize({ children, contentClassName, className = "", expand = false, onSettled, ...props }, forwardedRef) {
  const shell = useRef<HTMLDivElement | null>(null);
  const content = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState<{ height: number; nested: boolean }>();
  const present = useIsPresent();
  const reduced = useReducedMotion();
  useLayoutEffect(() => {
    const outer = shell.current, inner = content.current;
    if (!outer || !inner) return;
    const measure = () => {
      const limit = parseFloat(getComputedStyle(outer).maxHeight);
      const height = Math.min(inner.getBoundingClientRect().height, Number.isFinite(limit) ? limit : Infinity);
      // An inner disclosure already animates its own layout. Follow its frames
      // directly instead of starting a second, delayed animation on every frame.
      const nested = !!inner.querySelector('[data-resizing="true"]');
      setSize(previous => previous?.height === height && previous.nested === nested ? previous : { height, nested });
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(inner);
    window.addEventListener("resize", measure);
    return () => { observer.disconnect(); window.removeEventListener("resize", measure); };
  }, []);
  return <motion.div {...props} ref={node => {
    shell.current = node;
    if (typeof forwardedRef === "function") forwardedRef(node);
    else if (forwardedRef) forwardedRef.current = node;
  }} className={`animated-size ${className}`} inert={!present} aria-hidden={!present || undefined}
    initial={expand ? { height: 0 } : false}
    animate={{ height: size?.height ?? "auto" }} exit={{ height: 0 }}
    transition={{ duration: reduced || (present && size?.nested) ? 0 : 0.2, ease: [0.2, 0.8, 0.2, 1] }}
    onAnimationStart={() => { if (shell.current) shell.current.dataset.resizing = "true"; }}
    onAnimationComplete={() => { if (shell.current) delete shell.current.dataset.resizing; onSettled?.(); }}>
    <div ref={content} className={contentClassName}>{children}</div>
  </motion.div>;
});

export function AnimatedCollapse({ open, children, className, onSettled }: {
  open: boolean; children: ReactNode; className?: string; onSettled?: () => void;
}) {
  return <AnimatePresence initial={false}>{open && <Collapsible.Content forceMount asChild>
    <AnimatedSize expand contentClassName={className} onSettled={onSettled}>{children}</AnimatedSize>
  </Collapsible.Content>}</AnimatePresence>;
}
