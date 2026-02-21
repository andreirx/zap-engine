// Chemistry Lab — 3D molecule builder with VSEPR geometry.
// Click to place atoms, drag between atoms to create bonds.
// Drag on empty space to rotate the view.

import { useState, useCallback, useMemo, useEffect } from 'react';
import { useZapEngine, TimingBars } from '@zap/web/react';
import type { GameEvent } from '@zap/web/react';
import { CameraControls } from './CameraControls';

const WASM_URL = '/examples/chemistry-lab/pkg/chemistry_lab.js';
const ASSETS_URL = '/examples/chemistry-lab/public/assets/assets.json';
const PERIODIC_TABLE_URL = '/examples/chemistry-lab/public/assets/periodic-table.json';

interface ElementInfo {
  number: number;
  symbol: string;
  name: string;
  category: string;
  atomic_mass: number;
  shells: number[];
  'cpk-hex'?: string;
  xpos: number;
  ypos: number;
}

// Category colors
const CATEGORY_COLORS: Record<string, string> = {
  'alkali metal': '#ff6b6b',
  'alkaline earth metal': '#feca57',
  'transition metal': '#48dbfb',
  'post-transition metal': '#1dd1a1',
  'metalloid': '#5f27cd',
  'diatomic nonmetal': '#00d2d3',
  'polyatomic nonmetal': '#00d2d3',
  'halogen': '#ff9f43',
  'noble gas': '#ff9ff3',
  'lanthanide': '#54a0ff',
  'actinide': '#c8d6e5',
};

function getCategoryColor(category: string): string {
  return CATEGORY_COLORS[category.toLowerCase()] || '#576574';
}

// VSEPR geometry names
const VSEPR_NAMES: Record<number, string> = {
  0: 'None',
  1: 'Terminal',
  2: 'Linear (180°)',
  3: 'Trigonal Planar (120°)',
  4: 'Tetrahedral (109.5°)',
  5: 'Trigonal Bipyramidal',
  6: 'Octahedral (90°)',
};

// Bohr model SVG component
function BohrModel({ shells, symbol, color, size = 120 }: {
  shells: number[];
  symbol: string;
  color: string;
  size?: number;
}) {
  const cx = size / 2;
  const cy = size / 2;
  const nucleusRadius = 12;
  const maxShellRadius = (size / 2) - 8;
  const shellGap = shells.length > 0 ? (maxShellRadius - nucleusRadius) / shells.length : 0;

  return (
    <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
      {/* Electron shells (orbits) */}
      {shells.map((electrons, shellIndex) => {
        const shellRadius = nucleusRadius + shellGap * (shellIndex + 1);
        const electronRadius = 3;
        return (
          <g key={shellIndex}>
            {/* Shell orbit circle */}
            <circle
              cx={cx}
              cy={cy}
              r={shellRadius}
              fill="none"
              stroke="rgba(100,150,255,0.3)"
              strokeWidth={1}
              strokeDasharray="4 2"
            />
            {/* Electrons on this shell */}
            {Array.from({ length: electrons }).map((_, eIndex) => {
              const angle = (2 * Math.PI * eIndex) / electrons - Math.PI / 2;
              const ex = cx + shellRadius * Math.cos(angle);
              const ey = cy + shellRadius * Math.sin(angle);
              return (
                <circle
                  key={eIndex}
                  cx={ex}
                  cy={ey}
                  r={electronRadius}
                  fill="#4fc3f7"
                />
              );
            })}
          </g>
        );
      })}
      {/* Nucleus */}
      <circle cx={cx} cy={cy} r={nucleusRadius} fill={color} />
      <text
        x={cx}
        y={cy + 4}
        textAnchor="middle"
        fill="#000"
        fontSize={10}
        fontWeight="bold"
        fontFamily="monospace"
      >
        {symbol}
      </text>
    </svg>
  );
}

export function App() {
  const [selected, setSelected] = useState(6); // Carbon default
  const [atomCount, setAtomCount] = useState(0);
  const [bondCount, setBondCount] = useState(0);
  const [selectedAtomElement, setSelectedAtomElement] = useState<number | null>(null);
  const [vsperBondCount, setVsperBondCount] = useState(0);
  const [elements, setElements] = useState<ElementInfo[]>([]);
  const [hoveredElement, setHoveredElement] = useState<ElementInfo | null>(null);
  const [timingCollapsed, setTimingCollapsed] = useState(true);

  // Load periodic table data
  useEffect(() => {
    fetch(PERIODIC_TABLE_URL)
      .then(r => r.json())
      .then(data => setElements(data.elements))
      .catch(err => console.error('Failed to load periodic table:', err));
  }, []);
  const [tooltipPos, setTooltipPos] = useState<{ x: number; y: number } | null>(null);

  const onGameEvent = useCallback((events: GameEvent[]) => {
    for (const e of events) {
      if (e.kind === 1) setAtomCount(e.a);
      if (e.kind === 2) setBondCount(e.a);
      if (e.kind === 3) setSelectedAtomElement(e.a);
      if (e.kind === 4) setVsperBondCount(e.b);
    }
  }, []);

  const { canvasRef, sendEvent, fps, isReady, canvasKey, timing } = useZapEngine({
    wasmUrl: WASM_URL,
    assetsUrl: ASSETS_URL,
    gameWidth: 800,
    gameHeight: 600,
    onGameEvent,
  });

  // ── Wheel-to-zoom with cursor position (non-passive to allow preventDefault) ──────────
  // Custom event kind 10 = CAMERA_ZOOM, a = direction, b = normalized x, c = normalized y
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !sendEvent) return;
    const handler = (e: WheelEvent) => {
      e.preventDefault();
      const rect = canvas.getBoundingClientRect();
      const fx = (e.clientX - rect.left) / rect.width;  // 0-1 normalized
      const fy = (e.clientY - rect.top) / rect.height;
      const direction = e.deltaY < 0 ? 1 : -1; // scroll up = zoom in
      sendEvent({ type: 'custom', kind: 10, a: direction, b: fx, c: fy });
    };
    canvas.addEventListener('wheel', handler, { passive: false });
    return () => canvas.removeEventListener('wheel', handler);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sendEvent, canvasKey]);

  const selectElement = (atomicNumber: number) => {
    setSelected(atomicNumber);
    sendEvent({ type: 'custom', kind: 1, a: atomicNumber });
  };

  const handleClear = () => {
    sendEvent({ type: 'custom', kind: 2 });
    setSelectedAtomElement(null);
  };

  const handleResetCamera = () => {
    sendEvent({ type: 'custom', kind: 3 });
  };

  // Build periodic table grid (18 columns x 7 rows main + 2 for lanthanides/actinides)
  const periodicGrid = useMemo(() => {
    const grid: (ElementInfo | null)[][] = Array(9).fill(null).map(() => Array(18).fill(null));
    for (const el of elements) {
      const x = el.xpos - 1;
      const y = el.ypos - 1;
      if (x >= 0 && x < 18 && y >= 0 && y < 9) {
        grid[y][x] = el;
      }
    }
    return grid;
  }, [elements]);

  const selectedElementInfo = elements.find(e => e.number === selected);
  const selectedAtomInfo = selectedAtomElement ? elements.find(e => e.number === selectedAtomElement) : null;
  const displayElement = hoveredElement || selectedElementInfo;
  const vsperGeometry = VSEPR_NAMES[vsperBondCount] || `${vsperBondCount} bonds`;

  return (
    <div style={{
      display: 'flex',
      flexDirection: 'column',
      width: '100vw',
      height: '100vh',
      background: '#0a0a15',
      overflow: 'hidden',
    }}>
      {/* Periodic Table - Always Visible */}
      <div style={{
        background: '#12121f',
        padding: '12px',
        borderBottom: '1px solid #2a2a3a',
        overflowX: 'auto',
        flexShrink: 0,
        position: 'relative',
      }}>
        <div style={{
          display: 'flex',
          gap: 16,
          alignItems: 'flex-start',
          justifyContent: 'center',
        }}>
          {/* Periodic table grid - LARGER */}
          <div style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(18, 34px)',
            gridTemplateRows: 'repeat(9, 34px)',
            gap: 2,
            flexShrink: 0,
          }}>
            {periodicGrid.map((row, y) =>
              row.map((el, x) => {
                if (!el) {
                  return <div key={`${x}-${y}`} style={{ width: 34, height: 34 }} />;
                }
                const isSelected = el.number === selected;
                return (
                  <button
                    key={el.number}
                    onClick={() => selectElement(el.number)}
                    onMouseEnter={(e) => {
                      setHoveredElement(el);
                      const rect = e.currentTarget.getBoundingClientRect();
                      setTooltipPos({ x: rect.left + rect.width / 2, y: rect.bottom + 8 });
                    }}
                    onMouseLeave={() => {
                      setHoveredElement(null);
                      setTooltipPos(null);
                    }}
                    style={{
                      width: 34,
                      height: 34,
                      border: isSelected ? '2px solid #fff' : '1px solid rgba(255,255,255,0.1)',
                      borderRadius: 4,
                      background: getCategoryColor(el.category),
                      color: '#000',
                      fontFamily: 'monospace',
                      fontSize: 12,
                      fontWeight: 'bold',
                      cursor: 'pointer',
                      padding: 0,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      boxShadow: isSelected ? '0 0 8px rgba(255,255,255,0.6)' : 'none',
                      transition: 'transform 0.1s',
                    }}
                    onMouseDown={(e) => (e.currentTarget.style.transform = 'scale(0.95)')}
                    onMouseUp={(e) => (e.currentTarget.style.transform = 'scale(1)')}
                  >
                    {el.symbol}
                  </button>
                );
              })
            )}
          </div>

          {/* Controls */}
          <div style={{
            display: 'flex',
            flexDirection: 'column',
            gap: 6,
            flexShrink: 0,
          }}>
            <div style={{
              color: '#fff',
              fontFamily: 'monospace',
              fontSize: 11,
              background: 'rgba(0,0,0,0.3)',
              padding: '4px 8px',
              borderRadius: 4,
            }}>
              {atomCount} atoms &bull; {bondCount} bonds
            </div>
            <div style={{ display: 'flex', gap: 4 }}>
              <button
                onClick={handleResetCamera}
                style={{
                  fontFamily: 'monospace',
                  fontSize: 10,
                  padding: '4px 8px',
                  borderRadius: 4,
                  border: '1px solid #444',
                  background: '#2a2a3a',
                  color: '#fff',
                  cursor: 'pointer',
                }}
              >
                Reset
              </button>
              <button
                onClick={handleClear}
                style={{
                  fontFamily: 'monospace',
                  fontSize: 10,
                  padding: '4px 8px',
                  borderRadius: 4,
                  border: 'none',
                  background: '#c0392b',
                  color: '#fff',
                  cursor: 'pointer',
                }}
              >
                Clear
              </button>
            </div>
          </div>
        </div>

        {/* Floating tooltip for hovered element */}
        {hoveredElement && tooltipPos && (
          <div style={{
            position: 'fixed',
            left: tooltipPos.x,
            top: tooltipPos.y,
            transform: 'translateX(-50%)',
            background: 'rgba(0,0,0,0.95)',
            borderRadius: 8,
            padding: 12,
            color: '#fff',
            fontFamily: 'system-ui',
            fontSize: 12,
            zIndex: 1000,
            pointerEvents: 'none',
            boxShadow: '0 4px 20px rgba(0,0,0,0.5)',
            minWidth: 180,
          }}>
            <div style={{ display: 'flex', gap: 10, alignItems: 'center', marginBottom: 8 }}>
              <div style={{
                width: 40,
                height: 40,
                borderRadius: 6,
                background: getCategoryColor(hoveredElement.category),
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                fontFamily: 'monospace',
                fontWeight: 'bold',
                fontSize: 18,
                color: '#000',
              }}>
                {hoveredElement.symbol}
              </div>
              <div>
                <div style={{ fontWeight: 'bold', fontSize: 14 }}>{hoveredElement.name}</div>
                <div style={{ fontSize: 11, opacity: 0.7 }}>
                  #{hoveredElement.number} &bull; {hoveredElement.atomic_mass.toFixed(3)} u
                </div>
              </div>
            </div>
            <div style={{ fontSize: 11, opacity: 0.8, marginBottom: 4 }}>
              {hoveredElement.category}
            </div>
            <div style={{ fontSize: 11 }}>
              <span style={{ opacity: 0.6 }}>Electron shells:</span>{' '}
              <span style={{ fontFamily: 'monospace' }}>[{hoveredElement.shells.join(', ')}]</span>
            </div>
            <div style={{ fontSize: 11, color: '#4fc3f7', marginTop: 4 }}>
              Valence: {hoveredElement.shells[hoveredElement.shells.length - 1]} electrons
            </div>
          </div>
        )}
      </div>

      {/* Canvas area */}
      <div style={{ flex: 1, position: 'relative', minHeight: 0 }}>
        <canvas
          key={canvasKey}
          ref={canvasRef}
          style={{ width: '100%', height: '100%', display: 'block' }}
        />

        {/* Camera Controls - Left side */}
        {isReady && (
          <CameraControls
            sendEvent={sendEvent}
            style={{
              position: 'absolute',
              top: 12,
              left: 12,
            }}
          />
        )}

        {/* Selected atom info with Bohr model and VSEPR */}
        {selectedAtomInfo && (
          <div style={{
            position: 'absolute',
            top: 12,
            right: 12,
            background: 'rgba(0,0,0,0.85)',
            borderRadius: 8,
            padding: 12,
            color: '#fff',
            fontFamily: 'system-ui',
            fontSize: 12,
            minWidth: 160,
          }}>
            {/* Header with symbol and name */}
            <div style={{ display: 'flex', gap: 10, alignItems: 'center', marginBottom: 8 }}>
              <div style={{
                width: 36,
                height: 36,
                borderRadius: 4,
                background: getCategoryColor(selectedAtomInfo.category),
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                fontFamily: 'monospace',
                fontWeight: 'bold',
                fontSize: 16,
                color: '#000',
              }}>
                {selectedAtomInfo.symbol}
              </div>
              <div>
                <div style={{ fontWeight: 'bold', fontSize: 14 }}>{selectedAtomInfo.name}</div>
                <div style={{ fontSize: 10, opacity: 0.7 }}>
                  #{selectedAtomInfo.number} &bull; {selectedAtomInfo.atomic_mass.toFixed(2)} u
                </div>
              </div>
            </div>

            {/* Bohr model */}
            <div style={{
              background: 'rgba(20,20,40,0.8)',
              borderRadius: 6,
              padding: 8,
              display: 'flex',
              justifyContent: 'center',
              marginBottom: 8,
            }}>
              <BohrModel
                shells={selectedAtomInfo.shells}
                symbol={selectedAtomInfo.symbol}
                color={getCategoryColor(selectedAtomInfo.category)}
                size={130}
              />
            </div>

            {/* Electron configuration */}
            <div style={{
              fontSize: 11,
              padding: '6px 8px',
              background: 'rgba(255,255,255,0.1)',
              borderRadius: 4,
              marginBottom: 6,
            }}>
              <div style={{ opacity: 0.7, marginBottom: 2 }}>Electron shells:</div>
              <div style={{ fontFamily: 'monospace' }}>
                {selectedAtomInfo.shells.map((e, i) => (
                  <span key={i} style={{ marginRight: 4 }}>
                    {i + 1}:{e}
                  </span>
                ))}
              </div>
              <div style={{ marginTop: 4, color: '#4fc3f7' }}>
                Valence electrons: {selectedAtomInfo.shells[selectedAtomInfo.shells.length - 1]}
              </div>
            </div>

            {/* VSEPR geometry */}
            {vsperBondCount > 0 && (
              <div style={{
                padding: '6px 8px',
                background: 'rgba(100,150,255,0.2)',
                borderRadius: 4,
                fontSize: 11,
              }}>
                <div style={{ fontWeight: 'bold' }}>VSEPR: {vsperGeometry}</div>
                <div style={{ opacity: 0.7, marginTop: 2 }}>
                  {vsperBondCount} bond{vsperBondCount !== 1 ? 's' : ''} formed
                </div>
              </div>
            )}
          </div>
        )}

        {/* Instructions */}
        {isReady && (
          <div style={{
            position: 'absolute',
            bottom: 8,
            left: '50%',
            transform: 'translateX(-50%)',
            color: 'rgba(255,255,255,0.5)',
            fontFamily: 'monospace',
            fontSize: 11,
            background: 'rgba(0,0,0,0.5)',
            padding: '4px 10px',
            borderRadius: 4,
            whiteSpace: 'nowrap',
          }}>
            Click: place atom &bull; Drag atom→atom: bond &bull; Drag empty: rotate
          </div>
        )}

        {/* Performance Timing */}
        {isReady && (
          <div style={{
            position: 'absolute',
            bottom: 8,
            right: 8,
          }}>
            <div style={{
              color: 'rgba(255,255,255,0.5)',
              fontFamily: 'monospace',
              fontSize: 10,
              textAlign: 'right',
              marginBottom: 4,
            }}>
              {fps} FPS
            </div>
            <TimingBars
              timing={timing}
              usPerPixel={50}
              maxWidth={150}
              barHeight={6}
              collapsed={timingCollapsed}
              onToggle={() => setTimingCollapsed(!timingCollapsed)}
            />
          </div>
        )}
      </div>

      {/* Attribution */}
      <div style={{
        background: '#12121f',
        padding: '4px 8px',
        textAlign: 'center',
        color: 'rgba(255,255,255,0.3)',
        fontFamily: 'system-ui',
        fontSize: 9,
        borderTop: '1px solid #2a2a3a',
      }}>
        Data from{' '}
        <a
          href="https://github.com/Bowserinator/Periodic-Table-JSON"
          target="_blank"
          rel="noopener noreferrer"
          style={{ color: 'rgba(255,255,255,0.5)' }}
        >
          Periodic-Table-JSON
        </a>
        {' '}(CC-BY-A)
      </div>
    </div>
  );
}
