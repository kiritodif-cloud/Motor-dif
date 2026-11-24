use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DayNightCycle {
    pub hour: f32,
    pub zenith_color: [f32; 3],
    pub horizon_color: [f32; 3],
    pub fog_color: [f32; 3],
    pub fog_density: f32,
    pub sun_dir: [f32; 3],
    pub sun_color: [f32; 3],
    pub sun_intensity: f32,
    pub stars_visibility: f32,
    pub is_day: bool,
}

pub fn normalize_hour(hour: f32) -> f32 {
    let mut h = hour % 24.0;
    if h < 0.0 {
        h += 24.0;
    }
    h
}

fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn lerp_rgb(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        lerp_f32(a[0], b[0], t),
        lerp_f32(a[1], b[1], t),
        lerp_f32(a[2], b[2], t),
    ]
}

pub fn compute_day_night(hour: f32, base_fog_density: f32) -> DayNightCycle {
    let h = normalize_hour(hour);

    // 4 Keyframes:
    // 0:00 (Midnight)
    let mid_zenith = [0.015, 0.02, 0.045];
    let mid_horizon = [0.03, 0.04, 0.08];
    let mid_fog = [0.02, 0.025, 0.05];
    let mid_sun_col = [0.25, 0.38, 0.65]; // cool blue moonlight
    let mid_sun_intensity = 0.15;
    let mid_stars = 1.0;

    // 6:00 (Dawn)
    let dawn_zenith = [0.18, 0.26, 0.48];
    let dawn_horizon = [0.94, 0.46, 0.20]; // warm golden orange
    let dawn_fog = [0.75, 0.40, 0.26];
    let dawn_sun_col = [1.0, 0.70, 0.40]; // golden morning light
    let dawn_sun_intensity = 0.70;
    let dawn_stars = 0.0;

    // 12:00 (Noon)
    let noon_zenith = [0.12, 0.38, 0.80];
    let noon_horizon = [0.65, 0.80, 0.95];
    let noon_fog = [0.55, 0.72, 0.88];
    let noon_sun_col = [1.0, 0.98, 0.92]; // bright neutral sunlight
    let noon_sun_intensity = 1.0;
    let noon_stars = 0.0;

    // 18.5 (Dusk)
    let dusk_zenith = [0.14, 0.16, 0.32];
    let dusk_horizon = [0.90, 0.32, 0.18]; // rich crimson dusk
    let dusk_fog = [0.68, 0.28, 0.20];
    let dusk_sun_col = [1.0, 0.58, 0.28];
    let dusk_sun_intensity = 0.65;
    let dusk_stars = 0.35;

    let (zenith_color, horizon_color, fog_color, sun_color, sun_intensity, stars_visibility) =
        if h < 6.0 {
            let t = h / 6.0;
            let smooth_t = t * t * (3.0 - 2.0 * t);
            (
                lerp_rgb(mid_zenith, dawn_zenith, smooth_t),
                lerp_rgb(mid_horizon, dawn_horizon, smooth_t),
                lerp_rgb(mid_fog, dawn_fog, smooth_t),
                lerp_rgb(mid_sun_col, dawn_sun_col, smooth_t),
                lerp_f32(mid_sun_intensity, dawn_sun_intensity, smooth_t),
                lerp_f32(mid_stars, dawn_stars, smooth_t),
            )
        } else if h < 12.0 {
            let t = (h - 6.0) / 6.0;
            let smooth_t = t * t * (3.0 - 2.0 * t);
            (
                lerp_rgb(dawn_zenith, noon_zenith, smooth_t),
                lerp_rgb(dawn_horizon, noon_horizon, smooth_t),
                lerp_rgb(dawn_fog, noon_fog, smooth_t),
                lerp_rgb(dawn_sun_col, noon_sun_col, smooth_t),
                lerp_f32(dawn_sun_intensity, noon_sun_intensity, smooth_t),
                lerp_f32(dawn_stars, noon_stars, smooth_t),
            )
        } else if h < 18.5 {
            let t = (h - 12.0) / 6.5;
            let smooth_t = t * t * (3.0 - 2.0 * t);
            (
                lerp_rgb(noon_zenith, dusk_zenith, smooth_t),
                lerp_rgb(noon_horizon, dusk_horizon, smooth_t),
                lerp_rgb(noon_fog, dusk_fog, smooth_t),
                lerp_rgb(noon_sun_col, dusk_sun_col, smooth_t),
                lerp_f32(noon_sun_intensity, dusk_sun_intensity, smooth_t),
                lerp_f32(noon_stars, dusk_stars, smooth_t),
            )
        } else {
            let t = (h - 18.5) / 5.5;
            let smooth_t = t * t * (3.0 - 2.0 * t);
            (
                lerp_rgb(dusk_zenith, mid_zenith, smooth_t),
                lerp_rgb(dusk_horizon, mid_horizon, smooth_t),
                lerp_rgb(dusk_fog, mid_fog, smooth_t),
                lerp_rgb(dusk_sun_col, mid_sun_col, smooth_t),
                lerp_f32(dusk_sun_intensity, mid_sun_intensity, smooth_t),
                lerp_f32(dusk_stars, mid_stars, smooth_t),
            )
        };

    let solar_angle = (h - 6.0) / 24.0 * std::f32::consts::TAU;
    let sun_elev = solar_angle.sin();
    let sun_azim = solar_angle.cos();

    let is_day = h >= 5.5 && h <= 19.0;
    let sun_dir = if is_day {
        Vec3::new(sun_azim * 0.5, sun_elev.max(0.08), -0.3).normalize().to_array()
    } else {
        let moon_elev = (-solar_angle).sin();
        Vec3::new(-sun_azim * 0.5, moon_elev.max(0.12), 0.3).normalize().to_array()
    };

    DayNightCycle {
        hour: h,
        zenith_color,
        horizon_color,
        fog_color,
        fog_density: base_fog_density,
        sun_dir,
        sun_color,
        sun_intensity,
        stars_visibility,
        is_day,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_hour() {
        assert_eq!(normalize_hour(0.0), 0.0);
        assert_eq!(normalize_hour(12.0), 12.0);
        assert_eq!(normalize_hour(24.0), 0.0);
        assert_eq!(normalize_hour(26.5), 2.5);
        assert_eq!(normalize_hour(-1.0), 23.0);
        assert_eq!(normalize_hour(-25.0), 23.0);
    }

    #[test]
    fn test_keyframes_interpolation() {
        let dawn = compute_day_night(6.0, 0.005);
        assert_eq!(dawn.hour, 6.0);
        assert!(dawn.is_day);
        assert!(dawn.horizon_color[0] > 0.8, "Dawn horizon should be warm golden orange");
        assert!(dawn.stars_visibility < 0.01, "Dawn should have no stars");

        let noon = compute_day_night(12.0, 0.005);
        assert_eq!(noon.hour, 12.0);
        assert!(noon.is_day);
        assert_eq!(noon.sun_intensity, 1.0);
        assert!(noon.sun_dir[1] > 0.8, "Sun at noon should be high overhead");
        assert_eq!(noon.stars_visibility, 0.0);

        let dusk = compute_day_night(18.5, 0.005);
        assert_eq!(dusk.hour, 18.5);
        assert!(dusk.horizon_color[0] > 0.8, "Dusk horizon should be fiery");
        assert!(dusk.sun_intensity < 0.7);

        let night = compute_day_night(0.0, 0.005);
        assert_eq!(night.hour, 0.0);
        assert!(!night.is_day);
        assert!(night.stars_visibility > 0.9, "Midnight should have full stars");
        assert!(night.zenith_color[2] < 0.1, "Midnight zenith should be dark");
    }
}
