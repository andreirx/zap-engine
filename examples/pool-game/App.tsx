// Pool Game - 2D billiards with Rapier2D physics and SDF ball rendering.

import { useState, useCallback, useEffect } from 'react';
import { useZapEngine, TimingBars } from '@zap/web/react';
import type { GameEvent } from '@zap/web/react';

const WASM_URL = '/examples/pool-game/pkg/pool_game.js';
const ASSETS_URL = '/examples/pool-game/public/assets/assets.json';

export function App() {
  const [ballsRemaining, setBallsRemaining] = useState(15);
  const [timingCollapsed, setTimingCollapsed] = useState(false);
  const [isPortrait, setIsPortrait] = useState(false);
  const [containerSize, setContainerSize] = useState({ width: 0, height: 0 });

  // World aspect ratio: 1160:660 ≈ 1.76:1
  const ASPECT = 1160 / 660;

  // Detect portrait mode and compute container size
  useEffect(() => {
    function updateLayout() {
      const vw = window.innerWidth;
      const vh = window.innerHeight;
      const portrait = vh > vw;
      setIsPortrait(portrait);

      if (portrait) {
        // Portrait: rotate canvas, so container is tall (inverted aspect)
        // Container aspect = 800:1300 = 0.615:1
        const maxW = vw * 0.95;
        const maxH = vh * 0.92;
        // For inverted aspect: height = width / 0.615 = width * 1.625
        const heightFromWidth = maxW * ASPECT;
        const widthFromHeight = maxH / ASPECT;
        const width = heightFromWidth <= maxH ? maxW : widthFromHeight;
        const height = width * ASPECT;
        setContainerSize({ width, height });
      } else {
        // Landscape: fill viewport as much as possible
        const maxW = vw * 0.98;
        const maxH = vh * 0.92;
        // For aspect 1.625:1: width = height * 1.625
        const widthFromHeight = maxH * ASPECT;
        const width = Math.min(maxW, widthFromHeight);
        const height = width / ASPECT;
        setContainerSize({ width, height });
      }
    }
    updateLayout();
    window.addEventListener('resize', updateLayout);
    return () => window.removeEventListener('resize', updateLayout);
  }, []);

  const onGameEvent = useCallback((events: GameEvent[]) => {
    for (const e of events) {
      if (e.kind === 1) {  // BALLS_REMAINING
        setBallsRemaining(e.a);
      }
    }
  }, []);

  // World dimensions: 1160x660 (table 1000x500 centered with 80px margin)
  const { canvasRef, sendEvent, fps, isReady, canvasKey, timing } = useZapEngine({
    wasmUrl: WASM_URL,
    assetsUrl: ASSETS_URL,
    gameWidth: 1160,
    gameHeight: 660,
    onGameEvent,
  });

  const handleReset = () => {
    sendEvent({ type: 'custom', kind: 1 });  // RESET event
  };

  return (
    <div style={{
      position: 'relative',
      width: '100vw',
      height: '100vh',
      background: '#1a1a2e',
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      justifyContent: 'center',
      overflow: 'hidden',
    }}>
      {/* HUD - positioned at viewport edges */}
      <div style={{
        position: 'fixed',
        top: 12,
        left: 16,
        display: 'flex',
        gap: 12,
        alignItems: 'center',
        zIndex: 10,
      }}>
        <div style={{
          color: '#fff',
          fontFamily: 'monospace',
          fontSize: 16,
          background: 'rgba(0,0,0,0.7)',
          padding: '6px 14px',
          borderRadius: 6,
        }}>
          Balls: {ballsRemaining}
        </div>
        <button
          onClick={handleReset}
          style={{
            fontFamily: 'monospace',
            fontSize: 14,
            padding: '6px 14px',
            borderRadius: 6,
            border: 'none',
            background: '#2d5a27',
            color: '#fff',
            cursor: 'pointer',
          }}
        >
          Reset
        </button>
      </div>

      {/* FPS and timing - positioned at viewport edge */}
      <div style={{
        position: 'fixed',
        top: 12,
        right: 16,
        zIndex: 10,
      }}>
        <div style={{
          color: '#fff',
          fontFamily: 'monospace',
          fontSize: 14,
          textAlign: 'right',
          marginBottom: 4,
        }}>
          {isReady ? `${fps} FPS` : 'Loading...'}
        </div>
        {isReady && (
          <TimingBars
            timing={timing}
            usPerPixel={50}
            maxWidth={150}
            barHeight={6}
            collapsed={timingCollapsed}
            onToggle={() => setTimingCollapsed(!timingCollapsed)}
          />
        )}
      </div>

      {/* Canvas container */}
      <div style={{
        position: 'relative',
        width: containerSize.width,
        height: containerSize.height,
      }}>
        {/* Canvas wrapper - rotates only the canvas in portrait mode */}
        <div style={{
          position: 'absolute',
          ...(isPortrait ? {
            // Portrait: rotate canvas 90° CW
            width: containerSize.height,
            height: containerSize.width,
            left: (containerSize.width - containerSize.height) / 2,
            top: (containerSize.height - containerSize.width) / 2,
            transform: 'rotate(90deg)',
            transformOrigin: 'center center',
          } : {
            // Landscape: fill container normally
            inset: 0,
          }),
        }}>
          <canvas
            key={canvasKey}
            ref={canvasRef}
            style={{
              width: '100%',
              height: '100%',
              display: 'block',
            }}
          />
        </div>
      </div>

      {/* Instructions - positioned at viewport bottom */}
      {isReady && (
        <div style={{
          position: 'fixed',
          bottom: 12,
          left: 0,
          right: 0,
          color: 'rgba(255,255,255,0.5)',
          fontFamily: 'monospace',
          fontSize: 12,
          textAlign: 'center',
        }}>
          Drag anywhere to aim, release to shoot
        </div>
      )}
    </div>
  );
}
