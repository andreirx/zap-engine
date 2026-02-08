use glam::Vec2;

/// State of a single stroke trace attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceState {
    /// Waiting for the user to start drawing.
    Idle,
    /// User is actively tracing the stroke.
    Tracing,
    /// Stroke completed successfully.
    Complete,
}

/// Result of processing a pointer event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceResult {
    /// Point accepted — on path, correct direction.
    Accepted,
    /// Touch wasn't near the stroke start.
    NotStarted,
    /// Point rejected — off path or wrong direction.
    Rejected,
    /// The stroke has been completed.
    StrokeComplete,
    /// No action (e.g., pointer up while idle).
    Ignored,
}

/// Validates user touch input against a reference stroke path.
///
/// The path is a sequence of evenly-spaced points (from the baked glyph data,
/// already mapped to world coordinates). The tracer checks that the user's
/// touch stays close to the path and moves in the correct direction.
pub struct StrokeTracer {
    /// Reference path points (world coordinates).
    path: Vec<Vec2>,
    /// Index of the furthest point the user has reached along the path.
    progress_idx: usize,
    /// Accepted user touch points (for rendering the green stroke).
    pub user_points: Vec<Vec2>,
    /// Maximum allowed distance from the path (world units).
    threshold: f32,
    /// Current state of this trace attempt.
    pub state: TraceState,
    /// How many path points from the end count as "complete".
    complete_margin: usize,
    /// Maximum allowed backward steps (for finger wiggle tolerance).
    backtrack_tolerance: usize,
}

impl StrokeTracer {
    /// Create a new tracer for a stroke path.
    ///
    /// - `path`: Reference points in world coordinates
    /// - `threshold`: Max distance from path in world units (~30-40 for touchscreen)
    pub fn new(path: Vec<Vec2>, threshold: f32) -> Self {
        let path_len = path.len();
        Self {
            path,
            progress_idx: 0,
            user_points: Vec::with_capacity(256),
            threshold,
            state: TraceState::Idle,
            complete_margin: 3.min(path_len.saturating_sub(1)),
            backtrack_tolerance: 3,
        }
    }

    /// Handle pointer down — user starts touching the screen.
    pub fn on_pointer_down(&mut self, pos: Vec2) -> TraceResult {
        if self.path.is_empty() {
            return TraceResult::Ignored;
        }

        match self.state {
            TraceState::Idle => {
                // Check if touch is near the start of the stroke
                let dist = pos.distance(self.path[0]);
                if dist <= self.threshold {
                    self.state = TraceState::Tracing;
                    self.progress_idx = 0;
                    self.user_points.clear();
                    self.user_points.push(pos);
                    TraceResult::Accepted
                } else {
                    TraceResult::NotStarted
                }
            }
            TraceState::Tracing => TraceResult::Ignored,
            TraceState::Complete => TraceResult::Ignored,
        }
    }

    /// Handle pointer move — user drags finger.
    pub fn on_pointer_move(&mut self, pos: Vec2) -> TraceResult {
        if self.state != TraceState::Tracing || self.path.is_empty() {
            return TraceResult::Ignored;
        }

        // Find nearest point on path ahead of (or near) current progress.
        // Search window: from (progress - backtrack_tolerance) to end.
        let search_start = self.progress_idx.saturating_sub(self.backtrack_tolerance);
        let (nearest_idx, nearest_dist) = self.find_nearest_in_range(pos, search_start);

        if nearest_dist > self.threshold {
            // Too far from path
            return TraceResult::Rejected;
        }

        // Check direction: nearest point should be at or ahead of progress (with tolerance)
        if nearest_idx + self.backtrack_tolerance < self.progress_idx {
            // Moving too far backward
            return TraceResult::Rejected;
        }

        // Accept the point
        if nearest_idx > self.progress_idx {
            self.progress_idx = nearest_idx;
        }
        self.user_points.push(pos);

        // Check if we've reached the end
        if self.progress_idx + self.complete_margin >= self.path.len() {
            self.state = TraceState::Complete;
            // Add the final path point for a clean finish
            if let Some(&last) = self.path.last() {
                self.user_points.push(last);
            }
            return TraceResult::StrokeComplete;
        }

        TraceResult::Accepted
    }

    /// Handle pointer up — user lifts finger.
    pub fn on_pointer_up(&mut self) -> TraceResult {
        match self.state {
            TraceState::Tracing => {
                // Lifted finger before completing — reject the attempt
                TraceResult::Rejected
            }
            _ => TraceResult::Ignored,
        }
    }

    /// Reset the tracer to idle state (for retry after rejection).
    pub fn reset(&mut self) {
        self.state = TraceState::Idle;
        self.progress_idx = 0;
        self.user_points.clear();
    }

    /// Get the current progress as a fraction (0.0 to 1.0).
    pub fn progress_fraction(&self) -> f32 {
        if self.path.is_empty() {
            return 0.0;
        }
        self.progress_idx as f32 / (self.path.len() - 1) as f32
    }

    /// Find the nearest path point to `pos` in the range [start..path.len()).
    /// Returns (index, distance).
    fn find_nearest_in_range(&self, pos: Vec2, start: usize) -> (usize, f32) {
        let mut best_idx = start;
        let mut best_dist = f32::MAX;

        // Search forward from start, but limit search window to avoid
        // matching far-ahead points that happen to be close (e.g., loops).
        let search_end = (self.progress_idx + 30).min(self.path.len());
        let effective_start = start.min(search_end);

        for i in effective_start..search_end {
            let dist = pos.distance(self.path[i]);
            if dist < best_dist {
                best_dist = dist;
                best_idx = i;
            }
        }

        (best_idx, best_dist)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_horizontal_path(n: usize) -> Vec<Vec2> {
        (0..n)
            .map(|i| Vec2::new(100.0 + i as f32 * 5.0, 300.0))
            .collect()
    }

    #[test]
    fn start_near_path_beginning() {
        let path = make_horizontal_path(20);
        let mut tracer = StrokeTracer::new(path, 40.0);

        let result = tracer.on_pointer_down(Vec2::new(105.0, 305.0));
        assert_eq!(result, TraceResult::Accepted);
        assert_eq!(tracer.state, TraceState::Tracing);
    }

    #[test]
    fn start_too_far_from_beginning() {
        let path = make_horizontal_path(20);
        let mut tracer = StrokeTracer::new(path, 40.0);

        let result = tracer.on_pointer_down(Vec2::new(300.0, 300.0));
        assert_eq!(result, TraceResult::NotStarted);
        assert_eq!(tracer.state, TraceState::Idle);
    }

    #[test]
    fn trace_along_path_completes() {
        let path = make_horizontal_path(20);
        let mut tracer = StrokeTracer::new(path, 40.0);

        tracer.on_pointer_down(Vec2::new(100.0, 300.0));

        // Move along the path — stop before complete_margin (3 from end)
        for i in 1..15 {
            let result = tracer.on_pointer_move(Vec2::new(100.0 + i as f32 * 5.0, 302.0));
            assert_eq!(result, TraceResult::Accepted, "point {i} should be accepted");
        }

        // Moving near the end should trigger completion (within complete_margin)
        let result = tracer.on_pointer_move(Vec2::new(100.0 + 19.0 * 5.0, 300.0));
        assert_eq!(result, TraceResult::StrokeComplete);
        assert_eq!(tracer.state, TraceState::Complete);
    }

    #[test]
    fn reject_off_path() {
        let path = make_horizontal_path(20);
        let mut tracer = StrokeTracer::new(path, 20.0);

        tracer.on_pointer_down(Vec2::new(100.0, 300.0));

        // Move far from path
        let result = tracer.on_pointer_move(Vec2::new(120.0, 400.0));
        assert_eq!(result, TraceResult::Rejected);
    }

    #[test]
    fn reject_pointer_up_midway() {
        let path = make_horizontal_path(20);
        let mut tracer = StrokeTracer::new(path, 40.0);

        tracer.on_pointer_down(Vec2::new(100.0, 300.0));
        tracer.on_pointer_move(Vec2::new(120.0, 300.0));

        let result = tracer.on_pointer_up();
        assert_eq!(result, TraceResult::Rejected);
    }

    #[test]
    fn reset_allows_retry() {
        let path = make_horizontal_path(20);
        let mut tracer = StrokeTracer::new(path, 40.0);

        tracer.on_pointer_down(Vec2::new(100.0, 300.0));
        tracer.reset();

        assert_eq!(tracer.state, TraceState::Idle);
        assert!(tracer.user_points.is_empty());

        // Can start again
        let result = tracer.on_pointer_down(Vec2::new(100.0, 300.0));
        assert_eq!(result, TraceResult::Accepted);
    }

    #[test]
    fn progress_fraction() {
        let path = make_horizontal_path(20);
        let mut tracer = StrokeTracer::new(path, 40.0);

        assert_eq!(tracer.progress_fraction(), 0.0);

        tracer.on_pointer_down(Vec2::new(100.0, 300.0));
        for i in 1..10 {
            tracer.on_pointer_move(Vec2::new(100.0 + i as f32 * 5.0, 300.0));
        }

        assert!(tracer.progress_fraction() > 0.0);
        assert!(tracer.progress_fraction() < 1.0);
    }
}
