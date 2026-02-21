//! Chemistry Lab game - thin controller layer.
//!
//! Routes input to systems and coordinates simulation/rendering.

use glam::Vec2;
use zap_engine::api::game::GameConfig;
use zap_engine::input::queue::{InputEvent, InputQueue};
use zap_engine::{EngineContext, Game, GameEvent};

use crate::interaction::{InteractionResult, InteractionSystem};
use crate::math3d::{Camera3D, Vec3};
use crate::periodic_table::ElementRegistry;
use crate::renderer::{BondPreview, FrameEffects, MoleculeRenderer};
use crate::sim::ChemistrySim;

const WORLD_W: f32 = 800.0;
const WORLD_H: f32 = 600.0;

/// Custom event kinds from React UI.
mod events {
    pub const SELECT_ELEMENT: u32 = 1;
    pub const CLEAR: u32 = 2;
    pub const RESET_CAMERA: u32 = 3;
    // Camera controls: a = delta value
    pub const CAMERA_ZOOM: u32 = 10;       // a > 0: zoom in, a < 0: zoom out
    pub const CAMERA_PAN_X: u32 = 11;      // a: pan amount
    pub const CAMERA_PAN_Y: u32 = 12;
    pub const CAMERA_PAN_Z: u32 = 13;
    pub const CAMERA_ROTATE_AZIMUTH: u32 = 14;   // a: rotation in radians
    pub const CAMERA_ROTATE_ELEVATION: u32 = 15;
}

/// Game event kinds to React.
mod game_events {
    pub const ATOM_COUNT: f32 = 1.0;
    pub const BOND_COUNT: f32 = 2.0;
    pub const SELECTED_ELEMENT: f32 = 3.0;
    pub const VSEPR_GEOMETRY: f32 = 4.0;
}

/// The Chemistry Lab game.
pub struct ChemistryLab {
    sim: ChemistrySim,
    camera: Camera3D,
    interaction: InteractionSystem,
    renderer: MoleculeRenderer,
    /// When true, camera follows molecule centroid. Disabled by manual camera controls.
    auto_follow: bool,
}

impl ChemistryLab {
    pub fn new() -> Self {
        let registry = ElementRegistry::load()
            .expect("Failed to load periodic table");

        Self {
            sim: ChemistrySim::new(registry),
            camera: Camera3D::new(WORLD_W, WORLD_H),
            interaction: InteractionSystem::new(),
            renderer: MoleculeRenderer::new(),
            auto_follow: true,
        }
    }

    /// Handle custom events from React UI.
    fn handle_custom_event(&mut self, kind: u32, a: f32, b: f32, c: f32) {
        match kind {
            events::SELECT_ELEMENT => {
                self.sim.set_selected_element(a as u32);
            }
            events::CLEAR => {
                self.sim.clear();
                self.interaction.reset();
            }
            events::RESET_CAMERA => {
                self.camera.reset();
                self.auto_follow = true; // Re-enable auto-follow on reset
            }
            // Camera controls - disable auto-follow when user takes manual control
            events::CAMERA_ZOOM => {
                // a = direction (+1 zoom in, -1 zoom out)
                // b = normalized screen x (0-1), c = normalized screen y (0-1)
                // If b/c are provided (non-zero), use zoom-to-point
                if b != 0.0 || c != 0.0 {
                    self.camera.zoom_toward(a, b, c);
                } else {
                    self.camera.zoom(a);
                }
                self.auto_follow = false;
            }
            events::CAMERA_PAN_X => {
                // Negate: moving target right makes view shift left, but user expects opposite
                self.camera.pan(-a, 0.0, 0.0);
                self.auto_follow = false;
            }
            events::CAMERA_PAN_Y => {
                self.camera.pan(0.0, -a, 0.0);
                self.auto_follow = false;
            }
            events::CAMERA_PAN_Z => {
                self.camera.pan(0.0, 0.0, -a);
                self.auto_follow = false;
            }
            events::CAMERA_ROTATE_AZIMUTH => {
                self.camera.rotate_azimuth(a);
                self.auto_follow = false;
            }
            events::CAMERA_ROTATE_ELEVATION => {
                self.camera.rotate_elevation(a);
                self.auto_follow = false;
            }
            _ => {}
        }
    }

    /// Process interaction result.
    fn handle_interaction(&mut self, result: InteractionResult) {
        match result {
            InteractionResult::SpawnAtom { position } => {
                self.sim.spawn_atom(position);
            }
            InteractionResult::CreateBond { from, to } => {
                self.sim.try_bond(from, to);
            }
            InteractionResult::SelectAtom { atom_idx } => {
                self.sim.set_selected_atom(Some(atom_idx));
            }
            InteractionResult::CameraOrbited => {
                self.auto_follow = false; // Disable auto-follow when user orbits manually
            }
            InteractionResult::None => {}
        }
    }

    /// Emit game events to React.
    fn emit_events(&self, ctx: &mut EngineContext) {
        // Atom and bond counts
        ctx.emit_event(GameEvent {
            kind: game_events::ATOM_COUNT,
            a: self.sim.atom_count() as f32,
            b: 0.0,
            c: 0.0,
        });
        ctx.emit_event(GameEvent {
            kind: game_events::BOND_COUNT,
            a: self.sim.bond_count() as f32,
            b: 0.0,
            c: 0.0,
        });

        // Selected atom info
        if let Some(atom_idx) = self.sim.selected_atom() {
            if let Some(atom) = self.sim.molecule().atoms.get(atom_idx) {
                ctx.emit_event(GameEvent {
                    kind: game_events::SELECTED_ELEMENT,
                    a: atom.element_kind as f32,
                    b: 0.0,
                    c: 0.0,
                });

                // Use geometry with lone pairs for accurate VSEPR
                let geometry = atom.geometry_with_registry(self.sim.registry());
                ctx.emit_event(GameEvent {
                    kind: game_events::VSEPR_GEOMETRY,
                    a: geometry.bond_angle_degrees(),
                    b: atom.bonds.len() as f32,
                    c: 0.0,
                });
            }
        }
    }
}

impl Game for ChemistryLab {
    fn config(&self) -> GameConfig {
        GameConfig {
            fixed_dt: 1.0 / 60.0,
            world_width: WORLD_W,
            world_height: WORLD_H,
            max_sdf_instances: 4096,  // 1024 atoms + up to 3072 bonds
            max_vector_vertices: 4096,
            gravity: Vec2::ZERO,
            ..GameConfig::default()
        }
    }

    fn init(&mut self, _ctx: &mut EngineContext) {
        self.camera.target = Vec3::ZERO;
    }

    fn update(&mut self, ctx: &mut EngineContext, input: &InputQueue) {
        // Clear effects from previous frame
        ctx.effects.clear();

        // Process input events
        for event in input.iter() {
            match event {
                InputEvent::Custom { kind, a, b, c } => {
                    self.handle_custom_event(*kind, *a, *b, *c);
                }
                InputEvent::PointerDown { x, y } => {
                    let pos = Vec2::new(*x, *y);
                    let result = self.interaction.on_pointer_down(
                        pos,
                        self.sim.molecule(),
                        &self.camera,
                        self.sim.registry(),
                    );
                    self.handle_interaction(result);
                }
                InputEvent::PointerMove { x, y } => {
                    let pos = Vec2::new(*x, *y);
                    let result = self.interaction.on_pointer_move(pos, &mut self.camera);
                    self.handle_interaction(result);
                }
                InputEvent::PointerUp { x, y } => {
                    let pos = Vec2::new(*x, *y);
                    let result = self.interaction.on_pointer_up(
                        pos,
                        self.sim.molecule(),
                        &self.camera,
                        self.sim.registry(),
                    );
                    self.handle_interaction(result);
                }
                _ => {}
            }
        }

        // Step physics simulation (using fixed timestep)
        self.sim.update(1.0 / 60.0);

        // Smooth camera follow (only when auto-follow is enabled)
        if self.auto_follow && self.sim.atom_count() > 0 {
            let centroid = self.sim.centroid();
            self.camera.target = Vec3::new(
                self.camera.target.x * 0.95 + centroid.x * 0.05,
                self.camera.target.y * 0.95 + centroid.y * 0.05,
                self.camera.target.z * 0.95 + centroid.z * 0.05,
            );
        }

        // Prepare visual effects
        let effects = FrameEffects {
            bond_preview: self.interaction.bond_drag_source().map(|from_atom| {
                BondPreview {
                    from_atom_idx: from_atom,
                    to_screen_pos: self.interaction.pointer_pos(),
                }
            }),
        };

        // Render
        self.renderer.sync_visuals(
            ctx,
            self.sim.molecule(),
            &self.camera,
            self.sim.registry(),
            &effects,
        );

        // Emit events to React
        self.emit_events(ctx);
    }
}

impl Default for ChemistryLab {
    fn default() -> Self {
        Self::new()
    }
}
