use yatmux::config::{ExperimentalConfig, FontScaleClampConfig};

#[test]
fn test_font_scale_clamp_normalized_default() {
    let config = FontScaleClampConfig::default();
    let (min, max) = config.normalized();
    assert_eq!(min, 1.0);
    assert_eq!(max, 8.0);
}

#[test]
fn test_font_scale_clamp_normalized_swaps_if_inverted() {
    let config = FontScaleClampConfig { min: 8.0, max: 1.0 };
    let (min, max) = config.normalized();
    assert_eq!(min, 1.0);
    assert_eq!(max, 8.0);
}

#[test]
fn test_font_scale_clamp_normalized_handles_infinity() {
    let config = FontScaleClampConfig {
        min: f32::NEG_INFINITY,
        max: f32::INFINITY,
    };
    let (min, max) = config.normalized();
    assert_eq!(min, 1.0);
    assert_eq!(max, 8.0);
}

#[test]
fn test_font_scale_clamp_normalized_clamps_to_safe_range() {
    let config = FontScaleClampConfig { min: 0.1, max: 100.0 };
    let (min, max) = config.normalized();
    assert_eq!(min, 0.25); // Clamped to minimum
    assert_eq!(max, 64.0); // Clamped to maximum
}

#[test]
fn test_font_scale_clamp_normalized_handles_nan() {
    let config = FontScaleClampConfig {
        min: f32::NAN,
        max: f32::NAN,
    };
    let (min, max) = config.normalized();
    assert_eq!(min, 1.0);
    assert_eq!(max, 8.0);
}

#[test]
fn test_font_scale_clamp_config_integration() {
    let toml = r#"
        [font_scale_clamp]
        min = 0.5
        max = 16.0
    "#;

    let experimental: ExperimentalConfig = toml::from_str(toml).unwrap();
    let (min, max) = experimental.font_scale_clamp.normalized();
    assert_eq!(min, 0.5);
    assert_eq!(max, 16.0);
}

#[test]
fn test_fractional_font_scale_increments() {
    // Test that 0.25 increments work as expected
    let scales = vec![0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0];
    for scale in scales {
        let config = FontScaleClampConfig { min: 0.25, max: 8.0 };
        let (min, max) = config.normalized();
        assert!(
            scale >= min && scale <= max,
            "Scale {} should be within range [{}, {}]",
            scale,
            min,
            max
        );
    }
}
