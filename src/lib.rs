#![allow(
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

mod config;
mod hsl;

pub use config::*;
use hsl::corrected_hsl_to_rgb;
use ril::prelude::*;
pub use ril::{self, ImageFormat};

/// Colors used by an identicon.
struct ColorCandidates {
    light_gray: Rgba,
    dark_gray: Rgba,
    light_color: Rgba,
    mid_color: Rgba,
    dark_color: Rgba,
}

impl ColorCandidates {
    #[inline]
    const fn get_from_rotation_index(&self, index: usize) -> Rgba {
        match index {
            0 => self.dark_gray,
            1 => self.mid_color,
            2 => self.light_gray,
            3 => self.light_color,
            _ /* 4 */ => self.dark_color,
        }
    }
}

impl Config {
    /// Retrieves a hue allowed by the configured hues.
    pub(crate) fn resolve_hue(&self, hue: f64) -> f64 {
        if self.hues.is_empty() {
            hue
        } else {
            self.hues[(hue / 360.0 * self.hues.len() as f64) as usize]
        }
    }

    /// Retrieves a color lightness that conforms to the configured lightness range. The lightness
    /// is expected to be in the range `[0.0, 1.0]`.
    #[inline]
    pub(crate) fn resolve_color_lightness(&self, lightness: f64) -> f64 {
        (self.color_lightness.end() - self.color_lightness.start())
            .mul_add(lightness, *self.color_lightness.start())
    }

    /// Retrieves a grayscale lightness that conforms to the configured lightness range. The
    /// lightness is expected to be in the range `[0.0, 1.0]`.
    #[inline]
    pub(crate) fn resolve_grayscale_lightness(&self, lightness: f64) -> f64 {
        (self.grayscale_lightness.end() - self.grayscale_lightness.start())
            .mul_add(lightness, *self.grayscale_lightness.start())
    }

    /// Retrieves a set of color candidates that conform to this configuration.
    pub(crate) fn color_candidates(&self, hue: f64) -> ColorCandidates {
        let hue = self.resolve_hue(hue);

        macro_rules! resolve {
            ($s:ident, $l_meth:ident, $l_value:literal) => {{
                corrected_hsl_to_rgb(hue, self.$s, self.$l_meth($l_value)).into_rgba()
            }};
            (@grayscale $l_value:literal) => {{
                resolve!(grayscale_saturation, resolve_grayscale_lightness, $l_value)
            }};
            (@color $l_value:literal) => {{
                resolve!(color_saturation, resolve_color_lightness, $l_value)
            }};
        }

        ColorCandidates {
            light_gray: resolve!(@grayscale 1.0),
            dark_gray: resolve!(@grayscale 0.0),
            light_color: resolve!(@color 1.0),
            mid_color: resolve!(@color 0.5),
            dark_color: resolve!(@color 0.0),
        }
    }
}

// (x, y, size, rotation)
#[derive(Copy, Clone, Default)]
struct Transform {
    x: u32,
    y: u32,
    pub rotation: u8,
    right: u32,
    bottom: u32,
}

impl Transform {
    pub(crate) const fn new(x: u32, y: u32, size: u32, rotation: u8) -> Self {
        Self {
            x,
            y,
            rotation,
            right: x + size,
            bottom: y + size,
        }
    }

    pub(crate) const fn transform(&self, (x, y): (u32, u32), (w, h): (u32, u32)) -> (u32, u32) {
        match self.rotation {
            0 => (self.x + x, self.y + y),
            1 => (self.right - y - h, self.y + x),
            2 => (self.right - x - w, self.bottom - y - h),
            _ /* 3 */ => (self.x + y, self.bottom - x - w),
        }
    }
}

struct ShapeRenderer<'a> {
    image: &'a mut Image<Rgba>,
    pub current_transform: Transform,
}

impl<'a> ShapeRenderer<'a> {
    pub fn new(image: &'a mut Image<Rgba>) -> Self {
        Self {
            image,
            current_transform: Transform::default(),
        }
    }

    pub fn polygon(
        &mut self,
        color: Rgba,
        points: impl IntoIterator<Item = (u32, u32)>,
    ) -> &mut Self {
        let polygon = Polygon::from_vertices(
            points
                .into_iter()
                .map(|pos| self.current_transform.transform(pos, (0, 0))),
        )
        .with_fill(color);

        self.image.draw(&polygon);
        self
    }

    pub fn circle(&mut self, color: Rgba, top_left: (u32, u32), diameter: u32) -> &mut Self {
        let (x, y) = self
            .current_transform
            .transform(top_left, (diameter, diameter));
        let circle = Ellipse::from_bounding_box(x, y, x + diameter, y + diameter).with_fill(color);

        self.image.draw(&circle);
        self
    }

    // top left is top left of the bounding box
    // this creates a right triangle
    pub fn triangle<const ROTATION: usize>(
        &mut self,
        color: Rgba,
        (x, y): (u32, u32),
        (w, h): (u32, u32),
    ) -> &mut Self {
        let (a, b, c, d) = ((x + w, y), (x + w, y + h), (x, y + h), (x, y));
        let points = match ROTATION % 4 {
            0 => [b, c, d],
            1 => [a, c, d],
            2 => [a, b, d],
            3 => [a, b, c],
            // SAFETY: `rotation % 4` on an unsigned int is always in the range `[0, 3]`.
            _ => unsafe { std::hint::unreachable_unchecked() },
        };

        self.polygon(color, points);
        self
    }

    pub fn rectangle(
        &mut self,
        color: Rgba,
        top_left: (u32, u32),
        mut size: (u32, u32),
    ) -> &mut Self {
        let (x, y) = self.current_transform.transform(top_left, size);
        if self.current_transform.rotation & 1 == 1 {
            std::mem::swap(&mut size.0, &mut size.1);
        }

        let rect = Rectangle::new()
            .with_position(x, y)
            .with_size(size.0, size.1 + 1)
            .with_fill(color);

        self.image.draw(&rect);
        self
    }

    // top left is top left of the bounding box
    pub fn rhombus(&mut self, color: Rgba, top_left: (u32, u32), size: (u32, u32)) -> &mut Self {
        self.polygon(
            color,
            [
                (top_left.0 + size.0 / 2, top_left.1),
                (top_left.0 + size.0, top_left.1 + size.1 / 2),
                (top_left.0 + size.0 / 2, top_left.1 + size.1),
                (top_left.0, top_left.1 + size.1 / 2),
            ],
        )
    }
}

#[inline]
fn pad_zeroes<const FROM: usize, const TO: usize>(arr: [u8; FROM]) -> [u8; TO] {
    let mut b = [0; TO];
    b[TO - FROM..].copy_from_slice(&arr);
    b
}

fn into_nibbles(hash: [u8; 20]) -> [u8; 40] {
    let mut nibbles = [0; 40];
    for i in 0..20 {
        nibbles[i * 2] = hash[i] >> 4;
        nibbles[i * 2 + 1] = hash[i] & 0x0f;
    }
    nibbles
}

#[inline]
fn hash_substring_u32<const LEN: usize>(nibbles: &[u8; 40], start: usize) -> u32 {
    let nibbles = pad_zeroes::<LEN, 8>(unsafe {
        // SAFETY: The hash is always 20 bytes long
        nibbles[start..start + LEN].try_into().unwrap_unchecked()
    });
    // Join nibbles back into bytes
    let mut bytes = [0; 4];
    for i in 0..4 {
        bytes[i] = nibbles[i * 2] << 4 | nibbles[i * 2 + 1];
    }

    u32::from_be_bytes(bytes)
}

#[allow(clippy::too_many_arguments)]
fn render_shape(
    hash: &[u8; 40],
    shape_index: usize,
    rotation_index: Option<usize>,
    renderer: &mut ShapeRenderer,
    color: Rgba,
    background_color: Rgba,
    cell_offset: u32,
    cell_size: u32,
    render_fn: impl Fn(&mut ShapeRenderer, Rgba, Rgba, u32, u8, usize),
    render_positions: impl IntoIterator<Item = (u32, u32)>,
) {
    let mut rotation = rotation_index.map(|idx| hash[idx]).unwrap_or_default();
    let shape_index = hash[shape_index];

    render_positions
        .into_iter()
        .enumerate()
        .for_each(|(i, (x, y))| {
            renderer.current_transform = Transform::new(
                cell_offset + x * cell_size,
                cell_offset + y * cell_size,
                cell_size,
                rotation % 4,
            );
            rotation += 1;

            render_fn(renderer, color, background_color, cell_size, shape_index, i);
        });
}

fn render_outer(
    renderer: &mut ShapeRenderer,
    color: Rgba,
    _background_color: Rgba,
    cell_size: u32,
    shape_index: u8,
    _position_index: usize,
) {
    match shape_index % 4 {
        0 => renderer.triangle::<0>(color, (0, 0), (cell_size, cell_size)),
        1 => renderer.triangle::<0>(color, (0, cell_size / 2), (cell_size, cell_size / 2)),
        2 => renderer.rhombus(color, (0, 0), (cell_size, cell_size)),
        _ /* 3 */ => {
            let m = cell_size / 6;
            renderer.circle(color, (m, m), cell_size - 2 * m)
        },
    };
}

#[allow(clippy::too_many_lines)]
fn render_center(
    renderer: &mut ShapeRenderer,
    color: Rgba,
    background_color: Rgba,
    cell_size: u32,
    shape_index: u8,
    position_index: usize,
) {
    match shape_index % 14 {
        0 => {
            let k = (cell_size as f64 * 0.42) as u32;
            renderer.polygon(
                color,
                [
                    (0, 0),
                    (cell_size, 0),
                    (cell_size, cell_size - k * 2),
                    (cell_size - k, cell_size),
                    (0, cell_size),
                ],
            );
        }
        1 => {
            let w = cell_size / 2;
            let h = (cell_size as f64 * 0.8) as u32;

            renderer.triangle::<2>(color, (cell_size - w, 0), (w, h));
        }
        2 => {
            let w = cell_size / 3;
            let dw = cell_size - w;

            renderer.rectangle(color, (w, w), (dw, dw));
        }
        3 => {
            let inner = cell_size as f64 / 10.0;
            // "Use fixed outer border widths in small icons to ensure the border is drawn"
            // https://github.com/dmester/jdenticon/blob/master/src/renderer/shapes.js#L41
            let outer = if cell_size < 6 {
                1
            } else if cell_size < 8 {
                2
            } else {
                cell_size / 4
            };

            let inner = if inner > 1.0 { inner as u32 } else { 1 };
            let p = cell_size - inner - outer;

            renderer.rectangle(color, (outer, outer), (p, p));
        }
        4 => {
            let m = (cell_size as f64 * 0.15) as u32;
            let w = cell_size / 2;
            let p = cell_size - w - m;

            renderer.circle(color, (p, p), w);
        }
        5 => {
            let inner = cell_size / 10;
            let outer = (cell_size as f64 * 0.4) as u32;

            renderer
                .rectangle(color, (0, 0), (cell_size, cell_size))
                .polygon(
                    background_color,
                    [
                        (outer, outer),
                        (cell_size - inner, outer),
                        (outer + (cell_size - outer - inner) / 2, cell_size - inner),
                    ],
                );
        }
        6 => {
            let tenth = cell_size / 10;
            let four_tenths = tenth * 4;
            let seven_tenths = tenth * 7;

            renderer.polygon(
                color,
                [
                    (0, 0),
                    (cell_size, 0),
                    (cell_size, seven_tenths),
                    (four_tenths, four_tenths),
                    (seven_tenths, cell_size),
                    (0, cell_size),
                ],
            );
        }
        7 | 11 => {
            let half_cell = cell_size / 2;
            let diff = cell_size - half_cell;
            renderer.triangle::<3>(color, (half_cell, half_cell), (diff, diff));
        }
        8 => {
            let half_cell = cell_size / 2;
            let diff = cell_size - half_cell;

            renderer
                .rectangle(color, (0, 0), (cell_size, diff))
                .rectangle(color, (0, half_cell), (diff, diff))
                .triangle::<1>(color, (half_cell, half_cell), (diff, diff));
        }
        9 => {
            let inner = (cell_size as f64 * 0.14) as u32;
            let outer = if cell_size < 4 {
                1
            } else if cell_size < 6 {
                2
            } else {
                (cell_size as f64 * 0.35) as u32
            };

            let p = cell_size - outer - inner;
            renderer
                .rectangle(color, (0, 0), (cell_size, cell_size))
                .rectangle(background_color, (outer, outer), (p, p));
        }
        10 => {
            let inner = cell_size as f64 * 0.12;
            let outer = (inner * 3.0) as u32;
            let inner = inner as u32;

            renderer
                .rectangle(color, (0, 0), (cell_size, cell_size))
                .circle(background_color, (outer, outer), cell_size - inner - outer);
        }
        12 => {
            let m = cell_size / 4;
            let p = cell_size - m;

            renderer
                .rectangle(color, (0, 0), (cell_size, cell_size))
                .rectangle(background_color, (m, m), (p, p));
        }
        13 if position_index == 0 => {
            let fcell = cell_size as f64;
            let m = (fcell * 0.4) as u32;
            let w = (fcell * 1.2) as u32;

            renderer.circle(color, (m, m), w);
        }
        _ => (),
    }
}

/// Renders an identicon for the given hash. The hash is strictly 20-bytes long. If your hash is
/// shorter, you should pad it. Similarly, if your hash is longer, you should truncate it.
///
/// # Returns
/// A ril [`Image`] with the identicon rendered on it. See [`Image::save_inferred`] to save the
/// image to a file, and similarly [`Image::encode`] to encode the image to a buffer in memory.
///
/// Saving identicons to different encodings require different features to be enabled. By default,
/// rdenticon enables the `ril/png` feature. If, for example, I wanted to save identicons as JPEGs,
/// I would enable the `ril/jpeg` feature. See the [`ril`] crate for more information on features.
pub fn render_identicon(hash: [u8; 20], config: &Config) -> Image<Rgba> {
    const SIDE_POSITIONS: [(u32, u32); 8] = [
        (1, 0),
        (2, 0),
        (2, 3),
        (1, 3),
        (0, 1),
        (3, 1),
        (3, 2),
        (0, 2),
    ];
    const CORNER_POSITIONS: [(u32, u32); 4] = [(0, 0), (3, 0), (3, 3), (0, 3)];
    const CENTER_POSITIONS: [(u32, u32); 4] = [(1, 1), (2, 1), (2, 2), (1, 2)];

    let mut image = Image::new(config.size, config.size, config.background_color);

    let padding = (config.padding * config.size as f64).round() as u32;
    let size = config.size - padding * 2;

    let cell = size / 4;
    let offset = padding + size / 2 - cell * 2;

    let hash = into_nibbles(hash);
    let hue = 360.0 * hash_substring_u32::<7>(&hash, 33) as f64 / 0xfffffff as f64;
    let color_candidates = config.color_candidates(hue);

    let mut selected_indices = [!0; 3];
    // `.contains` optimization
    macro_rules! contains_opt {
        ($value:literal) => {{
            selected_indices[0] == $value
                || selected_indices[1] == $value
                || selected_indices[2] == $value
        }};
    }

    for i in 0..3 {
        let index = hash[i + 8] % 5;
        let index = match index {
            0 | 4 if contains_opt!(0) || contains_opt!(4) => 1,
            2 | 3 if contains_opt!(2) || contains_opt!(3) => 1,
            _ => index,
        };

        selected_indices[i] = index;
    }

    let [side_color, corner_color, center_color] = selected_indices;
    let (side_color, corner_color, center_color) = (
        color_candidates.get_from_rotation_index(side_color as usize),
        color_candidates.get_from_rotation_index(corner_color as usize),
        color_candidates.get_from_rotation_index(center_color as usize),
    );

    let mut renderer = ShapeRenderer::new(&mut image);
    macro_rules! render {
        (
            $shape_index:literal,
            $rotation_index:expr,
            $color:ident,
            $render_fn:ident,
            $render_positions:ident
        ) => {
            render_shape(
                &hash,
                $shape_index,
                $rotation_index,
                &mut renderer,
                $color,
                config.background_color,
                offset,
                cell,
                $render_fn,
                $render_positions,
            );
        };
    }

    render!(2, Some(3), side_color, render_outer, SIDE_POSITIONS);
    render!(4, Some(5), corner_color, render_outer, CORNER_POSITIONS);
    render!(1, None, center_color, render_center, CENTER_POSITIONS);

    image
}

/// Generates an identicon for the given message. The message can be something like a username or a
/// unique key.
///
/// # Note
/// Identicons are hashed with SHA-1, which is not cryptographically secure. If you need a secure
/// hash (or if you simply do not want to use SHA-1), generate a hash of 20 bytes with a separate
/// algorithm and pass those bytes manually to [`render_identicon`].
///
/// # Returns
/// A ril [`Image`] with the identicon rendered on it. See [`Image::save_inferred`] to save the
/// image to a file, and similarly [`Image::encode`] to encode the image to a buffer in memory.
///
/// Saving identicons to different encodings require different features to be enabled. By default,
/// rdenticon enables the `ril/png` feature. If, for example, I wanted to save identicons as JPEGs,
/// I would enable the `ril/jpeg` feature. See the [`ril`] crate for more information on features.
///
/// # Example
/// ```no_run
/// fn main() -> rdenticon::ril::Result<()> {
///     // Build configuration
///     let config = rdenticon::Config::builder()
///         .size(512) // Generate a 512x512 image
///         .padding(0.1) // Add a 10% padding
///         .background_color(rdenticon::Rgba::transparent()) // Make the background transparent
///         .build()
///         .expect("invalid config");
///
///     // Render the identicon
///     let image = rdenticon::generate_identicon("super-cool-username", &config);
///
///     // Save the identicon to a file
///     image.save_inferred("identicon.png")?;
///
///     // OR: Save the identicon to memory
///     let mut out = Vec::new();
///     image.encode(rdenticon::ImageFormat::Png, &mut out)?;
///
///     Ok(())
/// }
/// ```
pub fn generate_identicon(message: impl AsRef<str>, config: &Config) -> Image<Rgba> {
    let hash = sha1_smol::Sha1::from(message.as_ref()).digest().bytes();
    render_identicon(hash, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rdenticon() -> ril::Result<()> {
        // Build configuration
        let config = Config::builder()
            .size(512) // Generate a 512x512 image
            .padding(0.1) // Add a 10% padding
            .background_color(Rgba::white())
            .build()
            .expect("invalid config");

        let image = generate_identicon("sample", &config);
        image.save_inferred("identicon.png")
    }
}
