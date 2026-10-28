//! State for the wallpaper's idle scene rotation.

/// Starts rotating scenes after 90 seconds without input and restores the
/// user's chosen scene as soon as input resumes.
#[derive(Default)]
pub struct IdleCycle {
    original_scene: Option<String>,
    last_change: f64,
}

impl IdleCycle {
    pub const IDLE_THRESHOLD_SECONDS: f64 = 90.0;
    pub const CHANGE_INTERVAL_SECONDS: f64 = 20.0;

    /// Return a scene id when the wallpaper should change.
    pub fn update(
        &mut self,
        current_scene: &str,
        idle_seconds: f64,
        now_seconds: f64,
        scenes: &[&str],
    ) -> Option<String> {
        if scenes.is_empty() {
            return None;
        }

        if idle_seconds < Self::IDLE_THRESHOLD_SECONDS {
            return self
                .original_scene
                .take()
                .filter(|original| original != current_scene);
        }

        if self.original_scene.is_none() {
            self.original_scene = Some(current_scene.to_owned());
            self.last_change = now_seconds;
            return None;
        }

        if now_seconds - self.last_change < Self::CHANGE_INTERVAL_SECONDS {
            return None;
        }

        self.last_change = now_seconds;
        let next = scenes
            .iter()
            .position(|scene| *scene == current_scene)
            .map_or(0, |index| (index + 1) % scenes.len());
        Some(scenes[next].to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::IdleCycle;

    const SCENES: &[&str] = &["donut", "helix", "matrix"];

    #[test]
    fn waits_before_starting_and_between_scene_changes() {
        let mut cycle = IdleCycle::default();
        assert_eq!(cycle.update("donut", 89.0, 0.0, SCENES), None);
        assert_eq!(cycle.update("donut", 90.0, 1.0, SCENES), None);
        assert_eq!(cycle.update("donut", 110.0, 20.9, SCENES), None);
        assert_eq!(
            cycle.update("donut", 110.0, 21.0, SCENES),
            Some("helix".into())
        );
        assert_eq!(cycle.update("helix", 130.0, 40.9, SCENES), None);
        assert_eq!(
            cycle.update("helix", 130.0, 41.0, SCENES),
            Some("matrix".into())
        );
    }

    #[test]
    fn restores_the_scene_chosen_before_idle() {
        let mut cycle = IdleCycle::default();
        assert_eq!(cycle.update("helix", 90.0, 0.0, SCENES), None);
        assert_eq!(
            cycle.update("helix", 110.0, 20.0, SCENES),
            Some("matrix".into())
        );
        assert_eq!(
            cycle.update("matrix", 0.0, 21.0, SCENES),
            Some("helix".into())
        );
        assert_eq!(cycle.update("helix", 0.0, 22.0, SCENES), None);
    }

    #[test]
    fn ignores_empty_scene_lists_and_restores_unchanged_scene() {
        let mut cycle = IdleCycle::default();
        assert_eq!(cycle.update("donut", 120.0, 0.0, &[]), None);
        assert_eq!(cycle.update("donut", 90.0, 0.0, SCENES), None);
        assert_eq!(cycle.update("donut", 0.0, 1.0, SCENES), None);
    }

    #[test]
    fn starts_at_first_scene_when_current_scene_is_unlisted() {
        let mut cycle = IdleCycle::default();
        assert_eq!(cycle.update("custom", 90.0, 0.0, SCENES), None);
        assert_eq!(
            cycle.update("custom", 110.0, 20.0, SCENES),
            Some("donut".into())
        );
    }
}
