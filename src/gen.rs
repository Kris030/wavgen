use std::{
    f64::consts::{PI, TAU},
    fmt::Display,
};

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

pub fn print_song(s: &Song) {
    println!("'{}': {}s, {} channels", s.name, s.length(), s.channels);
    for s in &s.sources {
        print!("  ");
        match &s.ty {
            SourceType::Periodic { freq, phase, ty } => {
                print!("{freq} Hz (phase: {phase}) {ty}",);
            }
        }

        println!(
            " {}:{}, volume: {}, channels: {}",
            s.start, s.end, s.volume, s.channels
        );

        for e in &s.effects {
            print!("    ");
            match e.ty {
                EffectType::FadeIn => print!("fade in"),
                EffectType::FadeOut => print!("fade out"),
            }
            println!(" {}:{}", e.start, e.end);
        }
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

impl Display for Channels {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Channels::List(l) => write!(f, "{l:?}"),
            Channels::One(c) => write!(f, "{c}"),
            Channels::All => write!(f, "all"),
        }
    }
}

#[derive(Debug)]
pub enum SourceType {
    Periodic {
        freq: Expression,
        phase: Expression,
        ty: PeriodicSource,
    },
}

#[derive(Debug)]
pub enum PeriodicSource {
    Sine,
    Saw,
    Square,
    Triangle,
}

impl Display for PeriodicSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PeriodicSource::Sine => write!(f, "sine"),
            PeriodicSource::Saw => write!(f, "saw"),
            PeriodicSource::Square => write!(f, "square"),
            PeriodicSource::Triangle => write!(f, "triangle"),
        }
    }
}

impl SourceType {
    pub fn gen(&mut self, gi: GenInfo) -> Result<f64, ExpressionError> {
        let t = gi.t;
        let gi = Some(gi);

        Ok(match self {
            Self::Periodic { freq, phase, ty } => {
                let phase = phase.evaluate(gi)?;
                let freq = freq.evaluate(gi)?;

                match ty {
                    PeriodicSource::Sine => sine(t, freq, phase),
                    PeriodicSource::Saw => saw(t, freq, phase),
                    PeriodicSource::Square => square(t, freq, phase),
                    PeriodicSource::Triangle => triangle(t, freq, phase),
                }
            }
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
