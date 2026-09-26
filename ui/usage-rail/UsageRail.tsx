import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import m from "../../shared/usage-panel.json";
import { api } from "../api";
import { t } from "../i18n";
import { isDesktop } from "../platform";
import { themeColor, themeColors } from "../theme";
import { useWindowSize } from "../state/hooks";
import { activeAgents, subscriptions } from "../state/subscriptions";
import { surfaceStatus } from "../subscriptions/presentation";
import type { ProviderSnapshot } from "../subscriptions/types";
import QuotaRing from "../subscriptions/QuotaRing";
import QuotaBubble from "../subscriptions/QuotaBubble";
import {
  berth,
  notchOutline,
  bubbleShape,
  path,
  clamp,
  type Point,
} from "./geometry";
import type { PanelState, PanelGeometry } from "./layout";
import { useSpring } from "./spring";
export default function UsageRail({
  state,
  onGeometry,
  onOpen,
}: {
  state?: PanelState;
  onGeometry?: (value: PanelGeometry) => void;
  onOpen?: (id: string) => void;
}) {
  const { snapshot } = subscriptions.use(),
    activity = activeAgents.use(),
    color = themeColor.use();
  const [nativePointer, setNativePointer] = useState<PanelState>(),
    pointer = state ?? nativePointer,
    l = pointer?.layout,
    prefs = pointer?.preferences;
  const selected = useMemo(
    () =>
      snapshot?.settings.enabled
        ? snapshot.providers.filter((p) => p.eligible && p.selected)
        : [],
    [snapshot],
  );
  const held = useRef<ProviderSnapshot[] | undefined>(undefined);
  useEffect(() => {
    held.current = pointer?.pressed ? [...selected] : undefined;
  }, [pointer?.pressed]);
  const allRows = held.current
    ? held.current.flatMap((p) => {
        const current = selected.find((row) => row.providerId === p.providerId);
        return current ? [current] : [];
      })
    : selected;
  const { width, height } = useWindowSize();
  const [page, setPage] = useState(0);
  const capacity = l?.metrics.visibleCount ?? allRows.length,
    pages = Math.max(1, Math.ceil(allRows.length / Math.max(1, capacity)));
  useEffect(() => setPage((old) => Math.min(old, pages - 1)), [pages]);
  const rows = allRows.slice(page * capacity, (page + 1) * capacity);
  const [retained, setRetained] = useState<string | null>(null),
    active = rows.find((p) => p.providerId === retained),
    [measured, setMeasured] = useState(117),
    content = useRef<HTMLDivElement>(null),
    [popover, setPopover] = useState<Point[]>([]);
  const rail = useSpring(
      [0, 238, 64, 46, 88, 0, 0],
      m.railResponse,
      m.railDamping,
    ),
    axis = useSpring([0], m.railResponse, m.railDamping),
    turnOffset = useSpring([0, 0], m.railResponse, m.railDamping),
    card = useSpring([0, 0, 0, 117, 0], m.cardResponse, m.cardDamping);
  const contentFade = useSpring([1], 0.18, 0.9);
  const previousLayout = useRef(l);
  const scale = l?.metrics.scale || 1,
    openness = clamp(rail.value[0]!, 0, 1),
    reveal = clamp(card.value[0]!, 0, 1),
    cardX = card.value[1]!,
    cardY = card.value[2]!,
    cardHeight = Math.max(80 * scale, card.value[3]!);
  useLayoutEffect(() => {
    if (!l) return;
    const previous = previousLayout.current;
    previousLayout.current = l;
    if (!previous || !pointer?.pressed) {
      axis.jump([l.metrics.horizontal ? 1 : 0]);
      turnOffset.jump([0, 0]);
      return;
    }
    if (
      l.metrics.horizontal !== previous.metrics.horizontal ||
      !!l.notch !== !!previous.notch
    ) {
      turnOffset.jump([
        turnOffset.value[0]! +
          (previous.screenX ?? 0) +
          previous.rail.x +
          previous.rail.width / 2 -
          (l.screenX ?? 0) -
          l.rail.x -
          l.rail.width / 2,
        turnOffset.value[1]! +
          (previous.screenY ?? 0) +
          previous.rail.y +
          previous.rail.height / 2 -
          (l.screenY ?? 0) -
          l.rail.y -
          l.rail.height / 2,
      ]);
      turnOffset.to([0, 0]);
    }
    axis.to([l.metrics.horizontal ? 1 : 0]);
  }, [l, pointer?.pressed]);
  useLayoutEffect(() => {
    if (!l) return;
    const q = l.metrics,
      values = [
        pointer?.expanded ? 1 : 0,
        q.length,
        q.thickness,
        q.padding,
        q.pitch,
        q.round ? 1 : 0,
        pointer?.dock === "floating" ? 1 : 0,
      ];
    if (!pointer?.generation) rail.jump(values);
    else rail.to(values);
  }, [pointer?.expanded, l, pointer?.dock]);
  const status = surfaceStatus(rows, prefs?.warningAt ?? 75),
    alert =
      prefs?.alertColor && status.alert
        ? status.color
        : themeColors[color].dark;
  const tint = useSpring([0, 0, 0], m.railResponse, m.railDamping);
  useEffect(() => {
    const color =
      pointer?.expanded || pointer?.dock === "floating" ? "#000000" : alert;
    tint.to([1, 3, 5].map((i) => parseInt(color.slice(i, i + 2), 16)));
  }, [alert, pointer?.expanded, pointer?.dock]);
  const railFill = `rgb(${tint.value.map((v) => Math.round(clamp(v, 0, 255))).join(",")})`;
  const railPoints: Point[] = l
    ? (() => {
        const [o, length, thickness, , , round, floating] = rail.value as [
          number,
          number,
          number,
          number,
          number,
          number,
          number,
        ];
        const r = l.rail,
          s = scale;
        if (l.notch)
          return notchOutline(
            openness,
            r.width,
            l.notch.width,
            l.notch.height,
            thickness,
            round,
            s,
          ).map(
            ([x, y]) =>
              [
                r.x + x + turnOffset.value[0]!,
                r.y + y + turnOffset.value[1]!,
              ] as Point,
          );
        const h =
            m.collapsedLength * s + (length - m.collapsedLength * s) * openness,
          w = m.sliverWidth * s + (thickness - m.sliverWidth * s) * openness,
          angle = (clamp(axis.value[0]!, 0, 1) * Math.PI) / 2,
          c = Math.cos(angle),
          sn = Math.sin(angle),
          mirror = l.edge === "left" || l.edge === "bottom" ? -1 : 1;
        return berth(
          clamp(o, 0, 1),
          length,
          thickness,
          clamp(round, 0, 1),
          clamp(floating, 0, 1),
          s,
        ).map(([x, y]) => {
          const dx = (x - w / 2 + (thickness - w) / 2) * mirror,
            dy = y - h / 2;
          return [
            r.x + r.width / 2 + c * dx + sn * dy + turnOffset.value[0]!,
            r.y + r.height / 2 - sn * dx + c * dy + turnOffset.value[1]!,
          ] as Point;
        });
      })()
    : [];
  const outline = !railPoints.length
    ? ""
    : l?.notch
      ? `M${railPoints.map((p) => p.join(",")).join("L")}`
      : rail.value[6]! > 0
        ? path(railPoints)
        : `M${[...railPoints.slice(18), ...railPoints.slice(0, 18)].map((p) => p.join(",")).join("L")}`;
  const ringPositions: Point[] = l
    ? rows.map((_, i) => {
        const r = l.rail,
          s = scale,
          v = rail.value,
          progress = clamp(axis.value[0]!, 0, 1),
          angle = (progress * Math.PI) / 2,
          c = Math.cos(angle),
          sn = Math.sin(angle),
          dy =
            v[3]! +
            18 * s +
            i * v[4]! +
            ((l.metrics.item - 36 * s) / 2) * progress -
            v[1]! / 2,
          nh = l.notch?.height || 0,
          dx = l.metrics.percentages ? 11 * s * progress : 0;
        return [
          r.x + r.width / 2 + c * dx + sn * dy + turnOffset.value[0]!,
          r.y +
            nh +
            (r.height - nh) / 2 -
            sn * dx +
            c * dy +
            turnOffset.value[1]!,
        ];
      })
    : [];
  const slot = pointer?.providerId
    ? rows.findIndex((p) => p.providerId === pointer.providerId)
    : pointer?.slot;
  const targetRing = slot != null ? ringPositions[slot] : undefined;
  useLayoutEffect(() => {
    if (!l) return;
    if (pointer?.pressed) {
      card.jump([0, cardX, cardY, cardHeight, card.value[4]!]);
      setRetained(null);
      return;
    }
    if (slot != null && rows[slot] && pointer?.expanded && targetRing) {
      const r = l.rail,
        s = scale,
        w = m.cardWidth * s,
        gap = (m.cardGap + m.pointerWidth) * s,
        pad = m.windowPadding * s,
        available =
          l.edge === "top"
            ? l.height - r.y - r.height - gap - pad
            : l.edge === "bottom"
              ? r.y - gap - pad
              : l.height - 2 * pad,
        h = Math.min(measured * s, Math.max(80 * s, available));
      let x = targetRing[0] - w / 2,
        y = targetRing[1] - h / 2;
      if (l.edge === "left") x = r.x + r.width + gap;
      else if (l.edge === "right") x = r.x - gap - w;
      else if (l.edge === "top") y = r.y + r.height + gap;
      else y = r.y - gap - h;
      x = clamp(x, pad, l.width - w - pad);
      y = clamp(y, pad, l.height - h - pad);
      const anchor = l.metrics.horizontal ? targetRing[0] : targetRing[1];
      if (retained === null) card.jump([0, x, y, h, anchor]);
      if (retained !== rows[slot]!.providerId) {
        contentFade.jump([0.35]);
        contentFade.to([1]);
      }
      setRetained(rows[slot]!.providerId);
      card.to([1, x, y, h, anchor]);
    } else card.to([0, cardX, cardY, cardHeight, card.value[4]!]);
  }, [
    l,
    slot,
    pointer?.expanded,
    pointer?.pressed,
    measured,
    targetRing?.[0],
    targetRing?.[1],
    rows.map((p) => p.providerId).join(","),
  ]);
  useEffect(() => {
    if (reveal === 0 && pointer?.slot == null && !pointer?.providerId)
      setRetained(null);
  }, [reveal, pointer?.slot, pointer?.providerId]);
  useLayoutEffect(() => {
    if (!content.current) return;
    const measure = () => setMeasured(content.current!.offsetHeight),
      observer = new ResizeObserver(measure);
    observer.observe(content.current);
    measure();
    return () => observer.disconnect();
  }, [active?.providerId]);
  useEffect(() => setPopover([]), [active?.providerId]);
  const acceptPopover = useCallback(
    (points: Point[]) =>
      setPopover((old) =>
        JSON.stringify(old) === JSON.stringify(points) ? old : points,
      ),
    [],
  );
  const bubble = l
    ? bubbleShape(
        cardX,
        cardY,
        m.cardWidth * scale,
        cardHeight,
        l.edge,
        card.value[4]!,
        scale,
      )
    : [];
  const pageRect = l
    ? l.metrics.horizontal
      ? {
          x: l.rail.x + l.rail.width - l.metrics.padding - 24 * scale,
          y: l.rail.y + l.rail.height / 2 - 10 * scale,
          width: 24 * scale,
          height: 20 * scale,
        }
      : {
          x: l.rail.x + 8 * scale,
          y: l.rail.y + l.rail.height - l.metrics.padding - 22 * scale,
          width: l.rail.width - 16 * scale,
          height: 20 * scale,
        }
    : { x: 0, y: 0, width: 0, height: 0 };
  const visible = reveal > 0.015 && active;
  let corridor: Point[] = [];
  if (visible && l) {
    const r = l.rail,
      a = card.value[4]!,
      g = m.cardGap * scale,
      s = scale;
    if (l.edge === "right")
      corridor = [
        [r.x - g, a - 12 * s],
        [r.x, a - 20 * s],
        [r.x, a + 20 * s],
        [r.x - g, a + 12 * s],
      ];
    if (l.edge === "left")
      corridor = [
        [r.x + r.width, a - 20 * s],
        [r.x + r.width + g, a - 12 * s],
        [r.x + r.width + g, a + 12 * s],
        [r.x + r.width, a + 20 * s],
      ];
    if (l.edge === "top")
      corridor = [
        [a - 20 * s, r.y + r.height],
        [a + 20 * s, r.y + r.height],
        [a + 12 * s, r.y + r.height + g],
        [a - 12 * s, r.y + r.height + g],
      ];
    if (l.edge === "bottom")
      corridor = [
        [a - 12 * s, r.y - g],
        [a + 12 * s, r.y - g],
        [a + 20 * s, r.y],
        [a - 20 * s, r.y],
      ];
  }
  let detail: Point[] = visible ? bubble : [];
  if (visible && popover.length) {
    const points = [...detail, ...popover],
      xs = points.map((p) => p[0]),
      ys = points.map((p) => p[1]),
      left = Math.min(...xs),
      right = Math.max(...xs),
      top = Math.min(...ys),
      bottom = Math.max(...ys);
    detail = [
      [left, top],
      [right, top],
      [right, bottom],
      [left, bottom],
    ];
  }
  const c = pageRect,
    geometry: PanelGeometry = {
      controls:
        pages > 1 && openness > 0.9
          ? [
              [c.x, c.y],
              [c.x + c.width, c.y],
              [c.x + c.width, c.y + c.height],
              [c.x, c.y + c.height],
            ]
          : [],
      width,
      height,
      generation: pointer?.generation,
      display: pointer?.display,
      dock: pointer?.dock,
      rail: railPoints,
      detail,
      rings:
        openness > 0.9
          ? ringPositions.map(([x, y], i) => ({
              x,
              y,
              radius: ((m.ringDiameter + m.ringStroke) / 2) * scale,
              slot: i,
              providerId: rows[i]!.providerId,
            }))
          : [],
      corridor,
    };
  const latestGeometry = useRef(geometry),
    sending = useRef(false),
    pending = useRef(false),
    alive = useRef(true);
  latestGeometry.current = geometry;
  const signature = JSON.stringify(geometry);
  useEffect(() => {
    onGeometry?.(latestGeometry.current);
    if (!isDesktop || !l) return;
    pending.current = true;
    if (sending.current) return;
    sending.current = true;
    void (async () => {
      try {
        while (pending.current && alive.current) {
          pending.current = false;
          await api("usage-panel/geometry", "POST", latestGeometry.current);
        }
      } catch (e) {
        console.error("Unable to publish usage panel geometry", e);
      } finally {
        sending.current = false;
      }
    })();
  }, [signature]);
  useEffect(() => {
    alive.current = true;
    let off: (() => void) | undefined;
    if (isDesktop)
      void import("@tauri-apps/api/event")
        .then(async ({ listen }) => {
          const dispose = await listen<PanelState>("usage-panel:pointer", (e) =>
            setNativePointer(e.payload),
          );
          if (!alive.current) dispose();
          else {
            off = dispose;
            await api("usage-panel/ready", "POST");
          }
        })
        .catch(console.error);
    return () => {
      alive.current = false;
      off?.();
    };
  }, []);
  async function open(id: string) {
    if (pointer?.pressed) return;
    if (isDesktop) await api("subscriptions/open", "POST", { providerId: id });
    else onOpen?.(id);
  }
  const working = rows.some((p) => activity.includes(p.agentId));
  return (
    <div
      className="usage-surface"
      onContextMenu={(e) => {
        if (isDesktop) e.preventDefault();
      }}
    >
      <svg
        className="surface-shapes"
        viewBox={`0 0 ${width} ${height}`}
        aria-hidden="true"
      >
        <path
          className="rail-hit-target"
          d={path(railPoints)}
          fill={railFill}
        />
        {l?.notch && openness < 0.99 && (
          <rect
            x={l.rail.x + (l.rail.width - l.notch.width) / 2 + 12 * scale}
            y={l.rail.y + l.notch.height}
            width={Math.max(0, l.notch.width - 24 * scale)}
            height={2 * scale}
            fill={alert}
            opacity={1 - openness}
          />
        )}
        <path
          className={`rail-outline ${working && openness < 0.99 ? "rail-activity" : ""}`}
          d={outline}
          opacity={working ? Math.max(0.5, openness) : openness}
        />
        {active && (
          <path className="bubble-shape" d={path(bubble)} opacity={reveal} />
        )}
      </svg>
      <div
        className="ring-layer"
        style={{ clipPath: `path('${path(railPoints)}')` }}
      >
        {rows.map((p, i) => (
          <div
            key={p.providerId}
            className="ring-group"
            style={{
              left: (ringPositions[i]?.[0] ?? 0) - 20 * scale,
              top: (ringPositions[i]?.[1] ?? 0) - 18 * scale,
              transform: `scale(${scale})`,
              opacity: openness,
              visibility: openness < 0.01 ? "hidden" : "visible",
            }}
          >
            <button
              type="button"
              className="ring-button"
              data-provider-id={p.providerId}
              style={{ pointerEvents: openness > 0.9 ? "auto" : "none" }}
              aria-label={`${p.name} · ${t("usageOpenDetails")}`}
              onClick={() => void open(p.providerId)}
            >
              <QuotaRing
                provider={p}
                warningAt={prefs?.warningAt}
                showPercentage={l?.metrics.percentages}
              />
            </button>
          </div>
        ))}
        {pages > 1 && openness > 0.9 && (
          <button
            type="button"
            className="rail-page"
            aria-label={t("usageNextPage")}
            style={{
              left: c.x,
              top: c.y,
              width: c.width,
              height: c.height,
              fontSize: 12 * scale,
            }}
            onClick={() => {
              if (!pointer?.pressed) {
                setPage((page + 1) % pages);
                setRetained(null);
              }
            }}
          >
            {page + 1}/{pages} ›
          </button>
        )}
      </div>
      {active && (
        <aside
          className="rail-bubble"
          style={{
            left: cardX,
            top: cardY,
            height: cardHeight / scale,
            transform: `scale(${scale})`,
            opacity: reveal,
            pointerEvents: reveal > 0.015 ? "auto" : "none",
          }}
        >
          <div ref={content} style={{ opacity: contentFade.value[0] }}>
            <QuotaBubble
              key={active.providerId}
              rail
              provider={active}
              warningAt={prefs?.warningAt}
              detailPlacement={l?.edge === "right" ? "left" : "right"}
              onPopover={acceptPopover}
            />
          </div>
        </aside>
      )}
    </div>
  );
}
