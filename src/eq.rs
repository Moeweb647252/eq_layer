use serde::{Deserialize, Serialize};

use std::num::ParseFloatError;
use std::str::FromStr;

/// Equalizer APO FilterType
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FilterType {
    Peaking,   // PK
    LowShelf,  // LSC
    HighShelf, // HSC
    LowPass,   // LP
    HighPass,  // HP
}

impl std::fmt::Display for FilterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Peaking => "Peak",
            Self::HighShelf => "HighShelf",
            Self::LowShelf => "LowShelf",
            Self::LowPass => "LowPass",
            Self::HighPass => "HighPass",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for FilterType {
    type Err = EqParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "PK" | "PEAK" => Ok(FilterType::Peaking),
            "LSC" | "LOWSHELF" => Ok(FilterType::LowShelf),
            "HSC" | "HIGHSHELF" => Ok(FilterType::HighShelf),
            "LP" | "LOWPASS" => Ok(FilterType::LowPass),
            "HP" | "HIGHPASS" => Ok(FilterType::HighPass),
            _ => Err(EqParseError::UnknownFilterType),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub enabled: bool,
    pub filter_type: FilterType,
    pub frequency: f64, // Hz
    pub gain: f64,      // dB
    pub q_factor: f64,
    pub bandwidth: Option<f64>,
}

impl Default for Filter {
    fn default() -> Self {
        Self {
            enabled: true,
            filter_type: FilterType::Peaking,
            frequency: 1000.0,
            gain: 0.0,
            q_factor: 0.707,
            bandwidth: None,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct EqProfile {
    pub preamp_db: f64,
    pub filters: Vec<Filter>,
}

#[derive(Debug)]
pub enum EqParseError {
    ParseFloatError,
    UnknownFilterType,
}

impl From<ParseFloatError> for EqParseError {
    fn from(_: ParseFloatError) -> Self {
        EqParseError::ParseFloatError
    }
}

impl Filter {
    pub(crate) fn effective_q(&self) -> f64 {
        if let Some(bw) = self.bandwidth {
            if bw > 0.0 { self.frequency / bw } else { self.q_factor }
        } else {
            self.q_factor
        }
    }
}

impl FromStr for EqProfile {
    type Err = EqParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut profile = EqProfile::default();

        for line in s.lines() {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if line.to_uppercase().starts_with("PREAMP:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let val_str = if parts[0].ends_with(':') {
                        parts[1]
                    } else {
                        parts[2]
                    };
                    profile.preamp_db = val_str.parse()?;
                }
                continue;
            }

            if line.to_uppercase().starts_with("FILTER") {
                let filter = parse_filter_line(line)?;
                profile.filters.push(filter);
                continue;
            }
        }

        Ok(profile)
    }
}

fn parse_filter_line(line: &str) -> Result<Filter, EqParseError> {
    let parts: Vec<&str> = line.splitn(2, ':').collect();
    if parts.len() < 2 {
        return Err(EqParseError::UnknownFilterType);
    }
    let token_str = parts[1].trim();
    let tokens: Vec<&str> = token_str.split_whitespace().collect();

    let mut filter = Filter::default();

    let mut i = 0;
    while i < tokens.len() {
        let token = tokens[i];
        match token.to_uppercase().as_str() {
            "ON" => filter.enabled = true,
            "OFF" => filter.enabled = false,
            "FC" if i + 1 < tokens.len() => {
                filter.frequency = tokens[i + 1].parse()?;
                i += 1;
            }
            "GAIN" if i + 1 < tokens.len() => {
                filter.gain = tokens[i + 1].parse()?;
                i += 1;
            }
            "Q" if i + 1 < tokens.len() => {
                filter.q_factor = tokens[i + 1].parse()?;
                i += 1;
            }
            "BW" if i + 1 < tokens.len() => {
                filter.bandwidth = Some(tokens[i + 1].parse()?);
                i += 1;
            }
            "HZ" | "DB" => {}
            _ => {
                filter.filter_type = FilterType::from_str(token)?;
            }
        }
        i += 1;
    }

    Ok(filter)
}

use std::f32::consts::PI;

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;

/// Holds the raw coefficients for the Biquad filter.
/// Normalized so a0 = 1.0.
#[derive(Debug, Clone, Copy)]
struct BiquadCoeffs {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl BiquadCoeffs {
    /// Calculates coefficients based on RBJ Audio EQ Cookbook formulas
    fn calculate(
        filter_type: FilterType,
        freq: f32,
        q: f32,
        gain_db: f32,
        sample_rate: f32,
    ) -> Self {
        let omega = 2.0 * PI * freq / sample_rate;
        let (sin_w, cos_w) = omega.sin_cos();
        let a = 10.0f32.powf(gain_db / 40.0);
        let alpha = match filter_type {
            FilterType::LowShelf | FilterType::HighShelf => {
                sin_w / 2.0 * ((a + 1.0 / a) * (1.0 / q - 1.0) + 2.0).sqrt()
            }
            _ => sin_w / (2.0 * q),
        };

        let (b0, b1, b2, a0, a1, a2) = match filter_type {
            FilterType::Peaking => (
                1.0 + alpha * a,
                -2.0 * cos_w,
                1.0 - alpha * a,
                1.0 + alpha / a,
                -2.0 * cos_w,
                1.0 - alpha / a,
            ),
            FilterType::LowShelf => {
                let a_plus_1 = a + 1.0;
                let a_minus_1 = a - 1.0;
                let beta = 2.0 * a.sqrt() * alpha;
                (
                    a * (a_plus_1 - a_minus_1 * cos_w + beta),
                    2.0 * a * (a_minus_1 - a_plus_1 * cos_w),
                    a * (a_plus_1 - a_minus_1 * cos_w - beta),
                    a_plus_1 + a_minus_1 * cos_w + beta,
                    -2.0 * (a_minus_1 + a_plus_1 * cos_w),
                    a_plus_1 + a_minus_1 * cos_w - beta,
                )
            }
            FilterType::HighShelf => {
                let a_plus_1 = a + 1.0;
                let a_minus_1 = a - 1.0;
                let beta = 2.0 * a.sqrt() * alpha;
                (
                    a * (a_plus_1 + a_minus_1 * cos_w + beta),
                    -2.0 * a * (a_minus_1 + a_plus_1 * cos_w),
                    a * (a_plus_1 + a_minus_1 * cos_w - beta),
                    a_plus_1 - a_minus_1 * cos_w + beta,
                    2.0 * (a_minus_1 - a_plus_1 * cos_w),
                    a_plus_1 - a_minus_1 * cos_w - beta,
                )
            }
            FilterType::LowPass => (
                (1.0 - cos_w) / 2.0,
                1.0 - cos_w,
                (1.0 - cos_w) / 2.0,
                1.0 + alpha,
                -2.0 * cos_w,
                1.0 - alpha,
            ),
            FilterType::HighPass => (
                (1.0 + cos_w) / 2.0,
                -(1.0 + cos_w),
                (1.0 + cos_w) / 2.0,
                1.0 + alpha,
                -2.0 * cos_w,
                1.0 - alpha,
            ),
        };

        // Normalize by a0
        let inv_a0 = 1.0 / a0;
        BiquadCoeffs {
            b0: b0 * inv_a0,
            b1: b1 * inv_a0,
            b2: b2 * inv_a0,
            a1: a1 * inv_a0,
            a2: a2 * inv_a0,
        }
    }
}

/// Platform-independent scalar Direct Form I biquad.
#[allow(dead_code)]
struct ScalarBiquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

#[allow(dead_code)]
impl ScalarBiquad {
    fn new(coeffs: BiquadCoeffs) -> Self {
        Self {
            b0: coeffs.b0,
            b1: coeffs.b1,
            b2: coeffs.b2,
            a1: coeffs.a1,
            a2: coeffs.a2,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    #[inline(always)]
    fn process(&mut self, input: f32) -> f32 {
        let output = self.b0 * input + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;
        output
    }
}

/// The SIMD optimized Biquad Filter Node.
/// Stores state for 4 parallel channels (Quad-channel processing).
#[cfg(target_arch = "aarch64")]
#[repr(align(16))]
struct SimdBiquad {
    // Coefficients loaded into SIMD vectors (splatted)
    b0: float32x4_t,
    b1: float32x4_t,
    b2: float32x4_t,
    a1: float32x4_t,
    a2: float32x4_t,

    // State memory (History)
    // x[n-1], x[n-2]
    x1: float32x4_t,
    x2: float32x4_t,
    // y[n-1], y[n-2]
    y1: float32x4_t,
    y2: float32x4_t,
}

impl SimdBiquad {
    #[cfg(target_arch = "aarch64")]
    fn new(coeffs: BiquadCoeffs) -> Self {
        unsafe {
            Self {
                b0: vdupq_n_f32(coeffs.b0),
                b1: vdupq_n_f32(coeffs.b1),
                b2: vdupq_n_f32(coeffs.b2),
                a1: vdupq_n_f32(coeffs.a1),
                a2: vdupq_n_f32(coeffs.a2),
                x1: vdupq_n_f32(0.0),
                x2: vdupq_n_f32(0.0),
                y1: vdupq_n_f32(0.0),
                y2: vdupq_n_f32(0.0),
            }
        }
    }

    /// Process a single "Quad-Sample" (4 channels at the same time step).
    /// Returns the filtered Quad-Sample.
    /// Direct Form I Difference Equation:
    /// y[n] = b0*x[n] + b1*x[n-1] + b2*x[n-2] - a1*y[n-1] - a2*y[n-2]
    #[inline(always)]
    #[cfg(target_arch = "aarch64")]
    unsafe fn process_quad(&mut self, input: float32x4_t) -> float32x4_t {
        let mut acc = unsafe { vmulq_f32(self.b0, input) };

        // acc += b1 * x[n-1]
        acc = unsafe { vfmaq_f32(acc, self.b1, self.x1) };

        // acc += b2 * x[n-2]
        acc = unsafe { vfmaq_f32(acc, self.b2, self.x2) };

        // acc -= a1 * y[n-1]
        acc = unsafe { vfmsq_f32(acc, self.a1, self.y1) };

        // acc -= a2 * y[n-2]
        acc = unsafe { vfmsq_f32(acc, self.a2, self.y2) };

        // Shift state
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = acc;

        acc
    }
}

/// The main Equalizer struct.
/// "Dynamic" means it can handle a variable number of bands.
pub struct ParametricEq {
    sample_rate: f32,
    preamp_db: f64,
    #[cfg(target_arch = "aarch64")]
    bands: Vec<SimdBiquad>,
    #[cfg(not(target_arch = "aarch64"))]
    bands: Vec<ScalarBiquad>,
}

impl ParametricEq {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            preamp_db: 0.0,
            bands: Vec::with_capacity(8),
        }
    }

    pub fn from_profile(profile: &EqProfile, sample_rate: f32) -> Self {
        let mut eq = Self::new(sample_rate);
        eq.preamp_db = profile.preamp_db;
        for band in profile.filters.iter().filter(|f| f.enabled) {
            eq.add_band(
                band.filter_type,
                band.frequency as f32,
                band.effective_q() as f32,
                band.gain as f32,
            );
        }
        eq
    }

    /// Add a new band to the chain
    pub fn add_band(&mut self, filter_type: FilterType, freq: f32, q: f32, gain_db: f32) {
        let coeffs = BiquadCoeffs::calculate(filter_type, freq, q, gain_db, self.sample_rate);
        #[cfg(target_arch = "aarch64")]
        {
            self.bands.push(SimdBiquad::new(coeffs));
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            self.bands.push(ScalarBiquad::new(coeffs));
        }
    }

    /// Process a buffer of interleaved audio.
    ///
    /// SAFETY: This function expects `data` to contain interleaved Quad-Channel audio.
    /// E.g., [L, R, Aux1, Aux2, L, R, Aux1, Aux2...]
    /// If you only have Stereo [L, R], padding to 4 channels is required to use this specific SIMD kernel,
    /// or you can process 2 stereo frames at once (De-interleaving required).
    ///
    /// The buffer length must be a multiple of 4.
    pub fn process_buffer(&mut self, data: &mut [f32]) {
        #[cfg(target_arch = "aarch64")]
        unsafe {
            let preamp_gain = 10.0f32.powf(self.preamp_db as f32 / 20.0);
            let gain_v = vdupq_n_f32(preamp_gain);
            let len = data.len();
            let chunks = len / 4;
            let ptr = data.as_mut_ptr();
            for i in 0..chunks {
                let mut current_quad = vmulq_f32(gain_v, vld1q_f32(ptr.add(i * 4)));
                for band in &mut self.bands {
                    current_quad = band.process_quad(current_quad);
                }
                vst1q_f32(ptr.add(i * 4), current_quad);
            }
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            let preamp_gain = 10.0f32.powf(self.preamp_db as f32 / 20.0);
            for sample in data.iter_mut() {
                let mut s = *sample * preamp_gain;
                for band in &mut self.bands {
                    s = band.process(s);
                }
                *sample = s;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eq_config_parser() {
        let config_str = "
Preamp: -1.0 dB
Filter 1: ON PK Fc 21 Hz Gain 2.3 dB Q 3.500
Filter 2: ON PK Fc 150 Hz Gain -3.4 dB Q 0.600
Filter 3: ON PK Fc 690 Hz Gain 2.2 dB Q 3.100
Filter 4: ON PK Fc 1400 Hz Gain -1.2 dB Q 3.900
Filter 5: ON PK Fc 1900 Hz Gain -3.2 dB Q 4.400
Filter 6: ON PK Fc 10200 Hz Gain 4.0 dB Q 0.300
Filter 7: ON PK Fc 18974 Hz Gain 4.0 dB Q 1.100
Filter 8: ON PK Fc 20000 Hz Gain 0.0 dB Q 0.710
";
        let profile: EqProfile = config_str.parse().unwrap();
        assert_eq!(profile.preamp_db, -1.0);
        assert_eq!(profile.filters.len(), 8);
        assert_eq!(profile.filters[0].filter_type, FilterType::Peaking);
        assert_eq!(profile.filters[0].frequency, 21.0);
        assert_eq!(profile.filters[0].gain, 2.3);
        assert_eq!(profile.filters[0].q_factor, 3.5);
        assert!((profile.filters[0].effective_q() - 3.5).abs() < 0.001);
        println!("Profile:{:?}", profile);
    }

    #[test]
    fn test_parser_no_units() {
        let config = "
Preamp: -2.0 dB
Filter 1: ON PK Fc 1000 Gain 3.0 Q 1.5
Filter 2: OFF LSC Fc 80 Gain -2.0 Q 0.71
";
        let profile: EqProfile = config.parse().unwrap();
        assert_eq!(profile.preamp_db, -2.0);
        assert_eq!(profile.filters.len(), 2);
        assert_eq!(profile.filters[0].frequency, 1000.0);
        assert_eq!(profile.filters[0].gain, 3.0);
        assert!(!profile.filters[1].enabled);
        assert_eq!(profile.filters[1].filter_type, FilterType::LowShelf);
    }
}
