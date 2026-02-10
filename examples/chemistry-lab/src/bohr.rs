//! Bohr model visualization for atoms.
//!
//! Renders electron shells as concentric rings and electrons as small circles.

use glam::Vec2;
use zap_engine::EngineContext;
use zap_engine::systems::vector::VectorColor;
use crate::periodic_table::ElementData;

/// Configuration for Bohr model rendering.
#[derive(Debug, Clone)]
pub struct BohrConfig {
    /// Base radius of nucleus (before scaling by element).
    pub nucleus_base_radius: f32,
    /// Radius of electron dots.
    pub electron_radius: f32,
    /// Gap between shell rings.
    pub shell_gap: f32,
    /// Line width for shell rings.
    pub shell_line_width: f32,
    /// Alpha for shell rings.
    pub shell_alpha: f32,
    /// Alpha for electron dots.
    pub electron_alpha: f32,
    /// Maximum shells to render (for visual clarity).
    pub max_visual_shells: usize,
    /// Shell ring color.
    pub shell_color: [f32; 3],
    /// Electron dot color.
    pub electron_color: [f32; 3],
}

impl Default for BohrConfig {
    fn default() -> Self {
        Self {
            nucleus_base_radius: 15.0,
            electron_radius: 5.0,
            shell_gap: 18.0,
            shell_line_width: 1.5,
            shell_alpha: 0.35,
            electron_alpha: 0.9,
            max_visual_shells: 4,
            shell_color: [0.6, 0.8, 1.0],    // Light blue
            electron_color: [0.3, 0.6, 1.0], // Blue
        }
    }
}

/// Render Bohr model for an element at a given position.
///
/// Draws concentric shell rings and electron dots positioned on each shell.
/// The nucleus is already rendered as an SDF sphere by the atom entity.
pub fn render_bohr_model(
    ctx: &mut EngineContext,
    element: &ElementData,
    center: Vec2,
    config: &BohrConfig,
    scale: f32,
) {
    let nucleus_radius = config.nucleus_base_radius * scale;
    let shell_gap = config.shell_gap * scale;
    let electron_radius = config.electron_radius * scale;
    let line_width = config.shell_line_width * scale;

    // Limit shells for visual clarity
    let visible_shells = element.shells.len().min(config.max_visual_shells);

    // Draw each shell
    for (shell_idx, &electron_count) in element.shells.iter().take(visible_shells).enumerate() {
        let shell_radius = nucleus_radius + (shell_idx as f32 + 1.0) * shell_gap;

        // Draw shell ring
        let shell_color = VectorColor::new(
            config.shell_color[0],
            config.shell_color[1],
            config.shell_color[2],
            config.shell_alpha,
        );
        ctx.vectors.stroke_circle(center, shell_radius, line_width, shell_color);

        // Draw electrons on this shell
        if electron_count > 0 {
            let electron_color = VectorColor::new(
                config.electron_color[0],
                config.electron_color[1],
                config.electron_color[2],
                config.electron_alpha,
            );

            for i in 0..electron_count {
                let angle = (i as f32 / electron_count as f32) * std::f32::consts::TAU;
                let pos = center + Vec2::new(
                    shell_radius * angle.cos(),
                    shell_radius * angle.sin(),
                );
                ctx.vectors.fill_circle(pos, electron_radius, electron_color);
            }
        }
    }

    // If there are more shells than we're showing, indicate with a dashed outer ring
    if element.shells.len() > config.max_visual_shells {
        let outer_radius = nucleus_radius + (config.max_visual_shells as f32 + 0.5) * shell_gap;
        let indicator_color = VectorColor::new(0.5, 0.5, 0.5, 0.2);
        ctx.vectors.stroke_circle(center, outer_radius, line_width * 0.5, indicator_color);
    }
}

/// Calculate total visual radius for an element's Bohr model.
pub fn bohr_model_radius(element: &ElementData, config: &BohrConfig) -> f32 {
    let visible_shells = element.shells.len().min(config.max_visual_shells);
    config.nucleus_base_radius + (visible_shells as f32) * config.shell_gap + config.electron_radius
}

/// Render a simplified Bohr model (just shells, no electrons) for background.
pub fn render_bohr_shells_only(
    ctx: &mut EngineContext,
    shell_count: usize,
    center: Vec2,
    config: &BohrConfig,
    scale: f32,
) {
    let nucleus_radius = config.nucleus_base_radius * scale;
    let shell_gap = config.shell_gap * scale;
    let line_width = config.shell_line_width * scale;

    let visible_shells = shell_count.min(config.max_visual_shells);

    for shell_idx in 0..visible_shells {
        let shell_radius = nucleus_radius + (shell_idx as f32 + 1.0) * shell_gap;
        let shell_color = VectorColor::new(
            config.shell_color[0],
            config.shell_color[1],
            config.shell_color[2],
            config.shell_alpha * 0.5, // Dimmer for background
        );
        ctx.vectors.stroke_circle(center, shell_radius, line_width, shell_color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bohr_radius_hydrogen() {
        let config = BohrConfig::default();
        let h = crate::periodic_table::ElementRegistry::load()
            .unwrap()
            .get(1)
            .cloned()
            .unwrap();

        // Hydrogen: 1 shell
        let radius = bohr_model_radius(&h, &config);
        // nucleus (15) + 1 shell (18) + electron radius (5) = 38
        assert!((radius - 38.0).abs() < 0.01);
    }

    #[test]
    fn bohr_radius_carbon() {
        let config = BohrConfig::default();
        let c = crate::periodic_table::ElementRegistry::load()
            .unwrap()
            .get(6)
            .cloned()
            .unwrap();

        // Carbon: 2 shells
        let radius = bohr_model_radius(&c, &config);
        // nucleus (15) + 2 shells (36) + electron radius (5) = 56
        assert!((radius - 56.0).abs() < 0.01);
    }

    #[test]
    fn config_defaults() {
        let config = BohrConfig::default();
        assert_eq!(config.max_visual_shells, 4);
        assert!(config.shell_alpha < 1.0);
    }
}
