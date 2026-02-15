// TimingBars.tsx — Visual performance profiling component.
//
// Displays horizontal bars where each pixel represents a configurable time resolution.
// Shows WASM computation time and GPU render time side by side.

import { CSSProperties, useMemo } from 'react';
import type { PerformanceTiming } from './useZapEngine';

export interface TimingBarsProps {
  /** Timing data from useZapEngine hook. */
  timing: PerformanceTiming;
  /** Microseconds per pixel (default: 100). Lower = more zoomed in. */
  usPerPixel?: number;
  /** Maximum bar width in pixels (default: 200). */
  maxWidth?: number;
  /** Height of each timing bar in pixels (default: 8). */
  barHeight?: number;
  /** Whether to show the history bars (default: true). */
  showHistory?: boolean;
  /** Optional style overrides for the container. */
  style?: CSSProperties;
  /** Callback when panel is clicked (for hide/show toggle). */
  onToggle?: () => void;
  /** If true, only show a minimal collapsed view. */
  collapsed?: boolean;
}

/** Color for WASM timing bars. */
const WASM_COLOR = '#4CAF50'; // Green
/** Color for GPU/Draw timing bars. */
const GPU_COLOR = '#2196F3'; // Blue
/** Color for idle/wait time (compositor, VSync, rasterization). */
const IDLE_COLOR = '#9E9E9E'; // Gray
/** Color for frame time exceeding budget (>16.67ms = 60fps). */
const OVER_BUDGET_COLOR = '#FF5722'; // Orange-red

/**
 * Renders timing bars for performance visualization.
 *
 * Shows:
 * - Current frame WASM and GPU times as bars
 * - Rolling history as a mini chart (each pixel column = 1 frame)
 * - Frame time budget line at 16.67ms (60 FPS target)
 */
export function TimingBars({
  timing,
  usPerPixel = 100,
  maxWidth = 200,
  barHeight = 8,
  showHistory = true,
  style,
  onToggle,
  collapsed = false,
}: TimingBarsProps) {
  const {
    wasmTimeUs, drawTimeUs, frameTimeUs,
    wasmHistory, drawHistory, frameHistory,
  } = timing;

  // Convert microseconds to pixels
  const wasmWidth = Math.min(Math.round(wasmTimeUs / usPerPixel), maxWidth);
  const drawWidth = Math.min(Math.round(drawTimeUs / usPerPixel), maxWidth);

  // Raster time = frame time - measured work (WASM + Draw)
  // On Canvas2D this is software rasterization, on WebGPU it's GPU + VSync
  const measuredUs = wasmTimeUs + drawTimeUs;
  const rasterTimeUs = Math.max(0, frameTimeUs - measuredUs);
  const rasterWidth = Math.min(Math.round(rasterTimeUs / usPerPixel), maxWidth);

  // Check if over frame budget (16.67ms = 16670μs for 60 FPS)
  const frameBudgetUs = 16670;
  const isOverBudget = frameTimeUs > frameBudgetUs;

  // Generate history canvas data
  const historyCanvas = useMemo(() => {
    if (!showHistory || frameHistory.length === 0) return null;

    const width = Math.min(frameHistory.length, maxWidth);
    const height = barHeight * 2; // Just one row for frame time breakdown

    // Find max frame time for scaling (use actual frame times, not just measured)
    const maxTime = Math.max(
      ...frameHistory.slice(-width),
      frameBudgetUs, // At least show the 60fps budget line
    );

    return { width, height, maxTime };
  }, [showHistory, frameHistory, maxWidth, barHeight]);

  // Collapsed view - just show frame time
  if (collapsed) {
    return (
      <div
        onClick={onToggle}
        style={{
          fontFamily: 'monospace',
          fontSize: 10,
          color: '#888',
          background: 'rgba(0,0,0,0.5)',
          padding: '4px 8px',
          borderRadius: 4,
          cursor: onToggle ? 'pointer' : undefined,
          ...style,
        }}
      >
        {(frameTimeUs / 1000).toFixed(1)}ms
      </div>
    );
  }

  return (
    <div
      onClick={onToggle}
      style={{
        fontFamily: 'monospace',
        fontSize: 10,
        color: '#fff',
        background: 'rgba(0,0,0,0.7)',
        padding: 8,
        borderRadius: 4,
        cursor: onToggle ? 'pointer' : undefined,
        ...style,
      }}
    >
      {/* Labels and current values */}
      <div style={{ display: 'flex', gap: 8, marginBottom: 4, flexWrap: 'wrap' }}>
        <span>
          <span style={{ color: WASM_COLOR }}>WASM</span>:{' '}
          {(wasmTimeUs / 1000).toFixed(2)}ms
        </span>
        <span>
          <span style={{ color: GPU_COLOR }}>Draw</span>:{' '}
          {(drawTimeUs / 1000).toFixed(2)}ms
        </span>
        <span>
          <span style={{ color: OVER_BUDGET_COLOR }}>Raster</span>:{' '}
          {(rasterTimeUs / 1000).toFixed(1)}ms
        </span>
        <span style={{ color: isOverBudget ? OVER_BUDGET_COLOR : '#fff' }}>
          Frame: {(frameTimeUs / 1000).toFixed(1)}ms
        </span>
      </div>

      {/* Current frame bars - stacked horizontally to show breakdown */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
        {/* Combined frame breakdown bar */}
        <div
          style={{
            width: maxWidth,
            height: barHeight,
            background: '#333',
            position: 'relative',
            display: 'flex',
          }}
        >
          {/* WASM portion (green) */}
          <div
            style={{
              width: wasmWidth,
              height: '100%',
              background: WASM_COLOR,
              flexShrink: 0,
            }}
          />
          {/* Draw call portion (blue) */}
          <div
            style={{
              width: drawWidth,
              height: '100%',
              background: GPU_COLOR,
              flexShrink: 0,
            }}
          />
          {/* Raster portion (orange - software rasterization on Canvas2D, GPU+VSync on WebGPU) */}
          <div
            style={{
              width: rasterWidth,
              height: '100%',
              background: OVER_BUDGET_COLOR,
              flexShrink: 0,
            }}
          />
          {/* 16.67ms budget line */}
          <div
            style={{
              position: 'absolute',
              left: Math.round(frameBudgetUs / usPerPixel),
              top: 0,
              width: 1,
              height: '100%',
              background: '#FF5722',
              opacity: 0.7,
            }}
          />
        </div>
      </div>

      {/* History visualization */}
      {showHistory && historyCanvas && (
        <div style={{ marginTop: 6 }}>
          <div style={{ fontSize: 9, color: '#888', marginBottom: 2 }}>
            History ({frameHistory.length} frames)
          </div>
          <svg
            width={historyCanvas.width}
            height={historyCanvas.height}
            style={{ background: '#222', borderRadius: 2 }}
          >
            {/* 60 FPS budget line */}
            <line
              x1={0}
              y1={historyCanvas.height - (frameBudgetUs / historyCanvas.maxTime) * historyCanvas.height}
              x2={historyCanvas.width}
              y2={historyCanvas.height - (frameBudgetUs / historyCanvas.maxTime) * historyCanvas.height}
              stroke="#fff"
              strokeWidth={1}
              opacity={0.3}
            />
            {/* Render frame time bars - stacked: WASM, Draw, Raster */}
            {frameHistory.slice(-historyCanvas.width).map((frame, i) => {
              const startIdx = frameHistory.length - historyCanvas.width;
              const wasm = wasmHistory[startIdx + i] ?? 0;
              const draw = drawHistory[startIdx + i] ?? 0;
              const raster = Math.max(0, frame - wasm - draw);

              const scale = historyCanvas.height / historyCanvas.maxTime;
              const wasmH = wasm * scale;
              const drawH = draw * scale;
              const rasterH = raster * scale;

              let y = historyCanvas.height;

              return (
                <g key={i}>
                  {/* WASM at bottom (green) */}
                  <rect
                    x={i}
                    y={(y -= wasmH)}
                    width={1}
                    height={Math.max(0.5, wasmH)}
                    fill={WASM_COLOR}
                    opacity={0.9}
                  />
                  {/* Draw above WASM (blue) */}
                  <rect
                    x={i}
                    y={(y -= drawH)}
                    width={1}
                    height={Math.max(0.5, drawH)}
                    fill={GPU_COLOR}
                    opacity={0.9}
                  />
                  {/* Raster at top (orange - the bottleneck on Canvas2D) */}
                  <rect
                    x={i}
                    y={(y -= rasterH)}
                    width={1}
                    height={Math.max(0.5, rasterH)}
                    fill={OVER_BUDGET_COLOR}
                    opacity={0.9}
                  />
                </g>
              );
            })}
          </svg>
          <div style={{ fontSize: 9, color: '#666', marginTop: 2 }}>
            Max: {(historyCanvas.maxTime / 1000).toFixed(1)}ms | 60fps budget line
          </div>
        </div>
      )}
    </div>
  );
}
