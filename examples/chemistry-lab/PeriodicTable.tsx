// Full Periodic Table modal component for chemistry-lab.
// Data from Periodic-Table-JSON (https://github.com/Bowserinator/Periodic-Table-JSON), CC-BY-A license.

import { useState, useMemo } from 'react';

export interface ElementInfo {
  number: number;
  symbol: string;
  name: string;
  category: string;
  atomic_mass: number;
  shells: number[];
  'cpk-hex'?: string;
  xpos: number;
  ypos: number;
  period: number;
  group?: number;
}

interface PeriodicTableProps {
  elements: ElementInfo[];
  selectedElement: number;
  onSelect: (atomicNumber: number) => void;
  onClose: () => void;
}

// Category colors matching ElementCategory::ui_color() in Rust
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
  'unknown, probably transition metal': '#576574',
  'unknown, probably post-transition metal': '#576574',
  'unknown, probably metalloid': '#576574',
  'unknown, predicted to be noble gas': '#576574',
  'unknown': '#576574',
};

function getCategoryColor(category: string): string {
  return CATEGORY_COLORS[category.toLowerCase()] || '#576574';
}

function getCPKColor(cpkHex?: string): string {
  if (!cpkHex || cpkHex.length < 6) return '#cccccc';
  return `#${cpkHex}`;
}

export function PeriodicTable({ elements, selectedElement, onSelect, onClose }: PeriodicTableProps) {
  const [hoveredElement, setHoveredElement] = useState<ElementInfo | null>(null);

  // Build grid: 18 columns, 10 rows (7 main + 2 lanthanides/actinides + 1 gap)
  const grid = useMemo(() => {
    const cells: (ElementInfo | null)[][] = Array(10).fill(null).map(() => Array(18).fill(null));
    for (const el of elements) {
      // xpos: 1-18, ypos: 1-7 for main table, 8-9 for lanthanides/actinides
      const x = el.xpos - 1;
      const y = el.ypos - 1;
      if (x >= 0 && x < 18 && y >= 0 && y < 10) {
        cells[y][x] = el;
      }
    }
    return cells;
  }, [elements]);

  const displayedElement = hoveredElement || elements.find(e => e.number === selectedElement);

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        background: 'rgba(0,0,0,0.9)',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1000,
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div style={{
        background: '#1a1a2e',
        borderRadius: 12,
        padding: 20,
        maxWidth: '95vw',
        maxHeight: '95vh',
        overflow: 'auto',
      }}>
        <div style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          marginBottom: 16,
        }}>
          <h2 style={{ color: '#fff', margin: 0, fontFamily: 'system-ui' }}>
            Periodic Table of Elements
          </h2>
          <button
            onClick={onClose}
            style={{
              background: 'transparent',
              border: 'none',
              color: '#fff',
              fontSize: 24,
              cursor: 'pointer',
              padding: '0 8px',
            }}
          >
            &times;
          </button>
        </div>

        {/* Element info panel */}
        {displayedElement && (
          <div style={{
            background: 'rgba(255,255,255,0.1)',
            borderRadius: 8,
            padding: 12,
            marginBottom: 16,
            display: 'flex',
            gap: 20,
            alignItems: 'center',
          }}>
            <div style={{
              width: 60,
              height: 60,
              borderRadius: 8,
              background: getCPKColor(displayedElement['cpk-hex']),
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              fontFamily: 'monospace',
              fontWeight: 'bold',
              fontSize: 24,
              color: '#000',
            }}>
              {displayedElement.symbol}
            </div>
            <div style={{ color: '#fff', fontFamily: 'system-ui' }}>
              <div style={{ fontSize: 18, fontWeight: 'bold' }}>
                {displayedElement.number}. {displayedElement.name}
              </div>
              <div style={{ fontSize: 14, opacity: 0.8 }}>
                Mass: {displayedElement.atomic_mass.toFixed(3)} u
              </div>
              <div style={{ fontSize: 14, opacity: 0.8 }}>
                Shells: [{displayedElement.shells.join(', ')}]
              </div>
              <div style={{ fontSize: 12, opacity: 0.6, textTransform: 'capitalize' }}>
                {displayedElement.category}
              </div>
            </div>
          </div>
        )}

        {/* Periodic table grid */}
        <div style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(18, 36px)',
          gridTemplateRows: 'repeat(10, 36px)',
          gap: 2,
        }}>
          {grid.map((row, y) =>
            row.map((el, x) => {
              // Add gap row between main table and lanthanides/actinides
              if (y === 7 && !el) {
                return <div key={`${x}-${y}`} style={{ width: 36, height: 18 }} />;
              }
              if (!el) {
                return <div key={`${x}-${y}`} />;
              }
              const isSelected = el.number === selectedElement;
              const isHovered = hoveredElement?.number === el.number;
              return (
                <button
                  key={el.number}
                  onClick={() => onSelect(el.number)}
                  onMouseEnter={() => setHoveredElement(el)}
                  onMouseLeave={() => setHoveredElement(null)}
                  style={{
                    width: 36,
                    height: 36,
                    border: isSelected ? '2px solid #fff' : '1px solid rgba(255,255,255,0.2)',
                    borderRadius: 4,
                    background: getCategoryColor(el.category),
                    color: '#000',
                    fontFamily: 'monospace',
                    fontSize: 11,
                    fontWeight: 'bold',
                    cursor: 'pointer',
                    display: 'flex',
                    flexDirection: 'column',
                    alignItems: 'center',
                    justifyContent: 'center',
                    padding: 0,
                    transform: isHovered ? 'scale(1.1)' : 'none',
                    boxShadow: isSelected ? '0 0 8px rgba(255,255,255,0.5)' : 'none',
                    transition: 'transform 0.1s',
                    zIndex: isHovered ? 10 : 1,
                  }}
                  title={`${el.number}. ${el.name}`}
                >
                  <span style={{ fontSize: 8, opacity: 0.7 }}>{el.number}</span>
                  <span>{el.symbol}</span>
                </button>
              );
            })
          )}
        </div>

        {/* Category legend */}
        <div style={{
          marginTop: 16,
          display: 'flex',
          flexWrap: 'wrap',
          gap: 8,
          justifyContent: 'center',
        }}>
          {[
            ['Alkali Metal', '#ff6b6b'],
            ['Alkaline Earth', '#feca57'],
            ['Transition Metal', '#48dbfb'],
            ['Post-Transition', '#1dd1a1'],
            ['Metalloid', '#5f27cd'],
            ['Nonmetal', '#00d2d3'],
            ['Halogen', '#ff9f43'],
            ['Noble Gas', '#ff9ff3'],
            ['Lanthanide', '#54a0ff'],
            ['Actinide', '#c8d6e5'],
          ].map(([label, color]) => (
            <div key={label} style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
              <div style={{
                width: 12,
                height: 12,
                borderRadius: 2,
                background: color,
              }} />
              <span style={{ color: '#fff', fontSize: 10, fontFamily: 'system-ui' }}>
                {label}
              </span>
            </div>
          ))}
        </div>

        {/* Attribution */}
        <div style={{
          marginTop: 12,
          textAlign: 'center',
          fontSize: 10,
          color: 'rgba(255,255,255,0.4)',
          fontFamily: 'system-ui',
        }}>
          Data from{' '}
          <a
            href="https://github.com/Bowserinator/Periodic-Table-JSON"
            target="_blank"
            rel="noopener noreferrer"
            style={{ color: 'rgba(255,255,255,0.6)' }}
          >
            Periodic-Table-JSON
          </a>
          {' '}(CC-BY-A)
        </div>
      </div>
    </div>
  );
}
