use ril::Rgb;

fn hsl_to_raw_rgbf(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    // Optimize for grayscale
    if s == 0.0 {
        return (l, l, l);
    }

    let c = (1.0 - 2.0_f64.mul_add(l, -1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());

    let (r, g, b) = match h.rem_euclid(360.0) as u16 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        300.. => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    (r + m, g + m, b + m)
}

/// Converts an HSL color to an RGB color.
///
/// Assumes `h` is within the range `[0.0, 360.0)` and `s`, and `l` are within the range
/// `[0.0, 1.0]`.
pub fn hsl_to_rgb(h: f64, s: f64, l: f64) -> Rgb {
    let (r, g, b) = hsl_to_raw_rgbf(h, s, l);
    Rgb::new(
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    )
}

/// Specifes the perceived middle lightness for each hue
///
/// From <https://github.com/dmester/jdenticon/blob/master/dist/jdenticon-module.js#L137>
const HSL_CORRECTORS: [f64; 6] = [0.55, 0.5, 0.5, 0.46, 0.6, 0.55];

/// Converts an HSL color to an RGB color, correcting for lightness for dark hues.
///
/// Assumes `h` is within the range `[0.0, 360.0)` and `s`, and `l` are within the range
/// `[0.0, 1.0]`.
pub fn corrected_hsl_to_rgb(h: f64, s: f64, l: f64) -> Rgb {
    let corrector = HSL_CORRECTORS[(h / 60.0) as usize % 6];
    let l = if l < 0.5 {
        l * corrector * 2.0
    } else {
        ((l - 0.5) * (1.0 - corrector)).mul_add(2.0, corrector)
    };

    hsl_to_rgb(h, s, l)
}
