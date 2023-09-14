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

pub fn get_sources(gi: GenerationInfo) -> [f64; 3] {
    let freq = 50.;

    [
        f64::sin(gi.t * std::f64::consts::TAU * freq),
        f64::sin(0.5 * gi.t * std::f64::consts::TAU * freq),
        f64::cos(2. * gi.t * std::f64::consts::TAU * freq),
    ]
}

pub fn mix_sources(sources: &[f64]) -> f64 {
    sources.iter().sum::<f64>() / (sources.len() as f64).sqrt()
}
