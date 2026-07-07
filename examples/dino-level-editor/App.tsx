import { useCallback, useEffect, useRef, useState, type CSSProperties } from 'react';
import { TimingBars, useZapEngine } from '@zap/web/react';
import type { GameEvent } from '@zap/web/react';

const WASM_URL = '/examples/dino-level-editor/pkg/dino_level_editor.js';
const ASSETS_URL = '/examples/dino-level-editor/public/assets/assets.json';

const CUSTOM_SET_TOOL = 1;
const CUSTOM_SET_ACTION = 2;
const CUSTOM_SET_RECTANGLE = 3;
const CUSTOM_SET_DINO_COLOR = 4;
const CUSTOM_PLAY = 5;
const CUSTOM_EDIT = 6;
const CUSTOM_RESET_LEVEL = 7;

const EVENT_MODE = 1;
const EVENT_SCORE = 2;
const EVENT_LIVES = 3;

type Tool = 'ground' | 'lava' | 'water' | 'money1' | 'money10' | 'powerup' | 'dino' | 'finish' | 'volcano';
type Action = 'place' | 'erase';
type DinoColor = 'verde' | 'albastru' | 'galben' | 'mov' | 'rosu';
type SlotDialog = 'save' | 'load' | null;

const LEVEL_DB_NAME = 'zap-engine-dino-level-editor';
const LEVEL_DB_VERSION = 1;
const LEVEL_STORE_NAME = 'levels';
const DEFAULT_LEVEL_ID = 'default';
const LEVEL_LOCAL_STORAGE_KEY = 'zap-engine-dino-level-editor:default-level';
const LEVEL_LOCAL_STORAGE_PREFIX = 'zap-engine-dino-level-editor:level:';
const LEVEL_LOCAL_STORAGE_INDEX_KEY = 'zap-engine-dino-level-editor:named-slots';
const AUTO_SAVE_MS = 2500;

const namedSlotId = (name: string) => `named:${name.trim()}`;

const isNamedSlot = (record: StoredLevelRecord) => record.id.startsWith('named:');

let memoryLevelRecords = new Map<string, StoredLevelRecord>();

interface StoredLevelRecord {
  id: string;
  name?: string;
  json: string;
  savedAt: string;
  schemaVersion: number;
}

interface SavedLevelDocument {
  dino_color?: number;
}

const toolCodes: Record<Tool, number> = {
  ground: 0,
  money1: 1,
  money10: 2,
  dino: 3,
  finish: 4,
  volcano: 5,
  lava: 6,
  water: 7,
  powerup: 8,
};

const colorCodes: Record<DinoColor, number> = {
  verde: 0,
  albastru: 1,
  galben: 2,
  mov: 3,
  rosu: 4,
};

const toolLabels: { tool: Tool; label: string }[] = [
  { tool: 'ground', label: 'Ground' },
  { tool: 'lava', label: 'Lava' },
  { tool: 'water', label: 'Water' },
  { tool: 'money1', label: 'Ban ×1' },
  { tool: 'money10', label: 'Ban ×10' },
  { tool: 'powerup', label: 'Carne' },
  { tool: 'dino', label: 'Dino Start' },
  { tool: 'finish', label: 'Finish' },
  { tool: 'volcano', label: 'Volcano' },
];

const colorLabels: { color: DinoColor; label: string }[] = [
  { color: 'verde', label: 'Green' },
  { color: 'albastru', label: 'Blue' },
  { color: 'galben', label: 'Yellow' },
  { color: 'mov', label: 'Purple' },
  { color: 'rosu', label: 'Red' },
];

const colorFromSaveCode = (code: number | undefined): DinoColor | undefined => {
  switch (code) {
    case 0: return 'verde';
    case 1: return 'albastru';
    case 2: return 'galben';
    case 3: return 'mov';
    case 4: return 'rosu';
    default: return undefined;
  }
};

function openLevelDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(LEVEL_DB_NAME, LEVEL_DB_VERSION);
    request.onupgradeneeded = () => {
      const db = request.result;
      if (!db.objectStoreNames.contains(LEVEL_STORE_NAME)) {
        db.createObjectStore(LEVEL_STORE_NAME, { keyPath: 'id' });
      }
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error ?? new Error('Failed to open level database'));
  });
}

function levelRecord(id: string, json: string, name?: string): StoredLevelRecord {
  return {
    id,
    ...(name ? { name } : {}),
    json,
    savedAt: new Date().toISOString(),
    schemaVersion: 1,
  };
}

async function saveLevelToBrowserStorage(id: string, json: string, name?: string): Promise<string> {
  const record = levelRecord(id, json, name);
  if (typeof indexedDB === 'undefined') {
    if (typeof localStorage === 'undefined') {
      memoryLevelRecords.set(id, record);
      return 'memory';
    }
    localStorage.setItem(`${LEVEL_LOCAL_STORAGE_PREFIX}${id}`, JSON.stringify(record));
    if (id === DEFAULT_LEVEL_ID) {
      localStorage.setItem(LEVEL_LOCAL_STORAGE_KEY, JSON.stringify(record));
    } else {
      const existing = JSON.parse(localStorage.getItem(LEVEL_LOCAL_STORAGE_INDEX_KEY) ?? '[]') as string[];
      if (!existing.includes(id)) {
        existing.push(id);
        localStorage.setItem(LEVEL_LOCAL_STORAGE_INDEX_KEY, JSON.stringify(existing));
      }
    }
    return 'localStorage';
  }

  const db = await openLevelDb();
  try {
    await new Promise<void>((resolve, reject) => {
      const tx = db.transaction(LEVEL_STORE_NAME, 'readwrite');
      tx.objectStore(LEVEL_STORE_NAME).put(record);
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error ?? new Error('Failed to save level'));
    });
    return 'IndexedDB';
  } finally {
    db.close();
  }
}

async function loadLevelFromBrowserStorage(id = DEFAULT_LEVEL_ID): Promise<StoredLevelRecord | null> {
  if (typeof indexedDB === 'undefined') {
    if (typeof localStorage === 'undefined') {
      return memoryLevelRecords.get(id) ?? null;
    }
    const raw = localStorage.getItem(`${LEVEL_LOCAL_STORAGE_PREFIX}${id}`)
      ?? (id === DEFAULT_LEVEL_ID ? localStorage.getItem(LEVEL_LOCAL_STORAGE_KEY) : null);
    return raw ? (JSON.parse(raw) as StoredLevelRecord) : null;
  }

  const db = await openLevelDb();
  try {
    return await new Promise<StoredLevelRecord | null>((resolve, reject) => {
      const tx = db.transaction(LEVEL_STORE_NAME, 'readonly');
      const request = tx.objectStore(LEVEL_STORE_NAME).get(id);
      request.onsuccess = () => resolve((request.result as StoredLevelRecord | undefined) ?? null);
      request.onerror = () => reject(request.error ?? new Error('Failed to load level'));
    });
  } finally {
    db.close();
  }
}

async function listNamedLevelSlots(): Promise<StoredLevelRecord[]> {
  if (typeof indexedDB === 'undefined') {
    if (typeof localStorage === 'undefined') {
      return [...memoryLevelRecords.values()].filter(isNamedSlot).sort(compareSavedLevels);
    }
    const ids = JSON.parse(localStorage.getItem(LEVEL_LOCAL_STORAGE_INDEX_KEY) ?? '[]') as string[];
    const records = ids
      .map((id) => localStorage.getItem(`${LEVEL_LOCAL_STORAGE_PREFIX}${id}`))
      .filter((raw): raw is string => raw !== null)
      .map((raw) => JSON.parse(raw) as StoredLevelRecord)
      .filter(isNamedSlot);
    return records.sort(compareSavedLevels);
  }

  const db = await openLevelDb();
  try {
    return await new Promise<StoredLevelRecord[]>((resolve, reject) => {
      const tx = db.transaction(LEVEL_STORE_NAME, 'readonly');
      const request = tx.objectStore(LEVEL_STORE_NAME).getAll();
      request.onsuccess = () => {
        const records = (request.result as StoredLevelRecord[]).filter(isNamedSlot).sort(compareSavedLevels);
        resolve(records);
      };
      request.onerror = () => reject(request.error ?? new Error('Failed to list saved levels'));
    });
  } finally {
    db.close();
  }
}

function compareSavedLevels(a: StoredLevelRecord, b: StoredLevelRecord): number {
  return b.savedAt.localeCompare(a.savedAt);
}

export function App() {
  const [mode, setMode] = useState<'edit' | 'play'>('edit');
  const [tool, setTool] = useState<Tool>('ground');
  const [action, setAction] = useState<Action>('place');
  const [rectangle, setRectangle] = useState(false);
  const [dinoColor, setDinoColor] = useState<DinoColor>('verde');
  const [score, setScore] = useState(0);
  const [lives, setLives] = useState(1);
  const [timingCollapsed, setTimingCollapsed] = useState(true);
  const [saveStatus, setSaveStatus] = useState('Auto-loading browser storage when the engine is ready.');
  const [confirmResetOpen, setConfirmResetOpen] = useState(false);
  const [slotDialog, setSlotDialog] = useState<SlotDialog>(null);
  const [slotName, setSlotName] = useState('');
  const [namedSlots, setNamedSlots] = useState<StoredLevelRecord[]>([]);
  const pendingExportRef = useRef<{ id: string; name?: string; manual: boolean } | null>(null);
  const autoLoadTriedRef = useRef(false);

  const syncUiFromLevelJson = useCallback((json: string) => {
    try {
      const document = JSON.parse(json) as SavedLevelDocument;
      const loadedColor = colorFromSaveCode(document.dino_color);
      if (loadedColor) {
        setDinoColor(loadedColor);
      }
    } catch {
      // Loading still belongs to Rust; this parse only keeps the React color button in sync.
    }
  }, []);

  const onGameEvent = useCallback((events: GameEvent[]) => {
    for (const event of events) {
      if (event.kind === EVENT_MODE) {
        setMode(event.a >= 0.5 ? 'play' : 'edit');
      } else if (event.kind === EVENT_SCORE) {
        setScore(Math.round(event.a));
      } else if (event.kind === EVENT_LIVES) {
        setLives(Math.round(event.a));
      }
    }
  }, []);

  const onWorkerMessage = useCallback((data: Record<string, unknown>) => {
    if (data.type !== 'world_export') {
      return;
    }
    const json = typeof data.json === 'string' ? data.json : null;
    const pendingExport = pendingExportRef.current;
    if (!pendingExport) {
      return;
    }
    pendingExportRef.current = null;
    if (!json) {
      setSaveStatus('Save failed: the game did not return level JSON.');
      return;
    }
    void saveLevelToBrowserStorage(pendingExport.id, json, pendingExport.name)
      .then((backend) => {
        const time = new Date().toLocaleTimeString();
        if (pendingExport.manual) {
          setSaveStatus(`Saved "${pendingExport.name}" to ${backend} at ${time}.`);
        } else {
          setSaveStatus(`Auto-saved default slot to ${backend} at ${time}.`);
        }
      })
      .catch((err: unknown) => {
        setSaveStatus(`Save failed: ${err instanceof Error ? err.message : String(err)}`);
      });
  }, []);

  const { canvasRef, sendEvent, fps, isReady, canvasKey, timing } = useZapEngine({
    wasmUrl: WASM_URL,
    assetsUrl: ASSETS_URL,
    onGameEvent,
    onWorkerMessage,
  });

  useEffect(() => {
    if (!isReady || autoLoadTriedRef.current) {
      return;
    }
    autoLoadTriedRef.current = true;
    void loadLevelFromBrowserStorage(DEFAULT_LEVEL_ID)
      .then((record) => {
        if (!record) {
          setSaveStatus('No default auto-save found; using the starter level.');
          return;
        }
        sendEvent({ type: 'load_level', json: record.json });
        syncUiFromLevelJson(record.json);
        setMode('edit');
        setScore(0);
        setLives(1);
        setSaveStatus(`Auto-loaded default slot saved at ${new Date(record.savedAt).toLocaleString()}.`);
      })
      .catch((err: unknown) => {
        setSaveStatus(`Auto-load failed: ${err instanceof Error ? err.message : String(err)}`);
      });
  }, [isReady, sendEvent, syncUiFromLevelJson]);

  useEffect(() => {
    if (!isReady) {
      return;
    }
    const timer = window.setInterval(() => {
      if (mode !== 'edit' || pendingExportRef.current) {
        return;
      }
      pendingExportRef.current = { id: DEFAULT_LEVEL_ID, name: 'default', manual: false };
      sendEvent({ type: 'export_world' });
    }, AUTO_SAVE_MS);
    return () => window.clearInterval(timer);
  }, [isReady, mode, sendEvent]);

  useEffect(() => {
    const preventPageHotkeys = (event: KeyboardEvent) => {
      if (['Space', 'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(event.code)) {
        event.preventDefault();
      }
    };
    window.addEventListener('keydown', preventPageHotkeys, { passive: false });
    return () => window.removeEventListener('keydown', preventPageHotkeys);
  }, []);

  const selectTool = (nextTool: Tool) => {
    setTool(nextTool);
    sendEvent({ type: 'custom', kind: CUSTOM_SET_TOOL, a: toolCodes[nextTool] });
  };

  const selectAction = (nextAction: Action) => {
    setAction(nextAction);
    sendEvent({ type: 'custom', kind: CUSTOM_SET_ACTION, a: nextAction === 'erase' ? 1 : 0 });
  };

  const toggleRectangle = () => {
    const next = !rectangle;
    setRectangle(next);
    sendEvent({ type: 'custom', kind: CUSTOM_SET_RECTANGLE, a: next ? 1 : 0 });
  };

  const selectColor = (nextColor: DinoColor) => {
    setDinoColor(nextColor);
    sendEvent({ type: 'custom', kind: CUSTOM_SET_DINO_COLOR, a: colorCodes[nextColor] });
  };

  const play = () => {
    setScore(0);
    setLives(1);
    setMode('play');
    sendEvent({ type: 'custom', kind: CUSTOM_PLAY });
  };

  const edit = () => {
    setMode('edit');
    sendEvent({ type: 'custom', kind: CUSTOM_EDIT });
  };

  const refreshNamedSlots = useCallback(() => {
    void listNamedLevelSlots()
      .then(setNamedSlots)
      .catch((err: unknown) => {
        setSaveStatus(`Could not list saved levels: ${err instanceof Error ? err.message : String(err)}`);
      });
  }, []);

  const openSlotDialog = (dialog: Exclude<SlotDialog, null>) => {
    if (!isReady || mode !== 'edit') {
      return;
    }
    setSlotDialog(dialog);
    if (dialog === 'load') {
      refreshNamedSlots();
    }
  };

  const saveNamedSlot = () => {
    const trimmedName = slotName.trim();
    if (!isReady || mode !== 'edit' || !trimmedName) {
      return;
    }
    pendingExportRef.current = { id: namedSlotId(trimmedName), name: trimmedName, manual: true };
    setSaveStatus(`Saving "${trimmedName}"...`);
    setSlotDialog(null);
    sendEvent({ type: 'export_world' });
  };

  const loadNamedSlot = (record: StoredLevelRecord) => {
    if (!isReady || mode !== 'edit') {
      return;
    }
    setSaveStatus(`Loading "${record.name ?? record.id}"...`);
    void loadLevelFromBrowserStorage(record.id)
      .then((loadedRecord) => {
        if (!loadedRecord) {
          setSaveStatus(`Saved level "${record.name ?? record.id}" was not found.`);
          return;
        }
        setSlotDialog(null);
        sendEvent({ type: 'load_level', json: loadedRecord.json });
        syncUiFromLevelJson(loadedRecord.json);
        setMode('edit');
        setScore(0);
        setLives(1);
        setSaveStatus(`Loaded "${loadedRecord.name ?? loadedRecord.id}" saved at ${new Date(loadedRecord.savedAt).toLocaleString()}.`);
      })
      .catch((err: unknown) => {
        setSaveStatus(`Load failed: ${err instanceof Error ? err.message : String(err)}`);
      });
  };

  const confirmReset = () => {
    setConfirmResetOpen(false);
    sendEvent({ type: 'custom', kind: CUSTOM_RESET_LEVEL });
    setTool('ground');
    setAction('place');
    setRectangle(false);
    setDinoColor('verde');
    setScore(0);
    setLives(1);
    setSaveStatus('Reset to the built-in starter level. The default slot will auto-save while editing.');
  };

  return (
    <div style={styles.shell}>
      <div style={styles.topBar}>
        <div style={styles.brand}>Dino Level Editor</div>
        <div style={styles.modePill}>{mode === 'play' ? 'PLAYING' : 'EDITING'}</div>
        <button onClick={mode === 'play' ? edit : play} style={primaryButton(mode === 'edit')}>
          {mode === 'play' ? 'Back to Edit' : 'PLAY'}
        </button>
        <div style={styles.lives}>Lives: {lives}</div>
        <div style={styles.score}>Money: {score}</div>
      </div>

      <aside style={styles.sidebar}>
        {mode === 'edit' ? (
          <div style={styles.toolbar}>
            <div style={styles.group}>
              <div style={styles.groupLabel}>Tool</div>
              {toolLabels.map((entry) => (
                <button key={entry.tool} onClick={() => selectTool(entry.tool)} style={toolButton(tool === entry.tool)}>
                  {entry.label}
                </button>
              ))}
            </div>

            <div style={styles.group}>
              <div style={styles.groupLabel}>Operation</div>
              <button onClick={() => selectAction('place')} style={toolButton(action === 'place')}>Place</button>
              <button onClick={() => selectAction('erase')} style={toolButton(action === 'erase')}>Erase</button>
              <button onClick={toggleRectangle} style={toolButton(rectangle)}>Rectangle</button>
            </div>

            <div style={styles.group}>
              <div style={styles.groupLabel}>Dino Color</div>
              {colorLabels.map((entry) => (
                <button key={entry.color} onClick={() => selectColor(entry.color)} style={toolButton(dinoColor === entry.color)}>
                  {entry.label}
                </button>
              ))}
            </div>

            <div style={styles.group}>
              <div style={styles.groupLabel}>Level</div>
              <button disabled={!isReady} onClick={() => openSlotDialog('save')} style={toolButton(false)}>Save Named</button>
              <button disabled={!isReady} onClick={() => openSlotDialog('load')} style={toolButton(false)}>Load Named</button>
              <button disabled={!isReady} onClick={() => setConfirmResetOpen(true)} style={dangerButton}>
                Reset
              </button>
              <div style={styles.saveStatus}>{saveStatus}</div>
            </div>
          </div>
        ) : (
          <div style={styles.group}>
            <div style={styles.groupLabel}>Play Controls</div>
            <div style={styles.playHint}>D / →: move forward</div>
            <div style={styles.playHint}>Space / W / ↑: jump</div>
            <div style={styles.playHint}>Finish or fall returns here.</div>
          </div>
        )}

        <div style={styles.statusPanel}>
          <div>{isReady ? `${fps} FPS` : 'Loading...'}</div>
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
      </aside>

      <main style={styles.canvasPane}>
        <canvas key={canvasKey} ref={canvasRef} style={styles.canvas} />
      </main>

      <div style={styles.help}>
        {mode === 'edit'
          ? 'Piecewise edit by default. Rectangle is explicit. Dino start is continuous but constrained to ground.'
          : 'Move forward with D/→. Jump with Space, W, or ↑. Finish or fall returns to editing.'}
      </div>

      {confirmResetOpen && (
        <div style={styles.modalBackdrop}>
          <div style={styles.modal}>
            <div style={styles.modalTitle}>Reset level?</div>
            <div style={styles.modalBody}>
              This will replace the current editor state with the starter level. Browser storage is not overwritten until you press Save.
            </div>
            <div style={styles.modalActions}>
              <button onClick={confirmReset} style={dangerButton}>Yes, reset</button>
              <button onClick={() => setConfirmResetOpen(false)} style={toolButton(false)}>Cancel</button>
            </div>
          </div>
        </div>
      )}

      {slotDialog === 'save' && (
        <div style={styles.modalBackdrop}>
          <div style={styles.modal}>
            <div style={styles.modalTitle}>Save named level</div>
            <div style={styles.modalBody}>
              The default slot auto-saves. Use this for named snapshots you can load later.
            </div>
            <input
              value={slotName}
              onChange={(event) => setSlotName(event.target.value)}
              placeholder="Level name"
              style={styles.textInput}
              autoFocus
            />
            <div style={styles.modalActions}>
              <button disabled={!slotName.trim()} onClick={saveNamedSlot} style={primaryButton(Boolean(slotName.trim()))}>
                Save
              </button>
              <button onClick={() => setSlotDialog(null)} style={toolButton(false)}>Cancel</button>
            </div>
          </div>
        </div>
      )}

      {slotDialog === 'load' && (
        <div style={styles.modalBackdrop}>
          <div style={styles.modal}>
            <div style={styles.modalTitle}>Load named level</div>
            <div style={styles.modalBody}>
              Choose a named snapshot. The default auto-save is loaded automatically when the editor starts.
            </div>
            <div style={styles.slotList}>
              {namedSlots.length === 0 ? (
                <div style={styles.emptySlots}>No named saves yet.</div>
              ) : namedSlots.map((record) => (
                <button
                  key={record.id}
                  onClick={() => loadNamedSlot(record)}
                  style={styles.slotButton}
                >
                  <span style={styles.slotName}>{record.name ?? record.id.replace(/^named:/, '')}</span>
                  <span style={styles.slotTime}>{new Date(record.savedAt).toLocaleString()}</span>
                </button>
              ))}
            </div>
            <div style={styles.modalActions}>
              <button onClick={refreshNamedSlots} style={toolButton(false)}>Refresh</button>
              <button onClick={() => setSlotDialog(null)} style={toolButton(false)}>Cancel</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function toolButton(active: boolean): CSSProperties {
  return {
    ...styles.button,
    borderColor: active ? '#8df6a0' : 'rgba(255,255,255,0.16)',
    color: active ? '#d8ffe0' : 'rgba(255,255,255,0.78)',
    background: active ? 'rgba(40,130,70,0.72)' : 'rgba(10,18,28,0.78)',
  };
}

function primaryButton(active: boolean): CSSProperties {
  return {
    ...styles.button,
    fontWeight: 800,
    letterSpacing: '0.08em',
    borderColor: active ? '#f7d15e' : '#65b8ff',
    color: '#0a0a0a',
    background: active ? '#f7d15e' : '#65b8ff',
  };
}

const dangerButton: CSSProperties = {
  border: '1px solid rgba(255,120,120,0.72)',
  borderRadius: 8,
  padding: '7px 9px',
  fontSize: 13,
  cursor: 'pointer',
  color: '#ffe3e3',
  background: 'rgba(150,34,34,0.72)',
};

const styles: Record<string, CSSProperties> = {
  shell: {
    width: '100vw',
    height: '100vh',
    overflow: 'hidden',
    background: '#07111d',
    color: '#eef6ff',
    display: 'grid',
    gridTemplateColumns: '220px 1fr',
    gridTemplateRows: '56px 1fr 44px',
    fontFamily: 'Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
  },
  canvasPane: {
    gridColumn: 2,
    gridRow: 2,
    minWidth: 0,
    minHeight: 0,
    margin: '0 12px 0 0',
    borderRadius: 12,
    border: '1px solid rgba(255,255,255,0.12)',
    background: '#02060b',
    overflow: 'hidden',
  },
  canvas: {
    width: '100%',
    height: '100%',
    display: 'block',
    touchAction: 'none',
    cursor: 'crosshair',
  },
  topBar: {
    gridColumn: '1 / 3',
    gridRow: 1,
    display: 'flex',
    alignItems: 'center',
    gap: 10,
    padding: '10px 12px',
  },
  sidebar: {
    gridColumn: 1,
    gridRow: 2,
    minHeight: 0,
    display: 'flex',
    flexDirection: 'column',
    gap: 10,
    padding: '0 12px 0 12px',
    overflowY: 'auto',
  },
  brand: {
    padding: '8px 12px',
    borderRadius: 10,
    background: 'rgba(3,8,14,0.78)',
    border: '1px solid rgba(255,255,255,0.12)',
    fontWeight: 800,
  },
  modePill: {
    padding: '6px 10px',
    borderRadius: 999,
    background: 'rgba(255,255,255,0.10)',
    color: '#b9d7ff',
    fontSize: 12,
    fontWeight: 800,
    letterSpacing: '0.08em',
  },
  toolbar: {
    display: 'flex',
    flexDirection: 'column',
    gap: 10,
  },
  group: {
    display: 'flex',
    flexDirection: 'column',
    gap: 6,
    padding: 10,
    borderRadius: 12,
    background: 'rgba(3,8,14,0.76)',
    border: '1px solid rgba(255,255,255,0.10)',
    boxShadow: '0 10px 28px rgba(0,0,0,0.26)',
  },
  groupLabel: {
    color: 'rgba(255,255,255,0.48)',
    fontSize: 11,
    textTransform: 'uppercase',
    letterSpacing: '0.09em',
    marginBottom: 2,
  },
  button: {
    border: '1px solid rgba(255,255,255,0.16)',
    borderRadius: 8,
    padding: '7px 9px',
    fontSize: 13,
    cursor: 'pointer',
  },
  playHint: {
    color: 'rgba(255,255,255,0.72)',
    fontSize: 13,
    lineHeight: 1.45,
  },
  saveStatus: {
    color: 'rgba(255,255,255,0.62)',
    fontSize: 12,
    lineHeight: 1.35,
    marginTop: 2,
  },
  score: {
    marginLeft: 'auto',
    padding: '8px 12px',
    borderRadius: 10,
    background: 'rgba(3,8,14,0.78)',
    border: '1px solid rgba(255,255,255,0.12)',
    color: '#ffe28a',
    fontWeight: 800,
  },
  lives: {
    padding: '8px 12px',
    borderRadius: 10,
    background: 'rgba(3,8,14,0.78)',
    border: '1px solid rgba(255,255,255,0.12)',
    color: '#ffb1b1',
    fontWeight: 800,
  },
  statusPanel: {
    padding: 10,
    borderRadius: 12,
    background: 'rgba(3,8,14,0.76)',
    border: '1px solid rgba(255,255,255,0.10)',
    color: 'rgba(255,255,255,0.7)',
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, monospace',
    fontSize: 12,
    marginTop: 'auto',
  },
  help: {
    gridColumn: '1 / 3',
    gridRow: 3,
    alignSelf: 'center',
    justifySelf: 'center',
    maxWidth: 'min(820px, calc(100vw - 24px))',
    padding: '8px 12px',
    borderRadius: 999,
    background: 'rgba(3,8,14,0.72)',
    border: '1px solid rgba(255,255,255,0.10)',
    color: 'rgba(255,255,255,0.62)',
    fontSize: 13,
    textAlign: 'center',
    pointerEvents: 'none',
  },
  modalBackdrop: {
    gridColumn: '1 / 3',
    gridRow: '1 / 4',
    zIndex: 20,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    background: 'rgba(0,0,0,0.56)',
  },
  modal: {
    width: 360,
    maxWidth: 'calc(100vw - 40px)',
    padding: 18,
    borderRadius: 14,
    background: '#07111d',
    border: '1px solid rgba(255,255,255,0.18)',
    boxShadow: '0 24px 80px rgba(0,0,0,0.48)',
  },
  modalTitle: {
    fontSize: 18,
    fontWeight: 800,
    marginBottom: 8,
  },
  modalBody: {
    color: 'rgba(255,255,255,0.72)',
    fontSize: 14,
    lineHeight: 1.45,
    marginBottom: 16,
  },
  modalActions: {
    display: 'flex',
    justifyContent: 'flex-end',
    gap: 8,
  },
  textInput: {
    width: '100%',
    boxSizing: 'border-box',
    border: '1px solid rgba(255,255,255,0.20)',
    borderRadius: 8,
    padding: '9px 10px',
    marginBottom: 14,
    color: '#eef6ff',
    background: 'rgba(0,0,0,0.26)',
    outline: 'none',
  },
  slotList: {
    display: 'flex',
    flexDirection: 'column',
    gap: 8,
    maxHeight: 260,
    overflowY: 'auto',
    marginBottom: 14,
  },
  slotButton: {
    display: 'flex',
    flexDirection: 'column',
    gap: 3,
    alignItems: 'flex-start',
    border: '1px solid rgba(255,255,255,0.14)',
    borderRadius: 8,
    padding: 10,
    color: '#eef6ff',
    background: 'rgba(10,18,28,0.78)',
    cursor: 'pointer',
  },
  slotName: {
    fontWeight: 800,
  },
  slotTime: {
    color: 'rgba(255,255,255,0.58)',
    fontSize: 12,
  },
  emptySlots: {
    color: 'rgba(255,255,255,0.62)',
    fontSize: 13,
    padding: 8,
  },
};
