pub use ril::{Rgb, Rgba};
use std::{
    fmt,
    ops::{Range, RangeFrom, RangeInclusive, RangeTo, RangeToInclusive},
};

/// Configuration variables for rendering identicons.
///
/// For checked inputs and to otherwise avoid panics at runtime, it is advised you use
/// [`Config::builder`] to construct a [`Config`].
#[derive(Clone, Debug)]
pub struct Config {
    /// Limits the amount of hues in the identicon to only those specified in this `Vec`. All hues
    /// should be specified in degrees in the range `[0.0, 360.0)`.
    ///
    /// If an empty `Vec` is provided, all hues are allowed.
    pub hues: Vec<f64>,
    /// Specifies the lightness range of colored shapes in the identicon. This should be a sub-range
    /// of `0.0..=1.0`. Defaults to `0.4..=0.8`.
    pub color_lightness: RangeInclusive<f64>,
    /// Specifies the lightness range of grayscale shapes in the identicon. This should be a
    /// sub-range of `0.0..=1.0`. Defaults to `0.3..=0.9`.
    pub grayscale_lightness: RangeInclusive<f64>,
    /// Specifies the saturation range of colored shapes in the identicon, between 0 and 1.
    pub color_saturation: f64,
    /// Specifies the saturation range of grayscale shapes in the identicon, between 0 and 1.
    pub grayscale_saturation: f64,
    /// The background color to be rendered behind the identicon. Defaults to [`Rgba::white`].
    pub background_color: Rgba,
    /// The padding surrounding the icon relative to the size of the icon. This should be within
    /// the range `[0.0, 0.5]`. Defaults to `0.08`.
    pub padding: f64,
    /// The size of the icon in pixels. Defaults to `256`.
    pub size: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hues: Vec::new(),
            color_lightness: 0.4..=0.8,
            grayscale_lightness: 0.3..=0.9,
            color_saturation: 0.5,
            grayscale_saturation: 0.0,
            background_color: Rgba::white(),
            padding: 0.08,
            size: 256,
        }
    }
}

impl Config {
    /// Creates a new [`ConfigBuilder`] to construct a [`Config`].
    #[must_use = "ConfigBuilder does nothing on its own"]
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder {
            config: Config::default(),
        }
    }
}

/// A builder for [`Config`]s.
pub struct ConfigBuilder {
    config: Config,
}

/// A trait implemented by ranges that can be normalized to inclusive ranges within the range
/// `0.0..=1.0`.
pub trait NormalizableRange {
    fn normalize(self) -> RangeInclusive<f64>;
}

impl NormalizableRange for RangeInclusive<f64> {
    fn normalize(self) -> RangeInclusive<f64> {
        let start = *self.start();
        let end = *self.end();
        debug_assert!(start < end, "start must be less than end");
        debug_assert!(start >= 0.0 && end <= 1.0, "range must be within 0.0..=1.0");

        self
    }
}

impl NormalizableRange for Range<f64> {
    fn normalize(self) -> RangeInclusive<f64> {
        (self.start..=self.end + f64::EPSILON).normalize()
    }
}

impl NormalizableRange for RangeFrom<f64> {
    fn normalize(self) -> RangeInclusive<f64> {
        (self.start..=1.0).normalize()
    }
}

impl NormalizableRange for RangeTo<f64> {
    fn normalize(self) -> RangeInclusive<f64> {
        (0.0..self.end).normalize()
    }
}

impl NormalizableRange for RangeToInclusive<f64> {
    fn normalize(self) -> RangeInclusive<f64> {
        (0.0..=self.end).normalize()
    }
}

impl ConfigBuilder {
    /// Sets the hues to be used in the identicon. All hues should be specified in degrees in the
    /// range `[0.0, 360.0)`.
    #[must_use = "This method does not modify in place"]
    pub fn hues(mut self, hues: impl AsRef<[f64]>) -> Self {
        self.config.hues = hues.as_ref().to_vec();
        self
    }

    /// Sets the lightness range of colored shapes in the identicon. This should be a sub-range of
    /// `0.0..=1.0`.
    #[must_use = "This method does not modify in place"]
    pub fn color_lightness(mut self, lightness: impl NormalizableRange) -> Self {
        self.config.color_lightness = lightness.normalize();
        self
    }

    /// Sets the lightness range of grayscale shapes in the identicon. This should be a sub-range of
    /// `0.0..=1.0`.
    #[must_use = "This method does not modify in place"]
    pub fn grayscale_lightness(mut self, lightness: impl NormalizableRange) -> Self {
        self.config.grayscale_lightness = lightness.normalize();
        self
    }

    /// Sets the saturation range of colored shapes in the identicon, between 0 and 1.
    /// Defaults to `0.5`.
    #[must_use = "This method does not modify in place"]
    pub const fn color_saturation(mut self, saturation: f64) -> Self {
        self.config.color_saturation = saturation;
        self
    }

    /// Sets the saturation range of grayscale shapes in the identicon, between 0 and 1.
    /// Defaults to `0.0`.
    #[must_use = "This method does not modify in place"]
    pub const fn grayscale_saturation(mut self, saturation: f64) -> Self {
        self.config.grayscale_saturation = saturation;
        self
    }

    /// Sets the background color to be rendered behind the identicon.
    /// Defaults to [`Rgba::white`].
    #[must_use = "This method does not modify in place"]
    pub const fn background_color(mut self, color: Rgba) -> Self {
        self.config.background_color = color;
        self
    }

    /// Sets the padding surrounding the icon relative to the size of the icon.
    /// This should be within the range `[0.0, 0.5]`. Defaults to `0.08`.
    #[must_use = "This method does not modify in place"]
    pub const fn padding(mut self, padding: f64) -> Self {
        self.config.padding = padding;
        self
    }

    /// Sets the size of the icon in pixels. Defaults to `256`.
    #[must_use = "This method does not modify in place"]
    pub const fn size(mut self, size: u32) -> Self {
        self.config.size = size;
        self
    }

    /// Builds the [`Config`].
    ///
    /// # Errors
    /// * If hues are not within the range `[0.0, 360.0)`.
    /// * If color lightness is not within the range `0.0..=1.0`.
    /// * If grayscale lightness is not within the range `0.0..=1.0`.
    /// * If color saturation is not within the range `[0.0, 1.0]`.
    /// * If grayscale saturation is not within the range `[0.0, 1.0]`.
    /// * If padding is not within the range `[0.0, 0.5]`.
    pub fn build(self) -> Result<Config, ConfigBuilderError> {
        if self
            .config
            .hues
            .iter()
            .any(|hue| !(0.0..360.0).contains(hue))
        {
            return Err(ConfigBuilderError::InvalidHues);
        }
        if self.config.color_lightness.start() < &0.0 || self.config.color_lightness.end() > &1.0 {
            return Err(ConfigBuilderError::InvalidColorLightness);
        }
        if self.config.grayscale_lightness.start() < &0.0
            || self.config.grayscale_lightness.end() > &1.0
        {
            return Err(ConfigBuilderError::InvalidGrayscaleLightness);
        }
        if !(0.0..=1.0).contains(&self.config.color_saturation) {
            return Err(ConfigBuilderError::InvalidColorSaturation);
        }
        if !(0.0..=1.0).contains(&self.config.grayscale_saturation) {
            return Err(ConfigBuilderError::InvalidGrayscaleSaturation);
        }
        if !(0.0..=0.5).contains(&self.config.padding) {
            return Err(ConfigBuilderError::InvalidPadding);
        }

        Ok(self.config)
    }
}

/// An error that occurs when building a [`Config`].
/// See [`ConfigBuilder::build`] for more information.
#[derive(Clone, Debug)]
pub enum ConfigBuilderError {
    /// The hues are not within the range `[0.0, 360.0)`.
    InvalidHues,
    /// The color lightness is not within the range `0.0..=1.0`.
    InvalidColorLightness,
    /// The grayscale lightness is not within the range `0.0..=1.0`.
    InvalidGrayscaleLightness,
    /// The color saturation is not within the range `[0.0, 1.0]`.
    InvalidColorSaturation,
    /// The grayscale saturation is not within the range `[0.0, 1.0]`.
    InvalidGrayscaleSaturation,
    /// The padding is not within the range `[0.0, 0.5]`.
    InvalidPadding,
}

impl fmt::Display for ConfigBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHues => write!(f, "hues must be within the range [0.0, 360.0)"),
            Self::InvalidColorLightness => {
                write!(f, "color lightness must be within the range 0.0..=1.0")
            }
            Self::InvalidGrayscaleLightness => {
                write!(f, "grayscale lightness must be within the range 0.0..=1.0")
            }
            Self::InvalidColorSaturation => {
                write!(f, "color saturation must be within the range [0.0, 1.0]")
            }
            Self::InvalidGrayscaleSaturation => write!(
                f,
                "grayscale saturation must be within the range [0.0, 1.0]"
            ),
            Self::InvalidPadding => write!(f, "padding must be within the range [0.0, 0.5]"),
        }
    }
}
