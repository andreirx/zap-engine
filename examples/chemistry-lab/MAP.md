# examples/chemistry-lab/

Interactive molecule builder demonstrating SDF rendering, physics joints, and React UI integration.

## Gameplay

- **Select element**: Click H, O, C, or N buttons to choose the active element
- **Place atoms**: Click empty space to spawn a new atom (SDF sphere)
- **Create bonds**: Click an existing atom, then drag to another atom to create a bond (SDF capsule + spring joint)
- **Clear**: Reset the workspace

## Engine Features Demonstrated

- **SDF Rendering**: Atoms as raymarched spheres, bonds as raymarched capsules (molecule.wgsl)
- **Physics Joints** (A3): Spring joints between bonded atoms maintain rest length
- **Custom Events** (A4): Element selection + clear from React UI buttons
- **World Coordinates** (A2): Click positions arrive in world coords for accurate atom placement
- **No Sprite Atlas**: Pure SDF rendering — `atlases: []` in manifest

## Architecture

```
src/lib.rs         — WASM exports (thread_local! GameRunner pattern)
src/game.rs        — ChemistryLab: Game trait impl, atom spawning, bond creation
src/elements.rs    — ElementData (symbol, radius, color, max_bonds) for H/O/C/N
src/molecule.rs    — MoleculeState tracking atoms and bonds
App.tsx            — React UI with element buttons, stats, clear button
main.tsx           — React entry point
```

## Element Properties

| Symbol | Radius | Max Bonds | Color |
|--------|--------|-----------|-------|
| H      | 12     | 1         | White |
| O      | 16     | 2         | Red   |
| C      | 18     | 4         | Gray  |
| N      | 15     | 3         | Blue  |

## How to Run

1. Build WASM: `wasm-pack build examples/chemistry-lab --target web --out-dir pkg`
2. From project root: `npm run dev` → navigate to the chemistry-lab entry
