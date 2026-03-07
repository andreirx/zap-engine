// Thread Town — A synchronization visualizer for teaching concurrency concepts.
//
// Visual metaphors:
// - Robots = Threads
// - Lock = Mutex
// - Counter = Shared memory
// - Thought bubbles = Local registers

import { useState, useCallback } from 'react';
import { useZapEngine, TimingBars } from '@zap/web/react';
import type { GameEvent } from '@zap/web/react';

const WASM_URL = '/examples/thread-town/pkg/thread_town.js';
const ASSETS_URL = '/examples/thread-town/public/assets/assets.json';

// Phase names for display
const DATA_RACE_PHASES = [
  'Initial',
  'Both robots walk to counter',
  'Both robots at counter',
  'Robot A reads counter',
  'Robot B reads counter',
  'Robot A increments register',
  'Robot B increments register',
  'Robot A writes to counter',
  'Robot B writes to counter',
  'Race detected!',
  'Both robots walk home',
  'Complete',
];

const MUTEX_PHASES = [
  'Initial',
  'Robot A walks to counter',
  'Robot A acquires lock',
  'Robot A reads counter',
  'Robot A increments register',
  'Robot A writes to counter',
  'Robot A releases lock',
  'Robot B walks to counter',
  'Robot B acquires lock',
  'Robot B reads counter',
  'Robot B increments register',
  'Robot B writes to counter',
  'Robot B releases lock',
  'Robot B walks home',
  'Complete - Success!',
];

// Custom event codes (match Rust)
const CUSTOM_PLAY = 1;
const CUSTOM_PAUSE = 2;
const CUSTOM_STEP = 3;
const CUSTOM_RESET = 4;
const CUSTOM_SCENARIO = 5;

// Game event codes (match Rust)
const EVENT_COUNTER_VALUE = 2;
const EVENT_RACE_DETECTED = 3;
const EVENT_PHASE_NAME = 4;
const EVENT_SUCCESS = 5;
const EVENT_SCENARIO = 6;

type Scenario = 'data-race' | 'mutex-fix';

export function App() {
  const [counterValue, setCounterValue] = useState(5);
  const [phaseIndex, setPhaseIndex] = useState(0);
  const [raceDetected, setRaceDetected] = useState(false);
  const [success, setSuccess] = useState(false);
  const [isPlaying, setIsPlaying] = useState(false);
  const [scenario, setScenario] = useState<Scenario>('data-race');
  const [timingCollapsed, setTimingCollapsed] = useState(true);

  const onGameEvent = useCallback((events: GameEvent[]) => {
    for (const e of events) {
      if (e.kind === EVENT_COUNTER_VALUE) {
        setCounterValue(e.a);
      } else if (e.kind === EVENT_RACE_DETECTED) {
        setRaceDetected(true);
      } else if (e.kind === EVENT_SUCCESS) {
        setSuccess(true);
      } else if (e.kind === EVENT_PHASE_NAME) {
        // Mutex phases are offset by 100
        if (e.a >= 100) {
          setPhaseIndex(e.a - 100);
        } else {
          setPhaseIndex(e.a);
        }
      } else if (e.kind === EVENT_SCENARIO) {
        setScenario(e.a === 0 ? 'data-race' : 'mutex-fix');
      }
    }
  }, []);

  const { canvasRef, sendEvent, fps, isReady, canvasKey, timing } = useZapEngine({
    wasmUrl: WASM_URL,
    assetsUrl: ASSETS_URL,
    gameWidth: 800,
    gameHeight: 600,
    onGameEvent,
  });

  const handlePlay = () => {
    setIsPlaying(true);
    sendEvent({ type: 'custom', kind: CUSTOM_PLAY });
  };

  const handlePause = () => {
    setIsPlaying(false);
    sendEvent({ type: 'custom', kind: CUSTOM_PAUSE });
  };

  const handleStep = () => {
    sendEvent({ type: 'custom', kind: CUSTOM_STEP });
  };

  const handleReset = () => {
    setIsPlaying(false);
    setRaceDetected(false);
    setSuccess(false);
    setPhaseIndex(0);
    sendEvent({ type: 'custom', kind: CUSTOM_RESET });
  };

  const handleScenarioChange = (newScenario: Scenario) => {
    setScenario(newScenario);
    setIsPlaying(false);
    setRaceDetected(false);
    setSuccess(false);
    setPhaseIndex(0);
    sendEvent({ type: 'custom', kind: CUSTOM_SCENARIO, a: newScenario === 'data-race' ? 0 : 1 });
  };

  const phases = scenario === 'data-race' ? DATA_RACE_PHASES : MUTEX_PHASES;
  const currentPhase = phases[phaseIndex] || 'Unknown';
  const isComplete = phaseIndex >= phases.length - 1;

  // Determine background color for phase display
  let phaseBackground = 'rgba(0, 0, 0, 0.7)';
  if (raceDetected) {
    phaseBackground = 'rgba(231, 76, 60, 0.9)';
  } else if (success) {
    phaseBackground = 'rgba(39, 174, 96, 0.9)';
  }

  return (
    <div style={{ position: 'relative', width: '100vw', height: '100vh', background: '#1a1a2e' }}>
      <canvas
        key={canvasKey}
        ref={canvasRef}
        style={{ width: '100%', height: '100%', display: 'block' }}
      />

      {/* Title and explanation */}
      <div style={{
        position: 'absolute',
        top: 12,
        left: 16,
        color: '#fff',
        fontFamily: 'system-ui, sans-serif',
      }}>
        <h1 style={{ fontSize: 24, margin: 0, marginBottom: 4 }}>Thread Town</h1>
        <p style={{ fontSize: 12, opacity: 0.7, margin: 0 }}>
          {scenario === 'data-race'
            ? 'Watch how unsynchronized threads cause a data race'
            : 'See how a mutex prevents the data race'}
        </p>
      </div>

      {/* FPS and timing */}
      <div style={{ position: 'absolute', top: 12, right: 16 }}>
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

      {/* Scenario selector */}
      <div style={{
        position: 'absolute',
        top: 70,
        left: '50%',
        transform: 'translateX(-50%)',
        display: 'flex',
        gap: 8,
      }}>
        <button
          onClick={() => handleScenarioChange('data-race')}
          style={{
            fontFamily: 'system-ui, sans-serif',
            fontSize: 13,
            padding: '6px 14px',
            borderRadius: 6,
            border: scenario === 'data-race' ? '2px solid #e74c3c' : '2px solid transparent',
            background: scenario === 'data-race' ? 'rgba(231, 76, 60, 0.2)' : 'rgba(255, 255, 255, 0.1)',
            color: '#fff',
            cursor: 'pointer',
          }}
        >
          1. Data Race (Bug)
        </button>
        <button
          onClick={() => handleScenarioChange('mutex-fix')}
          style={{
            fontFamily: 'system-ui, sans-serif',
            fontSize: 13,
            padding: '6px 14px',
            borderRadius: 6,
            border: scenario === 'mutex-fix' ? '2px solid #27ae60' : '2px solid transparent',
            background: scenario === 'mutex-fix' ? 'rgba(39, 174, 96, 0.2)' : 'rgba(255, 255, 255, 0.1)',
            color: '#fff',
            cursor: 'pointer',
          }}
        >
          2. Mutex Fix (Solution)
        </button>
      </div>

      {/* Control panel */}
      <div style={{
        position: 'absolute',
        bottom: 20,
        left: '50%',
        transform: 'translateX(-50%)',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        gap: 12,
      }}>
        {/* Phase display */}
        <div style={{
          background: phaseBackground,
          padding: '8px 20px',
          borderRadius: 8,
          color: '#fff',
          fontFamily: 'system-ui, sans-serif',
          fontSize: 16,
          textAlign: 'center',
          minWidth: 320,
          transition: 'background 0.3s',
        }}>
          {currentPhase}
        </div>

        {/* Counter display */}
        <div style={{
          background: 'rgba(0, 0, 0, 0.7)',
          padding: '6px 16px',
          borderRadius: 6,
          color: raceDetected ? '#e74c3c' : success ? '#27ae60' : '#3498db',
          fontFamily: 'monospace',
          fontSize: 18,
          transition: 'color 0.3s',
        }}>
          Counter: {counterValue}
          {raceDetected && ' (Expected: 7, Got: 6 - Lost update!)'}
          {success && ' (Correct! 5 + 1 + 1 = 7)'}
        </div>

        {/* Control buttons */}
        <div style={{ display: 'flex', gap: 10 }}>
          {!isPlaying ? (
            <button
              onClick={handlePlay}
              disabled={isComplete}
              style={{
                fontFamily: 'system-ui, sans-serif',
                fontSize: 14,
                padding: '8px 20px',
                borderRadius: 6,
                border: 'none',
                background: '#27ae60',
                color: '#fff',
                cursor: 'pointer',
                opacity: isComplete ? 0.5 : 1,
              }}
            >
              Play
            </button>
          ) : (
            <button
              onClick={handlePause}
              style={{
                fontFamily: 'system-ui, sans-serif',
                fontSize: 14,
                padding: '8px 20px',
                borderRadius: 6,
                border: 'none',
                background: '#f39c12',
                color: '#fff',
                cursor: 'pointer',
              }}
            >
              Pause
            </button>
          )}
          <button
            onClick={handleStep}
            disabled={isPlaying || isComplete}
            style={{
              fontFamily: 'system-ui, sans-serif',
              fontSize: 14,
              padding: '8px 20px',
              borderRadius: 6,
              border: 'none',
              background: '#3498db',
              color: '#fff',
              cursor: 'pointer',
              opacity: (isPlaying || isComplete) ? 0.5 : 1,
            }}
          >
            Step
          </button>
          <button
            onClick={handleReset}
            style={{
              fontFamily: 'system-ui, sans-serif',
              fontSize: 14,
              padding: '8px 20px',
              borderRadius: 6,
              border: 'none',
              background: '#e74c3c',
              color: '#fff',
              cursor: 'pointer',
            }}
          >
            Reset
          </button>
        </div>
      </div>

      {/* Legend */}
      <div style={{
        position: 'absolute',
        bottom: 20,
        left: 16,
        background: 'rgba(0, 0, 0, 0.7)',
        padding: 12,
        borderRadius: 8,
        color: '#fff',
        fontFamily: 'system-ui, sans-serif',
        fontSize: 12,
      }}>
        <div style={{ marginBottom: 6, fontWeight: 'bold' }}>Legend:</div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
          <span style={{ color: '#3498db' }}>Blue Robot</span> = Thread A
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
          <span style={{ color: '#e67e22' }}>Orange Robot</span> = Thread B
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
          <span>Number above robot</span> = Local register
        </div>
        {scenario === 'mutex-fix' && (
          <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
            <span style={{ color: '#95a5a6' }}>Lock</span> = std::mutex
          </div>
        )}
      </div>

      {/* C++ code reference */}
      <div style={{
        position: 'absolute',
        bottom: 20,
        right: 16,
        background: 'rgba(0, 0, 0, 0.8)',
        padding: 12,
        borderRadius: 8,
        color: '#9cdcfe',
        fontFamily: 'monospace',
        fontSize: 11,
        whiteSpace: 'pre',
        lineHeight: 1.4,
      }}>
        {scenario === 'data-race' ? `// BUG: Data race!
void thread_func() {
  int local = counter; // read
  local++;             // increment
  counter = local;     // write
}` : `// FIXED: With mutex
void thread_func() {
  lock_guard<mutex> g(mtx);
  int local = counter; // read
  local++;             // increment
  counter = local;     // write
} // unlock`}
      </div>
    </div>
  );
}
