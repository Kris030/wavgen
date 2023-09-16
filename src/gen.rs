use std::{
    f64::consts::{PI, TAU},
    time::Duration,
};

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
    let mixed: f64 = mix_sources(get_sources(gi).into_iter(), gi);
    let scaled = (mixed * i16::MAX as f64) as i16;

    scaled.clamp(i16::MIN, i16::MAX)
}

pub fn get_sources(gi: GenerationInfo) -> impl IntoIterator<Item = f64> {
    let t = gi.t;

    let freq1 = (f64::sin(t * 2.0 * PI) + 1.) * 30. + 30.;

    [
        sine(t, freq1, 0.5),        //
        harmonic(3, t, freq1, 0.5), //
        triangle(t, freq1, 0.),
        triangle(t, 60., 0.),
    ]
}

pub fn mix_sources(sources: impl Iterator<Item = f64>, gi: GenerationInfo) -> f64 {
    let mut count: usize = 0;
    let mut sum = 0.;

    for s in sources {
        sum += s;
        count += 1;
    }

    let v = (sum / count as f64) * 0.8;

    fade_out(
        fade_in(v, Duration::from_millis(100), gi),
        Duration::from_millis(100),
        gi,
    )
}

pub fn sine(t: f64, freq: f64, phase: f64) -> f64 {
    f64::sin(t * freq * TAU + phase)
}
pub fn saw(t: f64, freq: f64, phase: f64) -> f64 {
    f64::fract(t * freq + phase) * 2. - 1.
}
pub fn square(t: f64, freq: f64, phase: f64) -> f64 {
    let x = t + phase;

    let freq = 1. / freq;
    if x % freq < freq / 2. {
        1.
    } else {
        -1.
    }
}
pub fn triangle(t: f64, freq: f64, phase: f64) -> f64 {
    ((f64::fract(t * freq + phase) * 2. - 1.).abs() - 0.5) * 2.
}
pub fn fade_in(v: f64, dur: Duration, gi: GenerationInfo) -> f64 {
    let dur = dur.as_secs_f64();

    if gi.t >= dur {
        return v;
    }

    v * (gi.t / dur)
}
pub fn fade_out(v: f64, dur: Duration, gi: GenerationInfo) -> f64 {
    let dur = dur.as_secs_f64();

    let left = gi.total_time.unwrap() - gi.t;
    if left >= dur {
        return v;
    }

    v * (left / dur)
}

pub fn lerp(t: f64, s: f64, e: f64) -> f64 {
    s * (1. - t) + e * t
}
pub fn sin_interpolate(t: f64, s: f64, e: f64) -> f64 {
    if t <= 0. {
        0.
    } else if t >= 1. {
        1.
    } else {
        let t = f64::sin(t * std::f64::consts::FRAC_PI_2);
        s * (1. - t) + e * t
    }
}

pub fn harmonic(nth: usize, t: f64, freq: f64, phase: f64) -> f64 {
    sine(t, freq * nth as f64, phase)
}

pub fn overtone(nth: usize, t: f64, freq: f64, phase: f64) -> f64 {
    harmonic(nth, t, freq, phase + PI)
}

pub fn harmonics<const H: usize>(t: f64, freq: f64, phase: f64) -> [f64; H] {
    std::array::from_fn(|i| harmonic(i, t, freq, phase))
}

pub fn overtones<const H: usize>(t: f64, freq: f64, phase: f64) -> [f64; H] {
    harmonics(t, freq, phase + PI)
}
