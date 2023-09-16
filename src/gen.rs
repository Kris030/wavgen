#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GenerationInfo {
    channel: usize,
    t: f64,

    total_time: Option<f64>,
    total_channels: usize,
}

impl GenerationInfo {
    pub fn new(channel: usize, t: f64, total_time: Option<f64>, total_channels: usize) -> Self {
        Self {
            channel,
            t,
            total_time,
            total_channels,
        }
    }
}

pub fn get_sample(gi: GenerationInfo) -> i16 {
    let mixed: f64 = mix_sources(&get_sources(gi));
    let scaled = (mixed * i16::MAX as f64) as i16;

    scaled.clamp(i16::MIN, i16::MAX)
}

pub fn get_sources(gi: GenerationInfo) -> [f64; 4] {
    let freq = 10.;
    let x = gi.t * std::f64::consts::TAU * freq;

    let f = f64::sin(x);

    let g = f64::cos(0.5 * x + std::f64::consts::FRAC_PI_2) / 2.;

    let h = f64::sin(3. * x);

    let n = (f64::sin(10. * x) + f64::sin(20. * x) + f64::sin(30. * x)) / 10.;

    [f, g, h, n]
}

pub fn mix_sources(sources: &[f64]) -> f64 {
    sources.iter().sum::<f64>() / (5f64).sqrt()
}
