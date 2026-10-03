import { useEffect, useState, useMemo } from "react";
import { formatSpeed } from "../lib/format";

interface Props {
  downloadSpeed: number;
  uploadSpeed: number;
}

interface SpeedPoint {
  dl: number;
  ul: number;
}

const HISTORY_LENGTH = 30; // 30 seconds rolling window

export default function BandwidthGraph({ downloadSpeed, uploadSpeed }: Props) {
  const [history, setHistory] = useState<SpeedPoint[]>(() =>
    Array.from({ length: HISTORY_LENGTH }, () => ({ dl: 0, ul: 0 })),
  );
  const [hoverIndex, setHoverIndex] = useState<number | null>(null);

  useEffect(() => {
    setHistory((prev) => {
      const next = [...prev.slice(1), { dl: downloadSpeed, ul: uploadSpeed }];
      return next;
    });
  }, [downloadSpeed, uploadSpeed]);

  const maxSpeed = useMemo(() => {
    const peak = history.reduce(
      (m, p) => Math.max(m, p.dl, p.ul),
      1024 * 100, // min scale: 100 KB/s
    );
    return peak;
  }, [history]);

  const width = 176;
  const height = 46;
  const paddingBottom = 4;
  const paddingTop = 4;
  const graphHeight = height - paddingTop - paddingBottom;

  // Build SVG path data for download
  const points = useMemo(() => {
    return history.map((pt, i) => {
      const x = (i / (HISTORY_LENGTH - 1)) * width;
      const y =
        paddingTop +
        graphHeight -
        (Math.min(pt.dl, maxSpeed) / maxSpeed) * graphHeight;
      return { x, y, pt };
    });
  }, [history, maxSpeed, graphHeight, paddingTop, width]);

  const linePath = useMemo(() => {
    if (points.length === 0) return "";
    return points.reduce((acc, p, i) => {
      if (i === 0) return `M ${p.x.toFixed(1)} ${p.y.toFixed(1)}`;
      return `${acc} L ${p.x.toFixed(1)} ${p.y.toFixed(1)}`;
    }, "");
  }, [points]);

  const areaPath = useMemo(() => {
    if (points.length === 0) return "";
    const first = points[0];
    const last = points[points.length - 1];
    const bottom = height - paddingBottom;
    return `${linePath} L ${last.x.toFixed(1)} ${bottom} L ${first.x.toFixed(1)} ${bottom} Z`;
  }, [linePath, points, height, paddingBottom]);

  const currentPoint = hoverIndex !== null ? history[hoverIndex] : null;

  return (
    <div
      className="bandwidth-graph-container"
      title={`Peak: ${formatSpeed(maxSpeed)}`}
      onMouseLeave={() => setHoverIndex(null)}
    >
      <div className="bandwidth-graph-header">
        <span className="bandwidth-graph-title">Bandwidth</span>
        <span className="bandwidth-graph-peak">
          {currentPoint
            ? `↓ ${formatSpeed(currentPoint.dl)}`
            : `Peak ${formatSpeed(maxSpeed)}`}
        </span>
      </div>

      <svg
        className="bandwidth-graph-svg"
        viewBox={`0 0 ${width} ${height}`}
        preserveAspectRatio="none"
        onMouseMove={(e) => {
          const rect = e.currentTarget.getBoundingClientRect();
          const relX = Math.max(0, Math.min(width, e.clientX - rect.left));
          const idx = Math.min(
            HISTORY_LENGTH - 1,
            Math.max(0, Math.round((relX / rect.width) * (HISTORY_LENGTH - 1))),
          );
          setHoverIndex(idx);
        }}
      >
        <defs>
          <linearGradient id="dl-gradient" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="var(--accent)" stopOpacity="0.45" />
            <stop offset="100%" stopColor="var(--accent)" stopOpacity="0.0" />
          </linearGradient>
        </defs>

        {/* Grid lines */}
        <line
          x1="0"
          y1={paddingTop + graphHeight * 0.5}
          x2={width}
          y2={paddingTop + graphHeight * 0.5}
          stroke="var(--border)"
          strokeDasharray="2 3"
          strokeWidth="1"
          opacity="0.5"
        />

        {/* Area fill */}
        <path d={areaPath} fill="url(#dl-gradient)" />

        {/* Line curve */}
        <path
          d={linePath}
          fill="none"
          stroke="var(--accent)"
          strokeWidth="1.8"
          strokeLinecap="round"
          strokeLinejoin="round"
        />

        {/* Hover indicator dot */}
        {hoverIndex !== null && points[hoverIndex] && (
          <circle
            cx={points[hoverIndex].x}
            cy={points[hoverIndex].y}
            r="3"
            fill="var(--accent)"
            stroke="var(--surface)"
            strokeWidth="1.5"
          />
        )}
      </svg>
    </div>
  );
}
