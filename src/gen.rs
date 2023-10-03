use std::f64::consts::{PI, TAU};

use crate::parse::{Expression, ExpressionError};

#[derive(Debug)]
pub struct Song {
    pub name: String,

    pub(crate) channels: usize,
    pub(crate) length_s: f64,

    pub(crate) sources: Vec<Source>,
}

impl Song {
    pub fn length(&self) -> f64 {
        self.length_s
    }
    pub fn channels(&self) -> usize {
        self.channels
    }
}

#[derive(Debug)]
pub struct Source {
    pub(crate) ty: SourceType,
    pub(crate) start: f64,
    pub(crate) end: f64,
    pub(crate) volume: Expression,
    pub(crate) channels: Channels,

    pub(crate) effects: Vec<Effect>,
}

#[derive(Debug)]
pub enum Channels {
    List(Vec<usize>),
    One(usize),
    All,
}

impl Channels {
    pub fn has(&self, c: usize) -> bool {
        match self {
            Self::List(l) => l.contains(&c),
            Self::One(ch) => c == *ch,
            Self::All => true,
        }
    }
}

#[derive(Debug)]
pub enum SourceType {
    Sine { freq: Expression, phase: Expression },
    Saw { freq: Expression, phase: Expression },
    Square { freq: Expression, phase: Expression },
    Triangle { freq: Expression, phase: Expression },
}

impl SourceType {
    pub fn gen(&mut self, gi: GenInfo) -> Result<f64, ExpressionError> {
        let t = gi.t;
        let gi = Some(gi);

        Ok(match self {
            Self::Sine { freq, phase } => sine(t, freq.evaluate(gi)?, phase.evaluate(gi)?),
            Self::Saw { freq, phase } => saw(t, freq.evaluate(gi)?, phase.evaluate(gi)?),
            Self::Square { freq, phase } => square(t, freq.evaluate(gi)?, phase.evaluate(gi)?),
            Self::Triangle { freq, phase } => triangle(t, freq.evaluate(gi)?, phase.evaluate(gi)?),
        })
    }
}

#[derive(Debug)]
pub enum EffectType {
    FadeIn,
    FadeOut,
}

impl EffectType {
    pub fn apply(&mut self, v: f64, gi: GenInfo) -> f64 {
        match *self {
            Self::FadeIn => v * gi.t,
            Self::FadeOut => v * (1. - gi.t),
        }
    }
}

#[derive(Debug)]
pub struct Effect {
    pub(crate) ty: EffectType,
    pub(crate) start: f64,
    pub(crate) end: f64,
}

impl Effect {
    pub fn apply(&mut self, v: f64, gi: GenInfo) -> f64 {
        self.ty.apply(v, gi)
    }
}

impl Source {
    pub fn gen(&mut self, gi: GenInfo) -> Result<f64, ExpressionError> {
        let mut v = self.ty.gen(gi)?;

        for e in &mut self.effects {
            if (e.start..=e.end).contains(&gi.t) {
                let gi_e = GenInfo::new(gi, e.start, e.end);
                v = e.apply(v, gi_e);
            }
        }

        Ok(v * self.volume.evaluate(Some(gi))?)
    }

    pub fn length(&self) -> f64 {
        self.end - self.start
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GenInfo {
    pub(crate) channel: usize,
    pub(crate) t: f64,
}

impl GenInfo {
    pub fn new(parent: GenInfo, start: f64, end: f64) -> Self {
        Self {
            channel: parent.channel,
            t: (parent.t - start) / (end - start),
        }
    }
}

pub fn get_sample(s: &mut Song, gi: GenInfo) -> Result<f64, ExpressionError> {
    let mut mixed = 0.;

    for src in &mut s.sources {
        if !src.channels.has(gi.channel) || !(src.start..=src.end).contains(&gi.t) {
            continue;
        }

        let gi = GenInfo::new(gi, src.start, src.end);
        let v = src.gen(gi)?;

        mixed = mix(mixed, v);
    }

    Ok(mixed)
}

pub fn mix(v1: f64, v2: f64) -> f64 {
    v1 + v2
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

pub fn harmonic(nth: usize, t: f64, freq: f64, phase: f64) -> f64 {
    sine(t, freq * nth as f64, phase)
}

pub fn overtone(nth: usize, t: f64, freq: f64, phase: f64) -> f64 {
    harmonic(nth, t, freq, phase + PI)
}

pub fn harmonics(n: usize, t: f64, freq: f64, phase: f64) -> Vec<f64> {
    (0..n).map(|i| harmonic(i, t, freq, phase)).collect()
}

pub fn overtones(n: usize, t: f64, freq: f64, phase: f64) -> Vec<f64> {
    harmonics(n, t, freq, phase + PI)
}
