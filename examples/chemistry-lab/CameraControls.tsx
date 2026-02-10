// CameraControls.tsx
// Dedicated camera control panel for Chemistry Lab
//
// Control types:
// - Zoom: in/out (changes camera distance)
// - Pan: move camera target in X/Y/Z
// - Rotate: azimuth (horizontal) and elevation (vertical)

import { CSSProperties, useCallback } from 'react';

/** Custom event IDs matching game.rs events module */
const CAMERA_EVENTS = {
  ZOOM: 10,
  PAN_X: 11,
  PAN_Y: 12,
  PAN_Z: 13,
  ROTATE_AZIMUTH: 14,
  ROTATE_ELEVATION: 15,
} as const;

/** Step sizes for different control types */
const STEP = {
  ZOOM: 0.5,           // Zoom factor per click
  PAN: 50,             // World units per click
  ROTATE: 0.2,         // Radians per click (~11 degrees)
} as const;

interface CameraControlsProps {
  /** Send custom event to WASM game */
  sendEvent: (event: { type: 'custom'; kind: number; a: number; b?: number; c?: number }) => void;
  /** Optional style overrides */
  style?: CSSProperties;
}

/** Reusable button component */
function ControlButton({
  onClick,
  label,
  title,
  width = 28,
}: {
  onClick: () => void;
  label: string;
  title: string;
  width?: number;
}) {
  return (
    <button
      onClick={onClick}
      title={title}
      style={{
        width,
        height: 28,
        border: '1px solid #444',
        borderRadius: 4,
        background: '#2a2a3a',
        color: '#fff',
        fontFamily: 'monospace',
        fontSize: 12,
        fontWeight: 'bold',
        cursor: 'pointer',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        transition: 'background 0.1s, transform 0.1s',
      }}
      onMouseDown={(e) => {
        e.currentTarget.style.background = '#3a3a4a';
        e.currentTarget.style.transform = 'scale(0.95)';
      }}
      onMouseUp={(e) => {
        e.currentTarget.style.background = '#2a2a3a';
        e.currentTarget.style.transform = 'scale(1)';
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.background = '#2a2a3a';
        e.currentTarget.style.transform = 'scale(1)';
      }}
    >
      {label}
    </button>
  );
}

/** Control group with label */
function ControlGroup({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
      <div style={{
        fontSize: 9,
        color: 'rgba(255,255,255,0.5)',
        textTransform: 'uppercase',
        letterSpacing: 1,
      }}>
        {label}
      </div>
      <div style={{ display: 'flex', gap: 4 }}>
        {children}
      </div>
    </div>
  );
}

export function CameraControls({ sendEvent, style }: CameraControlsProps) {
  // Zoom controls
  const zoomIn = useCallback(() => {
    sendEvent({ type: 'custom', kind: CAMERA_EVENTS.ZOOM, a: STEP.ZOOM });
  }, [sendEvent]);

  const zoomOut = useCallback(() => {
    sendEvent({ type: 'custom', kind: CAMERA_EVENTS.ZOOM, a: -STEP.ZOOM });
  }, [sendEvent]);

  // Pan controls
  const panLeft = useCallback(() => {
    sendEvent({ type: 'custom', kind: CAMERA_EVENTS.PAN_X, a: -STEP.PAN });
  }, [sendEvent]);

  const panRight = useCallback(() => {
    sendEvent({ type: 'custom', kind: CAMERA_EVENTS.PAN_X, a: STEP.PAN });
  }, [sendEvent]);

  const panUp = useCallback(() => {
    sendEvent({ type: 'custom', kind: CAMERA_EVENTS.PAN_Y, a: STEP.PAN });
  }, [sendEvent]);

  const panDown = useCallback(() => {
    sendEvent({ type: 'custom', kind: CAMERA_EVENTS.PAN_Y, a: -STEP.PAN });
  }, [sendEvent]);

  const panForward = useCallback(() => {
    sendEvent({ type: 'custom', kind: CAMERA_EVENTS.PAN_Z, a: STEP.PAN });
  }, [sendEvent]);

  const panBackward = useCallback(() => {
    sendEvent({ type: 'custom', kind: CAMERA_EVENTS.PAN_Z, a: -STEP.PAN });
  }, [sendEvent]);

  // Rotate controls
  const rotateLeft = useCallback(() => {
    sendEvent({ type: 'custom', kind: CAMERA_EVENTS.ROTATE_AZIMUTH, a: -STEP.ROTATE });
  }, [sendEvent]);

  const rotateRight = useCallback(() => {
    sendEvent({ type: 'custom', kind: CAMERA_EVENTS.ROTATE_AZIMUTH, a: STEP.ROTATE });
  }, [sendEvent]);

  const rotateUp = useCallback(() => {
    sendEvent({ type: 'custom', kind: CAMERA_EVENTS.ROTATE_ELEVATION, a: STEP.ROTATE });
  }, [sendEvent]);

  const rotateDown = useCallback(() => {
    sendEvent({ type: 'custom', kind: CAMERA_EVENTS.ROTATE_ELEVATION, a: -STEP.ROTATE });
  }, [sendEvent]);

  return (
    <div
      style={{
        background: 'rgba(0,0,0,0.85)',
        borderRadius: 8,
        padding: 12,
        display: 'flex',
        flexDirection: 'column',
        gap: 12,
        fontFamily: 'system-ui',
        ...style,
      }}
    >
      {/* Header */}
      <div style={{
        fontSize: 11,
        fontWeight: 'bold',
        color: '#fff',
        borderBottom: '1px solid #333',
        paddingBottom: 6,
      }}>
        Camera Controls
      </div>

      {/* Zoom */}
      <ControlGroup label="Zoom">
        <ControlButton onClick={zoomOut} label="-" title="Zoom out" />
        <ControlButton onClick={zoomIn} label="+" title="Zoom in" />
      </ControlGroup>

      {/* Pan XY */}
      <ControlGroup label="Pan">
        <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
          <div style={{ display: 'flex', gap: 2, justifyContent: 'center' }}>
            <ControlButton onClick={panUp} label="Y+" title="Pan up" />
          </div>
          <div style={{ display: 'flex', gap: 2 }}>
            <ControlButton onClick={panLeft} label="X-" title="Pan left" />
            <ControlButton onClick={panDown} label="Y-" title="Pan down" />
            <ControlButton onClick={panRight} label="X+" title="Pan right" />
          </div>
        </div>
      </ControlGroup>

      {/* Pan Z (depth) */}
      <ControlGroup label="Depth">
        <ControlButton onClick={panBackward} label="Z-" title="Pan backward" width={40} />
        <ControlButton onClick={panForward} label="Z+" title="Pan forward" width={40} />
      </ControlGroup>

      {/* Rotate */}
      <ControlGroup label="Rotate">
        <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
          <div style={{ display: 'flex', gap: 2, justifyContent: 'center' }}>
            <ControlButton onClick={rotateUp} label="^" title="Rotate up (elevation)" />
          </div>
          <div style={{ display: 'flex', gap: 2 }}>
            <ControlButton onClick={rotateLeft} label="<" title="Rotate left (azimuth)" />
            <ControlButton onClick={rotateDown} label="v" title="Rotate down (elevation)" />
            <ControlButton onClick={rotateRight} label=">" title="Rotate right (azimuth)" />
          </div>
        </div>
      </ControlGroup>
    </div>
  );
}
