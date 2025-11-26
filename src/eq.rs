use biquad::{Biquad, Coefficients, DirectForm2Transposed, ToHertz};
use serde::{Deserialize, Serialize};

use std::num::ParseFloatError;
use std::str::FromStr;

/// 定义滤波器类型 (Equalizer APO 标准缩写)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FilterType {
    Peak,      // PK
    LowShelf,  // LSC
    HighShelf, // HSC
    LowPass,   // LP
    HighPass,  // HP
    BandPass,  // BP
    Notch,     // NO
    AllPass,   // AP
    Unknown(String),
}

impl ToString for FilterType {
    fn to_string(&self) -> String {
        match self {
            Self::Peak => "Peak",
            Self::HighShelf => "HighShelf",
            Self::LowShelf => "LowShelf",
            _ => "Not Supported",
        }
        .to_string()
    }
}

impl FromStr for FilterType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "PK" | "PEAK" => Ok(FilterType::Peak),
            "LSC" | "LOWSHELF" => Ok(FilterType::LowShelf),
            "HSC" | "HIGHSHELF" => Ok(FilterType::HighShelf),
            "LP" | "LOWPASS" => Ok(FilterType::LowPass),
            "HP" | "HIGHPASS" => Ok(FilterType::HighPass),
            "BP" => Ok(FilterType::BandPass),
            "NO" | "NOTCH" => Ok(FilterType::Notch),
            "AP" => Ok(FilterType::AllPass),
            _ => Ok(FilterType::Unknown(s.to_string())),
        }
    }
}

/// 单个滤波器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub enabled: bool,
    pub filter_type: FilterType,
    pub frequency: f64,         // Hz
    pub gain: f64,              // dB
    pub q_factor: f64,          // Q 值
    pub bandwidth: Option<f64>, // Bandwidth (Q 的替代表示)
}

impl Default for Filter {
    fn default() -> Self {
        Self {
            enabled: true,
            filter_type: FilterType::Peak,
            frequency: 1000.0,
            gain: 0.0,
            q_factor: 0.707,
            bandwidth: None,
        }
    }
}

/// 完整的 EQ 配置文件结构
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct EqProfile {
    pub preamp_db: f64,
    pub filters: Vec<Filter>,
}

/// 解析错误类型
#[derive(Debug)]
pub enum EqParseError {
    ParseFloatError,
}

impl From<ParseFloatError> for EqParseError {
    fn from(_: ParseFloatError) -> Self {
        EqParseError::ParseFloatError
    }
}

impl FromStr for EqProfile {
    type Err = EqParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut profile = EqProfile::default();

        for line in s.lines() {
            let line = line.trim();

            // 忽略空行和注释
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // 解析 Preamp
            if line.to_uppercase().starts_with("PREAMP:") {
                // 格式示例: "Preamp: -6.0 dB"
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    // 尝试解析紧跟在 Preamp: 后的数字
                    // 有些文件写 "Preamp:" 有些写 "Preamp"
                    let val_str = if parts[0].ends_with(':') {
                        parts[1]
                    } else {
                        parts[2]
                    };
                    profile.preamp_db = val_str.parse()?;
                }
                continue;
            }

            // 解析 Filter
            if line.to_uppercase().starts_with("FILTER") {
                let filter = parse_filter_line(line)?;
                profile.filters.push(filter);
                continue;
            }
        }

        Ok(profile)
    }
}

/// 辅助函数：解析单行 Filter 字符串
fn parse_filter_line(line: &str) -> Result<Filter, EqParseError> {
    // 示例: Filter 1: ON PK Fc 100 Hz Gain 3.0 dB Q 1.41
    let tokens: Vec<&str> = line.split_whitespace().collect();

    let mut filter = Filter::default();

    // 简单的状态机遍历 token
    let mut i = 0;
    while i < tokens.len() {
        let token = tokens[i];
        match token.to_uppercase().as_str() {
            "ON" => filter.enabled = true,
            "OFF" => filter.enabled = false,
            "FC" => {
                if i + 1 < tokens.len() {
                    filter.frequency = tokens[i + 1].parse()?;
                    i += 1;
                }
            }
            "GAIN" => {
                if i + 1 < tokens.len() {
                    filter.gain = tokens[i + 1].parse()?;
                    i += 1;
                }
            }
            "Q" => {
                if i + 1 < tokens.len() {
                    filter.q_factor = tokens[i + 1].parse()?;
                    i += 1;
                }
            }
            "BW" => {
                if i + 1 < tokens.len() {
                    filter.bandwidth = Some(tokens[i + 1].parse()?);
                    i += 1;
                }
            }
            _ => {
                // 尝试解析 Filter 类型 (PK, LS, HS 等)
                // 通常类型出现在 "ON/OFF" 之后，或者 "Filter N:" 之后
                if let Ok(ft) = FilterType::from_str(token) {
                    if let FilterType::Unknown(_) = ft {
                        // 忽略未知 token (如 "Hz", "dB", "Filter", "1:")
                    } else {
                        filter.filter_type = ft;
                    }
                }
            }
        }
        i += 1;
    }

    Ok(filter)
}

pub struct Equalizer {
    pub config: EqProfile,
    pub filters: Vec<DirectForm2Transposed<f32>>,
}

unsafe impl Send for Equalizer {}
unsafe impl Sync for Equalizer {}

impl Equalizer {
    pub fn new(config: EqProfile, sample_rate: u32) -> Self {
        let mut filters = Vec::with_capacity(config.filters.len());
        for band in &config.filters {
            if !band.enabled {
                continue;
            }
            let biquad: Coefficients<f32> = match band.filter_type {
                FilterType::Peak => {
                    let q = band.q_factor as f32;
                    Coefficients::from_params(
                        biquad::Type::PeakingEQ(band.gain as f32),
                        sample_rate.hz(),
                        band.frequency.hz(),
                        q,
                    )
                    .unwrap()
                }
                FilterType::LowShelf => {
                    let q = band.q_factor as f32;
                    Coefficients::from_params(
                        biquad::Type::LowShelf(band.gain as f32),
                        sample_rate.hz(),
                        band.frequency.hz(),
                        q,
                    )
                    .unwrap()
                }
                FilterType::HighShelf => {
                    let q = band.q_factor as f32;
                    Coefficients::from_params(
                        biquad::Type::HighShelf(band.gain as f32),
                        sample_rate.hz(),
                        band.frequency.hz(),
                        q,
                    )
                    .unwrap()
                }
                FilterType::AllPass => {
                    let q = band.q_factor as f32;
                    Coefficients::from_params(
                        biquad::Type::AllPass,
                        sample_rate.hz(),
                        band.frequency.hz(),
                        q,
                    )
                    .unwrap()
                }
                _ => continue, // 其他类型暂不支持
            };
            filters.push(DirectForm2Transposed::<f32>::new(biquad));
        }
        Self { config, filters }
    }

    #[inline]
    pub fn process_sample(&mut self, sample: f32) -> f32 {
        let mut processed_sample = sample * db_to_linear(self.config.preamp_db as f32);
        for filter in &mut self.filters {
            processed_sample = filter.run(processed_sample);
        }
        processed_sample
    }
}

#[inline]
fn db_to_linear(preamp_db: f32) -> f32 {
    10f32.powf(preamp_db / 20.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eq_config_parser() {
        let config_str = "
Preamp: 0.0 dB
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
        assert_eq!(profile.filters[0].filter_type, FilterType::Peak);
        assert_eq!(profile.filters[0].frequency, 21.0);
        assert_eq!(profile.filters[0].gain, 2.3);
        assert_eq!(profile.filters[0].q_factor, 3.5);
        println!("Profile:{:?}", profile);
    }
}
