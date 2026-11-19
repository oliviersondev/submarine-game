//! Local, north-up chart geometry. Fictional geographic coordinates are presentation only.
use bevy::prelude::*;

pub(crate) const METERS_PER_NM: f32 = 1852.0;
const REFERENCE_LAT: f64 = 48.0;
const REFERENCE_LON: f64 = -10.0;

#[derive(Resource, Debug, Clone)]
pub(crate) struct ChartView {
    pub center: Vec2,
    pub size: Vec2,
    /// Half the visible distance along the shorter axis, in metres.
    pub half_range: f32,
    pub following: bool,
}

impl Default for ChartView {
    fn default() -> Self {
        Self {
            center: Vec2::ZERO,
            size: Vec2::new(320.0, 180.0),
            half_range: 10_000.0,
            following: true,
        }
    }
}

impl ChartView {
    pub fn scale(&self) -> f32 {
        self.size.min_element().max(1.0) / (self.half_range * 2.0)
    }

    pub fn project(&self, world: Vec2) -> Vec2 {
        let delta = (world - self.center) * self.scale();
        self.size * 0.5 + Vec2::new(delta.x, -delta.y)
    }

    pub fn unproject(&self, screen: Vec2) -> Vec2 {
        let delta = (screen - self.size * 0.5) / self.scale();
        self.center + Vec2::new(delta.x, -delta.y)
    }

    pub fn contains(&self, point: Vec2) -> bool {
        point.x >= 0.0 && point.y >= 0.0 && point.x <= self.size.x && point.y <= self.size.y
    }

    pub fn zoom(&mut self, factor: f32) {
        if factor.is_finite() && factor > 0.0 {
            self.half_range = (self.half_range * factor).clamp(500.0, 20_000.0);
        }
    }

    pub fn pan(&mut self, delta: Vec2) {
        if delta.is_finite() {
            self.center += Vec2::new(-delta.x, delta.y) / self.scale();
            self.center = self
                .center
                .clamp(Vec2::splat(-200_000.0), Vec2::splat(200_000.0));
            self.following = false;
        }
    }

    /// Clip a screen-space line, without pinning an off-screen object to the border.
    pub fn clip_line(&self, a: Vec2, b: Vec2) -> Option<(Vec2, Vec2)> {
        let delta = b - a;
        let mut low: f32 = 0.0;
        let mut high: f32 = 1.0;
        for (p, q) in [
            (-delta.x, a.x),
            (delta.x, self.size.x - a.x),
            (-delta.y, a.y),
            (delta.y, self.size.y - a.y),
        ] {
            if p.abs() < f32::EPSILON {
                if q < 0.0 {
                    return None;
                }
            } else if p < 0.0 {
                low = low.max(q / p);
            } else {
                high = high.min(q / p);
            }
        }
        (low <= high).then_some((a + low * delta, a + high * delta))
    }
}

pub(crate) fn direction(bearing: f32) -> Vec2 {
    let angle = bearing.to_radians();
    Vec2::new(angle.sin(), angle.cos())
}

pub(crate) fn coordinates(world: Vec2) -> (f64, f64) {
    let latitude = REFERENCE_LAT + f64::from(world.y) / (60.0 * f64::from(METERS_PER_NM));
    let longitude = REFERENCE_LON
        + f64::from(world.x) / (60.0 * f64::from(METERS_PER_NM) * REFERENCE_LAT.to_radians().cos());
    (latitude, longitude)
}

pub(crate) fn coordinate_label(degrees: f64, latitude: bool) -> String {
    let hemisphere = match (latitude, degrees >= 0.0) {
        (true, true) => "N",
        (true, false) => "S",
        (false, true) => "E",
        (false, false) => "O",
    };
    // Round total hundredths of minutes before splitting, avoiding a displayed 60.00'.
    let minutes = (degrees.abs() * 60.0 * 100.0).round() as u64;
    format!(
        "{:02}d{:05.2}'{}",
        minutes / 6000,
        (minutes % 6000) as f64 / 100.0,
        hemisphere
    )
}

pub(crate) fn grid_step(scale: f32, longitude: bool) -> f32 {
    let minute = METERS_PER_NM
        * if longitude {
            (REFERENCE_LAT as f32).to_radians().cos()
        } else {
            1.0
        };
    [0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 30.0, 60.0]
        .into_iter()
        .map(|n| n * minute)
        .find(|step| step * scale >= 100.0)
        .unwrap_or(60.0 * minute)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_is_uniform_north_up_and_invertible_in_every_format() {
        for size in [
            Vec2::new(320.0, 180.0),
            Vec2::new(1800.0, 300.0),
            Vec2::new(300.0, 500.0),
        ] {
            let view = ChartView {
                size,
                center: Vec2::new(250.0, -300.0),
                ..default()
            };
            let center = view.project(view.center);
            let east = view.project(view.center + Vec2::X * 500.0);
            let north = view.project(view.center + Vec2::Y * 500.0);
            assert!((east.distance(center) - north.distance(center)).abs() < 0.001);
            assert!(north.y < center.y && east.x > center.x);
            let world = Vec2::new(-1234.0, 9876.0);
            assert!(view.unproject(view.project(world)).distance(world) < 0.01);
            assert!(!view.contains(view.project(view.center + Vec2::Y * 30_000.0)));
        }
    }

    #[test]
    fn pan_detaches_following_and_zoom_is_bounded_and_keeps_center() {
        let mut view = ChartView::default();
        view.pan(Vec2::new(30.0, -15.0));
        assert!(!view.following);
        assert!(view.center.x < 0.0 && view.center.y < 0.0);
        let center = view.center;
        view.zoom(0.0001);
        assert_eq!(view.half_range, 500.0);
        view.zoom(1_000_000.0);
        assert_eq!(view.half_range, 20_000.0);
        view.zoom(f32::NAN);
        assert_eq!(view.center, center);
        assert_eq!(view.half_range, 20_000.0);
    }

    #[test]
    fn fictional_coordinates_have_consistent_nautical_units_and_rounding() {
        assert_eq!(coordinates(Vec2::ZERO), (48.0, -10.0));
        assert_eq!(coordinate_label(48.0, true), "48d00.00'N");
        assert_eq!(coordinate_label(-10.0, false), "10d00.00'O");
        assert_eq!(coordinate_label(47.9999999, true), "48d00.00'N");
        assert!((coordinates(Vec2::Y * METERS_PER_NM).0 - (48.0 + 1.0 / 60.0)).abs() < 1e-6);
    }

    #[test]
    fn clipping_preserves_visible_line_and_does_not_invent_edge_positions() {
        let view = ChartView::default();
        assert_eq!(
            view.clip_line(Vec2::new(-10.0, 50.0), Vec2::new(500.0, 50.0)),
            Some((Vec2::new(0.0, 50.0), Vec2::new(320.0, 50.0)))
        );
        assert!(view
            .clip_line(Vec2::new(-20.0, 0.0), Vec2::new(-20.0, 100.0))
            .is_none());
    }
}
