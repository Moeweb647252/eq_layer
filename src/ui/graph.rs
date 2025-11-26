use std::f32::consts::PI;

use eframe::egui::{Color32, Response, Ui};
use egui_plot::{GridMark, Line, Plot, PlotPoints};

use crate::{
    eq::{Filter, FilterType},
    ui::App,
};

struct BiquadCoeffs {
    b0: f64,
    b1: f64,
    b2: f64,
    a0: f64,
    a1: f64,
    a2: f64,
}

impl BiquadCoeffs {
    pub fn calc(band: &Filter, fs: f64) -> BiquadCoeffs {
        let w0 = 2.0 * PI as f64 * band.frequency / fs;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * band.q_factor);

        // 增益转线性幅度 A = 10^(dB/40)
        let a = 10.0_f64.powf(band.gain / 40.0);

        let (b0, b1, b2, a0, a1, a2) = match band.filter_type {
            FilterType::Peak => (
                1.0 + alpha * a,
                -2.0 * cos_w0,
                1.0 - alpha * a,
                1.0 + alpha / a,
                -2.0 * cos_w0,
                1.0 - alpha / a,
            ),
            FilterType::LowShelf => {
                let sqrt_a = a.sqrt();
                (
                    a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha),
                    2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0),
                    a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha),
                    (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha,
                    -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0),
                    (a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha,
                )
            }
            FilterType::HighShelf => {
                let sqrt_a = a.sqrt();
                (
                    a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha),
                    -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0),
                    a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha),
                    (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha,
                    2.0 * ((a - 1.0) - (a + 1.0) * cos_w0),
                    (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha,
                )
            }
            _ => unimplemented!(),
        };

        BiquadCoeffs {
            b0,
            b1,
            b2,
            a0,
            a1,
            a2,
        }
    }

    pub fn calc_magnitude_db(&self, freq: f64, fs: f64) -> f64 {
        let w = 2.0 * PI as f64 * freq / fs;
        let cw = w.cos();
        let c2w = (2.0 * w).cos();
        let sw = w.sin();
        let s2w = (2.0 * w).sin();

        // H(z) = (b0 + b1*z^-1 + b2*z^-2) / (a0 + a1*z^-1 + a2*z^-2)
        // 实际上是复数除法，这里分别计算分子分母的实部和虚部

        let num_re = self.b0 + self.b1 * cw + self.b2 * c2w;
        let num_im = -(self.b1 * sw + self.b2 * s2w);

        let den_re = self.a0 + self.a1 * cw + self.a2 * c2w;
        let den_im = -(self.a1 * sw + self.a2 * s2w);

        let mag_sq = (num_re * num_re + num_im * num_im) / (den_re * den_re + den_im * den_im);

        10.0 * mag_sq.log10() // 20 * log10(mag) = 10 * log10(mag^2)
    }
}

fn log_grid_spacer(input: egui_plot::GridInput) -> Vec<egui_plot::GridMark> {
    let mut marks = Vec::new();

    let (min, max) = input.bounds;
    let min = min.max(10.0); // 音频通常不看 DC

    let min_log = min.log10().floor() as i32;
    let max_log = max.log10().ceil() as i32;

    for power in min_log..=max_log {
        let base = 10.0_f64.powi(power);

        // 主要刻度: 10, 100, 1k, 10k
        if base >= min && base <= max {
            marks.push(GridMark {
                value: base,
                step_size: base, // 仅用于内部计算逻辑
            });
        }

        // 次要刻度: 20, 30... 200, 300...
        // 如果缩放足够大，显示次要刻度
        if input.bounds.1 - input.bounds.0 < base * 10.0 {
            for k in 2..10 {
                let val = base * k as f64;
                if val >= min && val <= max {
                    // 次要刻度没有标签，或者用更淡的颜色，这里简单处理
                    // egui_plot 目前对 GridMark 的自定义有限，这里只是生成位置
                    marks.push(GridMark {
                        value: val,
                        step_size: base,
                    });
                }
            }
        }
    }
    marks
}

impl App {
    pub fn graph_ui(&self, ui: &mut Ui) -> Response {
        let fs = 44000.0;
        let coeffs: Vec<_> = self
            .eq_settings
            .eq_profile
            .filters
            .iter()
            .map(|f| BiquadCoeffs::calc(f, fs))
            .collect();
        let width = ui.available_width();
        let point_count = width as usize * 2;
        let log_min = 20.0f64.ln();
        let log_max = 20000.0f64.ln();
        let log_range = log_max - log_min;

        let curve_points: PlotPoints = (0..=point_count)
            .map(|i| {
                let t = i as f64 / point_count as f64;
                // 将线性索引 t 映射到对数频率域
                let freq = (log_min + t * log_range).exp();

                // 总响应是所有滤波器 dB 值的累加
                let mut total_db = 0.0;
                for coeffs in &coeffs {
                    total_db += coeffs.calc_magnitude_db(freq, fs);
                }
                [freq, total_db]
            })
            .collect();

        let plot = Plot::new("Graph");
        plot.x_axis_label("Frequency (Hz)")
            .y_axis_label("Gain (dB)")
            .allow_drag(false)
            .allow_scroll(false)
            .allow_zoom(false)
            .x_grid_spacer(log_grid_spacer)
            .default_x_bounds(20.0, 20000.0)
            .show(ui, |ui| {
                ui.line(
                    Line::new("Line", curve_points)
                        .width(2.0)
                        .color(Color32::LIGHT_BLUE),
                );
            })
            .response
    }
}
