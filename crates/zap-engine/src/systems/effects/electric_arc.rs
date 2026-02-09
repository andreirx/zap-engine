//! Electric arc with midpoint displacement algorithm.

use super::rng::Rng;

/// An electric arc rendered using midpoint displacement.
#[derive(Debug, Clone)]
pub struct ElectricArc {
    pub start: [f32; 2],
    pub end: [f32; 2],
    pub points: Vec<[f32; 2]>,
    displacements: Vec<f32>,
    num_segments: usize,
    max_displacement: f32,
}

impl ElectricArc {
    pub fn new(start: [f32; 2], end: [f32; 2], power_of_two: u32, rng: &mut Rng) -> Self {
        let num_segments = 1usize << power_of_two;
        let mut arc = ElectricArc {
            start,
            end,
            points: vec![[0.0; 2]; num_segments + 1],
            displacements: vec![0.0; num_segments.saturating_sub(1)],
            num_segments,
            max_displacement: 0.2,
        };
        arc.points[0] = start;
        arc.points[num_segments] = end;
        arc.generate_displacements(rng);
        arc.generate_points(0, num_segments);
        arc
    }

    fn generate_displacements(&mut self, rng: &mut Rng) {
        for d in self.displacements.iter_mut() {
            let r = (rng.next_int(10000) as f32 / 5000.0) - 1.0;
            *d = r * self.max_displacement;
        }
    }

    fn generate_points(&mut self, start_idx: usize, end_idx: usize) {
        if end_idx - start_idx <= 1 {
            return;
        }
        let mid_idx = (start_idx + end_idx) / 2;
        let sp = self.points[start_idx];
        let ep = self.points[end_idx];
        let mid = [(sp[0] + ep[0]) * 0.5, (sp[1] + ep[1]) * 0.5];

        let dx = ep[0] - sp[0];
        let dy = ep[1] - sp[1];
        let len = (dx * dx + dy * dy).sqrt().max(0.001);
        let perp = [-dy / len, dx / len];

        let disp_idx = mid_idx.saturating_sub(1);
        let disp = if disp_idx < self.displacements.len() {
            self.displacements[disp_idx]
        } else {
            0.0
        };
        let dist = (dx * dx + dy * dy).sqrt();
        self.points[mid_idx] = [
            mid[0] + perp[0] * disp * dist,
            mid[1] + perp[1] * disp * dist,
        ];

        self.generate_points(start_idx, mid_idx);
        self.generate_points(mid_idx, end_idx);
    }

    /// Jitter the arc displacements for a living-wire effect.
    pub fn twitch(&mut self, factor: f32, rng: &mut Rng) {
        for d in self.displacements.iter_mut() {
            let r = (rng.next_int(10000) as f32 / 5000.0) - 1.0;
            *d += factor * r;
            *d = d.clamp(-self.max_displacement, self.max_displacement);
        }
        self.points[0] = self.start;
        self.points[self.num_segments] = self.end;
        self.generate_points(0, self.num_segments);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn electric_arc_generates_correct_point_count() {
        let mut rng = Rng::new(42);
        let arc = ElectricArc::new([0.0, 0.0], [100.0, 0.0], 3, &mut rng);
        assert_eq!(arc.points.len(), 9); // 2^3 + 1
    }

    #[test]
    fn electric_arc_po2_4() {
        let mut rng = Rng::new(42);
        let arc = ElectricArc::new([0.0, 0.0], [100.0, 0.0], 4, &mut rng);
        assert_eq!(arc.points.len(), 17); // 2^4 + 1
    }

    #[test]
    fn electric_arc_twitch_modifies_points() {
        let mut rng = Rng::new(42);
        let mut arc = ElectricArc::new([0.0, 0.0], [100.0, 0.0], 3, &mut rng);
        let points_before = arc.points.clone();
        arc.twitch(0.1, &mut rng);
        // At least some interior points should change
        assert_ne!(arc.points[4], points_before[4]);
    }
}
