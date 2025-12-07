use std::f32::consts::PI;

use eframe::egui::{Color32, Response, Ui};
use egui_plot::{GridInput, GridMark, Line, Plot, PlotPoints};

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
            FilterType::Peaking => (
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

fn audio_grid_spacer(input: GridInput) -> Vec<GridMark> {
    let mut marks = Vec::new();

    // input.bounds 包含了当前视图的最小值和最大值 (真实频率值，非 log 值)
    let (min, max) = input.bounds;

    // 计算 10 的幂次范围
    let min_log = min.log10().floor() as i32;
    let max_log = max.log10().ceil() as i32;

    for power in min_log..=max_log {
        let base = 10.0_f64.powi(power);

        // 我们通常希望显示 1, 2, 5 序列 (例如 100, 200, 500)
        // 或者如果缩放得够大，显示 1, 2, 3, 4...

        // 1. 主刻度 (10^n): 10, 100, 1k, 10k
        if base >= min && base <= max {
            marks.push(GridMark {
                value: base,
                step_size: base, // 这一项帮助 egui 决定是否显示 label
            });
        }

        // 2. 次级刻度: 2, 3, 4 ... 9 * base
        for k in 2..10 {
            let val = base * k as f64;
            if val > max {
                break;
            }
            if val < min {
                continue;
            }

            // 简单的细节剔除逻辑：
            // 只有当两个主刻度（比如 100 和 1000）在屏幕上的距离足够宽时，才显示中间的刻度
            // 这里我们用一个近似的启发式方法：
            // 如果视图范围涵盖了太多的数量级，就不显示次级刻度

            let range_magnitude = (max / min).log10();

            // 如果显示的范围跨度小于 4 个数量级 (比如 20Hz - 20kHz 是 3 个数量级)，显示详细网格
            if range_magnitude < 4.0 {
                // 音频常用: 2, 5 总是比较重要
                if k == 2 || k == 5 {
                    marks.push(GridMark {
                        value: val,
                        step_size: base,
                    });
                } else if range_magnitude < 2.5 {
                    // 只有放大得比较大时，才显示 3, 4, 6, 7, 8, 9
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
            .eq_profile
            .filters
            .iter()
            .filter(|f| f.enabled)
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
            .allow_axis_zoom_drag(false)
            .allow_boxed_zoom(false)
            .x_grid_spacer(audio_grid_spacer)
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
