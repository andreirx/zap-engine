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
/** Color for GPU timing bars. */
const GPU_COLOR = '#2196F3'; // Blue
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
  const { wasmTimeUs, gpuTimeUs, wasmHistory, gpuHistory } = timing;

  // Convert microseconds to pixels
  const wasmWidth = Math.min(Math.round(wasmTimeUs / usPerPixel), maxWidth);
  const gpuWidth = Math.min(Math.round(gpuTimeUs / usPerPixel), maxWidth);

  // Check if over frame budget (16.67ms = 16670μs for 60 FPS)
  const frameBudgetUs = 16670;
  const totalFrameUs = wasmTimeUs + gpuTimeUs;
  const isOverBudget = totalFrameUs > frameBudgetUs;

  // Generate history canvas data
  const historyCanvas = useMemo(() => {
    if (!showHistory || wasmHistory.length === 0) return null;

    const width = Math.min(wasmHistory.length, maxWidth);
    const height = barHeight * 3; // Space for both bars + gap

    // Find max for scaling
    const maxTime = Math.max(
      ...wasmHistory.slice(-width),
      ...gpuHistory.slice(-width),
      1000, // Minimum scale of 1ms
    );

    return { width, height, maxTime };
  }, [showHistory, wasmHistory, gpuHistory, maxWidth, barHeight]);

  // Collapsed view - just show total time
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
        {(totalFrameUs / 1000).toFixed(1)}ms
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
      <div style={{ display: 'flex', gap: 12, marginBottom: 4 }}>
        <span>
          <span style={{ color: WASM_COLOR }}>WASM</span>:{' '}
          {(wasmTimeUs / 1000).toFixed(2)}ms
        </span>
        <span>
          <span style={{ color: GPU_COLOR }}>GPU</span>:{' '}
          {(gpuTimeUs / 1000).toFixed(2)}ms
        </span>
        <span style={{ color: isOverBudget ? OVER_BUDGET_COLOR : '#888' }}>
          Total: {(totalFrameUs / 1000).toFixed(2)}ms
        </span>
      </div>

      {/* Current frame bars */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
        {/* WASM bar */}
        <div
          style={{
            width: maxWidth,
            height: barHeight,
            background: '#333',
            position: 'relative',
          }}
        >
          <div
            style={{
              width: wasmWidth,
              height: '100%',
              background: WASM_COLOR,
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
              opacity: 0.5,
            }}
          />
        </div>

        {/* GPU bar */}
        <div
          style={{
            width: maxWidth,
            height: barHeight,
            background: '#333',
            position: 'relative',
          }}
        >
          <div
            style={{
              width: gpuWidth,
              height: '100%',
              background: GPU_COLOR,
            }}
          />
          <div
            style={{
              position: 'absolute',
              left: Math.round(frameBudgetUs / usPerPixel),
              top: 0,
              width: 1,
              height: '100%',
              background: '#FF5722',
              opacity: 0.5,
            }}
          />
        </div>
      </div>

      {/* History visualization */}
      {showHistory && historyCanvas && (
        <div style={{ marginTop: 6 }}>
          <div style={{ fontSize: 9, color: '#888', marginBottom: 2 }}>
            History ({wasmHistory.length} frames)
          </div>
          <svg
            width={historyCanvas.width}
            height={historyCanvas.height}
            style={{ background: '#222', borderRadius: 2 }}
          >
            {/* Render WASM history bars */}
            {wasmHistory.slice(-historyCanvas.width).map((val, i) => {
              const h = Math.max(1, (val / historyCanvas.maxTime) * barHeight);
              return (
                <rect
                  key={`w${i}`}
                  x={i}
                  y={barHeight - h}
                  width={1}
                  height={h}
                  fill={WASM_COLOR}
                  opacity={0.8}
                />
              );
            })}
            {/* Render GPU history bars below WASM */}
            {gpuHistory.slice(-historyCanvas.width).map((val, i) => {
              const h = Math.max(1, (val / historyCanvas.maxTime) * barHeight);
              return (
                <rect
                  key={`g${i}`}
                  x={i}
                  y={barHeight + 2 + (barHeight - h)}
                  width={1}
                  height={h}
                  fill={GPU_COLOR}
                  opacity={0.8}
                />
              );
            })}
            {/* Combined frame time at bottom */}
            {wasmHistory.slice(-historyCanvas.width).map((wasm, i) => {
              const gpu = gpuHistory[gpuHistory.length - historyCanvas.width + i] ?? 0;
              const total = wasm + gpu;
              const h = Math.max(1, (total / (historyCanvas.maxTime * 2)) * barHeight);
              const overBudget = total > frameBudgetUs;
              return (
                <rect
                  key={`t${i}`}
                  x={i}
                  y={(barHeight + 2) * 2 + (barHeight - h)}
                  width={1}
                  height={h}
                  fill={overBudget ? OVER_BUDGET_COLOR : '#888'}
                  opacity={0.6}
                />
              );
            })}
          </svg>
          <div style={{ fontSize: 9, color: '#666', marginTop: 2 }}>
            Scale: {(historyCanvas.maxTime / 1000).toFixed(1)}ms |{' '}
            {usPerPixel}μs/px
          </div>
        </div>
      )}
    </div>
  );
}
