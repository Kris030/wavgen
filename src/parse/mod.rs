use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

use self::{
    result::ParserResult as Res,
    source::{Source, StringSource},
    tokenizer::{Number, Token, TokenType as Ty, Tokenizer},
};
use crate::gen::{self, Channels, PeriodicSource, Song, SourceType};
use thiserror::Error as ThisError;

pub mod printing;
pub mod result;
pub mod source;
pub mod tokenizer;

#[derive(Debug, ThisError)]
pub enum ParserError<S> {
    #[error("No track name was provided")]
    MissingName,

    #[error("No track duration was provided")]
    MissingDuration,

    #[error("No track channel count was provided")]
    MissingChannels,

    #[error(transparent)]
    TokenizerError(#[from] tokenizer::TokenizerError<S>),

    #[error("Unexpected {0:?}")]
    Unexpected(Ty),

    #[error("Expected {expected:?}, found {found:?}")]
    UnexpectedExact { expected: Ty, found: Ty },

    #[error(transparent)]
    Expression(#[from] ExpressionError),
}
use ParserError as ParsErr;

pub fn get_song<'name, 'text>(
    source_name: &'name str,
    src: &'text str,
) -> Result<Song, ParsErr<<StringSource<'name, 'text> as Source>::Error>> {
    let mut diagnostics = vec![];

    let source = StringSource::new(source_name, src);
    let tokenizer = tokenizer::Tokenizer::new(source, &mut diagnostics);

    match Parser::new(tokenizer).parse_song() {
        Res::Some(t) => Ok(t),
        Res::Err(e) => Err(e),

        Res::Done => todo!(),
    }
}

pub struct Parser<'d, 's, S> {
    song_channels: usize,
    song_length_s: f64,

    tokenizer: Tokenizer<'d, 's, S>,
    buffer: Vec<Token<'s, S>>,
}

impl<'d, 's, S: Source> Parser<'d, 's, S> {
    pub fn new(tokenizer: Tokenizer<'d, 's, S>) -> Self {
        Self {
            song_channels: 0,
            song_length_s: f64::NAN,

            tokenizer,
            buffer: vec![],
        }
    }

    pub fn parse_song<'src, 'name: 'src>(mut self) -> Res<Song, ParsErr<S::Error>> {
        let mut sources = vec![];

        let name = match self.get_token()? {
            Token {
                ty: Ty::StringLiteral(name),
                ..
            } => name,

            _ => return Res::Err(ParsErr::MissingName),
        };

        // TODO: unwrap
        self.song_length_s = self
            .parse_expression(|t| {
                if let Some("s") = t.text() {
                    Terminate::Yes {
                        discard_token: true,
                    }
                } else {
                    Terminate::No
                }
            })?
            .evaluate(None)
            .unwrap();

        self.song_channels = match self.parse_chan()? {
            Channels::One(channels) => channels,

            _ => return Res::Err(ParsErr::MissingChannels),
        };

        while let Some(s) = self.parse_source().to_res_opt()? {
            sources.push(s);
        }

        Res::Some(Song {
            channels: self.song_channels,
            length_s: self.song_length_s,
            sources,
            name,
        })
    }

    fn get_token(&mut self) -> Res<Token<'s, S>, ParsErr<S::Error>> {
        if let Some(t) = self.buffer.pop() {
            return Res::Some(t);
        }

        match self.tokenizer.get_next() {
            Ok(Some(v)) => Res::Some(v),
            Ok(None) => Res::Done,

            Err(e) => Res::Err(ParsErr::TokenizerError(e)),
        }
    }

    fn eat(&mut self, expected: Ty) -> Res<Token<'s, S>, ParsErr<S::Error>> {
        let token = self.get_token()?;

        if token.ty == expected {
            Res::Some(token)
        } else {
            let found = token.ty.clone();

            self.buffer.push(token);

            Res::Err(ParsErr::UnexpectedExact { expected, found })
        }
    }

    fn _eat_f<F>(&mut self, f: F) -> Res<Token<'s, S>, ParsErr<S::Error>>
    where
        F: FnOnce(&Token<'s, S>) -> bool,
    {
        let token = self.get_token()?;

        if f(&token) {
            Res::Some(token)
        } else {
            let ty = token.ty.clone();
            self.buffer.push(token);
            Res::Err(ParsErr::Unexpected(ty))
        }
    }

    fn parse_source(&mut self) -> Res<gen::Source, ParsErr<S::Error>> {
        let wave_type_t = self.eat(Ty::Identifier)?;

        let wave_type = wave_type_t
            .position
            .get_text()
            .expect("Couldn't get identifier contents");

        self.eat(Ty::LeftParenthesis)?;

        let ty = match wave_type {
            "sin" | "sine" | "saw" | "tri" | "triangle" | "square" => {
                let freq = self.parse_expression(|t| {
                    if let Some("Hz" | "hz") = t.text() {
                        Terminate::Yes {
                            discard_token: true,
                        }
                    } else {
                        Terminate::No
                    }
                })?;

                self.eat(Ty::Comma)?;

                SourceType::Periodic {
                    freq,
                    phase: Expression::zero(),
                    ty: match wave_type {
                        "sin" => PeriodicSource::Sine,
                        "sine" => PeriodicSource::Sine,
                        "saw" => PeriodicSource::Saw,
                        "tri" | "triangle" => PeriodicSource::Triangle,
                        "square" => PeriodicSource::Square,

                        _ => unreachable!(),
                    },
                }
            }

            _ => return Res::Err(ParsErr::Unexpected(wave_type_t.ty)),
        };

        let (start, end) = self.parse_timeframe(self.song_length_s)?;

        self.eat(Ty::RightParenthesis)?;

        let channels = self.parse_chan()?;
        let volume = self.parse_vol()?;

        let mut effects = vec![];

        let len_s = (end - start) * self.song_length_s;
        if let Res::Some(_) = self.eat(Ty::LeftCurlyBraces) {
            while let Res::Some(e) = self.parse_effect(len_s) {
                effects.push(e);
            }

            self.eat(Ty::RightCurlyBraces)?;
        }

        Res::Some(gen::Source {
            start,
            end,

            channels,
            volume,

            effects,

            ty,
        })
    }

    fn parse_effect(&mut self, parent_len_s: f64) -> Res<gen::Effect, ParsErr<S::Error>> {
        let name_t = self.eat(Ty::Identifier)?;
        let name = name_t
            .position
            .get_text()
            .expect("Couldn't get identifier name");

        let (start, end) = self.parse_timeframe(parent_len_s)?;

        let ty = match name {
            "fade_in" => gen::EffectType::FadeIn,
            "fade_out" => gen::EffectType::FadeOut,

            _ => return Res::Err(ParsErr::Unexpected(name_t.ty)),
        };

        Res::Some(gen::Effect { ty, start, end })
    }

    fn parse_chan(&mut self) -> Res<Channels, ParsErr<S::Error>> {
        self.eat(Ty::OnKw)?;

        match self.get_token()?.ty {
            Ty::NumberLiteral(Number::Integer(i)) => Res::Some(Channels::One(i as usize)),

            Ty::Star => Res::Some(Channels::All),

            ty => Res::Err(ParsErr::Unexpected(ty)),
        }
    }

    fn parse_vol(&mut self) -> Res<Expression, ParsErr<S::Error>> {
        self.eat(Ty::AtSign)?;

        self.parse_expression(|_| Terminate::No)
    }

    fn parse_time_unit(&mut self) -> Res<f64, ParsErr<S::Error>> {
        let t = self.eat(Ty::Identifier)?;
        let txt = t.position.get_text().unwrap();

        Res::Some(match txt {
            "h" => 3600.,
            "m" => 60.,

            "s" => 1.,
            "ms" => 0.001,

            "ns" => 1e-9,

            _ => {
                let ty = t.ty.clone();
                self.buffer.push(t);
                return Res::Err(ParsErr::Unexpected(ty));
            }
        })
    }

    fn parse_timeframe(&mut self, parent_len_s: f64) -> Res<(f64, f64), ParsErr<S::Error>> {
        let t = self.get_token()?;

        let mut need_colon = true;
        let start = match t.ty {
            Ty::NumberLiteral(n) => {
                let mut n: f64 = n.into();

                if let Res::Some(u) = self.parse_time_unit() {
                    n = (n * u) / parent_len_s;
                }

                n
            }

            Ty::Colon => {
                need_colon = false;
                0.
            }

            ty => return Res::Err(ParsErr::Unexpected(ty)),
        };

        if need_colon {
            self.eat(Ty::Colon)?;
        }

        let end = match self.get_token().to_res_opt()? {
            Some(Token {
                ty: Ty::NumberLiteral(n),
                ..
            }) => {
                let mut n: f64 = n.into();

                if let Res::Some(u) = self.parse_time_unit() {
                    n = (n * u) / parent_len_s;
                }

                n
            }

            t => {
                if let Some(t) = t {
                    self.buffer.push(t)
                }

                1.
            }
        };

        Res::Some((start, end))
    }

    fn parse_expression<F>(&mut self, mut terminate: F) -> Res<Expression, ParsErr<S::Error>>
    where
        F: FnMut(&Token<'s, S>) -> Terminate,
    {
        let mut output_queue = vec![];
        let mut ops = vec![];

        let prec = |t: &Token<'s, S>| match t.ty {
            Ty::LeftParenthesis => 0,
            Ty::Plus | Ty::Minus => 1,
            Ty::Slash | Ty::Star | Ty::Percent => 2,
            Ty::Caret => 3,

            _ => unreachable!(),
        };

        #[derive(Clone, Copy, PartialEq, Eq)]
        enum Assoc {
            Left,
            Right,
            NonAssoc,
        }
        let assoc = |t: &Token<'s, S>| match t.ty {
            Ty::LeftParenthesis | Ty::RightParenthesis => Assoc::NonAssoc,
            Ty::Caret => Assoc::Right,

            Ty::Slash | Ty::Star | Ty::Percent | Ty::Plus | Ty::Minus => Assoc::Left,

            _ => panic!("operator assoc called on non operator."),
        };

        // shunting yard algorithm
        // from https://en.wikipedia.org/wiki/Shunting_yard_algorithm#The_algorithm_in_detail

        // while there are tokens to be read:
        //     read a token
        while let Res::Some(t) = self.get_token() {
            if let Terminate::Yes { discard_token } = terminate(&t) {
                if !discard_token {
                    self.buffer.push(t);
                }
                break;
            }

            match t.ty {
                // if the token is:
                // - a number:
                //     put it into the output queue
                Ty::NumberLiteral(_) => output_queue.push(t),

                // - a function:
                //  push it onto the operator stack
                Ty::Identifier => {
                    if MathFunc::is_func(t.position.get_text().unwrap()) {
                        ops.push(t);
                    } else {
                        output_queue.push(t);
                    }
                }

                // - an operator o1:
                Ty::Plus | Ty::Minus | Ty::Star | Ty::Slash | Ty::Caret | Ty::Percent => {
                    // while (
                    //     there is an operator o2 at the top of the operator stack which is not a left parenthesis,
                    //     and (o2 has greater precedence than o1 or (o1 and o2 have the same precedence and o1 is left-associative))
                    // ):
                    while 'w: {
                        let Some(o2) = ops.last() else {
                            break 'w false;
                        };

                        if o2.ty != Ty::LeftParenthesis {
                            break 'w false;
                        }

                        (prec(o2) > prec(&t)) || (prec(&t) == prec(o2) && assoc(&t) == Assoc::Left)
                    } {
                        // pop o2 from the operator stack into the output queue
                        output_queue.push(ops.pop().unwrap());
                    }

                    // push o1 onto the operator stack
                    ops.push(t);
                }

                // - a ",":
                Ty::Comma => {
                    // while the operator at the top of the operator stack is not a left parenthesis:
                    while {
                        if let Some(o2) = ops.last() {
                            o2.ty != Ty::LeftParenthesis
                        } else {
                            false
                        }
                    } {
                        // pop the operator from the operator stack into the output queue
                        output_queue.push(ops.pop().unwrap());
                    }
                }

                // - a left parenthesis (i.e. "("):
                // push it onto the operator stack
                Ty::LeftParenthesis => ops.push(t),

                // - a right parenthesis (i.e. ")"):
                Ty::RightParenthesis => {
                    // while the operator at the top of the operator stack is not a left parenthesis:
                    while 'w: {
                        let Some(o2) = ops.last() else {
                            break 'w false;
                        };

                        o2.ty != Ty::LeftParenthesis
                    } {
                        // {assert the operator stack is not empty}
                        // /* If the stack runs out without finding a left parenthesis, then there are mismatched parentheses. */
                        if ops.is_empty() {
                            todo!("mismatched parentheses")
                        }

                        // pop the operator from the operator stack into the output queue
                        output_queue.push(ops.pop().unwrap());
                    }

                    // {assert there is a left parenthesis at the top of the operator stack}
                    // pop the left parenthesis from the operator stack and discard it
                    if !matches!(ops.pop().map(|t| t.ty), Some(Ty::LeftParenthesis)) {
                        todo!()
                    }

                    // if there is a function token at the top of the operator stack, then:
                    if !ops.is_empty()
                        && ops.last().unwrap().ty == Ty::Identifier
                        && MathFunc::is_func(t.position.get_text().unwrap())
                    {
                        // pop the function from the operator stack into the output queue
                        output_queue.push(ops.pop().unwrap());
                    }
                }

                _ => {
                    self.buffer.push(t);
                    break;
                }
            }
        }

        // /* After the while loop, pop the remaining items from the operator stack into the output queue. */
        // while there are tokens on the operator stack:
        while let Some(t) = ops.pop() {
            // /* If the operator token on the top of the stack is a parenthesis, then there are mismatched parentheses. */
            //     {assert the operator on top of the stack is not a (left) parenthesis}
            //     pop the operator from the operator stack onto the output queue
            if t.ty == Ty::LeftParenthesis {
                todo!("mismatched parenthesis")
            }

            output_queue.push(t);
        }

        // println!(
        //     "{:?}",
        //     output_queue.iter().map(|t| &t.ty).collect::<Vec<_>>()
        // );

        let expr = Expression::construct(&mut output_queue);

        self.buffer.append(&mut output_queue);

        Res::Some(expr)
    }
}

fn _match_identifier<'name, 's, S: Source>(
    name: &'name str,
) -> impl 'name + FnOnce(&Token<'s, S>) -> bool {
    move |t| match t.ty {
        Ty::Identifier => match t.position.get_text() {
            Some(t) => t == name,
            None => false,
        },

        _ => false,
    }
}

#[derive(Debug, Clone)]
pub enum Expression {
    Add(Box<Expression>, Box<Expression>),
    Sub(Box<Expression>, Box<Expression>),

    Mul(Box<Expression>, Box<Expression>),
    Div(Box<Expression>, Box<Expression>),

    Pow(Box<Expression>, Box<Expression>),
    Mod(Box<Expression>, Box<Expression>),

    Call(MathFunc, Box<Expression>),

    VarOrConst(String),
    Lit(Number),
}

#[derive(Debug, ThisError)]
pub enum ExpressionError {
    #[error("Unknown variable {0}")]
    UnknownVar(String),

    #[error("No GenInfo")]
    NoGenInfo,
}

impl Expression {
    pub fn evaluate(&self, gi: Option<gen::GenInfo>) -> Result<f64, ExpressionError> {
        Ok(match self {
            Self::Add(b, a) => a.evaluate(gi)? + b.evaluate(gi)?,
            Self::Sub(b, a) => a.evaluate(gi)? - b.evaluate(gi)?,

            Self::Mul(b, a) => a.evaluate(gi)? * b.evaluate(gi)?,
            Self::Div(b, a) => a.evaluate(gi)? / b.evaluate(gi)?,

            Self::Mod(b, a) => a.evaluate(gi)? % b.evaluate(gi)?,

            Self::Pow(b, a) => a.evaluate(gi)?.powf(b.evaluate(gi)?),

            Self::Call(f, arg) => f.call(arg.evaluate(gi)?),

            Self::VarOrConst(name) => match &name[..] {
                "pi" | "π" => std::f64::consts::PI,
                "e" => std::f64::consts::E,

                "channel" | "ch" => gi.ok_or(ExpressionError::NoGenInfo)?.channel as f64,
                "t" => gi.ok_or(ExpressionError::NoGenInfo)?.t,

                v => return Err(ExpressionError::UnknownVar(v.to_string())),
            },

            Self::Lit(a) => (*a).into(),
        })
    }

    fn construct<'s, S: Source + 's>(iter: &mut Vec<Token<'s, S>>) -> Self {
        let t = iter.pop().unwrap();

        match t.ty {
            Ty::NumberLiteral(n) => Self::Lit(n),

            Ty::Identifier => {
                let s = t.position.get_text().unwrap();
                if let Ok(f) = MathFunc::from_str(s) {
                    Self::Call(f, Box::new(Self::construct(iter)))
                } else {
                    Self::VarOrConst(s.to_string())
                }
            }

            Ty::Plus => Self::Add(
                Box::new(Self::construct(iter)),
                Box::new(Self::construct(iter)),
            ),
            Ty::Minus => Self::Sub(
                Box::new(Self::construct(iter)),
                Box::new(Self::construct(iter)),
            ),
            Ty::Star => Self::Mul(
                Box::new(Self::construct(iter)),
                Box::new(Self::construct(iter)),
            ),
            Ty::Slash => Self::Sub(
                Box::new(Self::construct(iter)),
                Box::new(Self::construct(iter)),
            ),
            Ty::Caret => Self::Pow(
                Box::new(Self::construct(iter)),
                Box::new(Self::construct(iter)),
            ),
            Ty::Percent => Self::Mod(
                Box::new(Self::construct(iter)),
                Box::new(Self::construct(iter)),
            ),

            Ty::Comma => todo!(),

            Ty::RightParenthesis | Ty::LeftParenthesis => Self::construct(iter),

            _ => unreachable!(),
        }
    }

    pub fn zero() -> Expression {
        Expression::Lit(Number::Real(0.))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathFunc {
    Sin,
    Cos,
    Tan,
    Ln,
    Log10,
    Log2,
    Sqrt,
    Abs,
    Round,
    Floor,
    Ceil,
    Rad,
    Deg,
}

impl MathFunc {
    pub fn call(&self, x: f64) -> f64 {
        match self {
            Self::Sin => x.sin(),
            Self::Cos => x.cos(),
            Self::Tan => x.tan(),
            Self::Ln => x.ln(),
            Self::Log10 => x.log10(),
            Self::Log2 => x.log2(),
            Self::Sqrt => x.sqrt(),
            Self::Abs => x.abs(),
            Self::Round => x.round(),
            Self::Floor => x.floor(),
            Self::Ceil => x.ceil(),
            Self::Rad => x.to_radians(),
            Self::Deg => x.to_degrees(),
        }
    }

    pub fn is_func(s: &str) -> bool {
        Self::from_str(s).is_ok()
    }
}

impl FromStr for MathFunc {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "sin" => MathFunc::Sin,
            "cos" => MathFunc::Cos,
            "tan" => MathFunc::Tan,
            "ln" => MathFunc::Ln,
            "lg" | "log10" => MathFunc::Log10,
            "log2" => MathFunc::Log2,
            "sqrt" => MathFunc::Sqrt,
            "abs" => MathFunc::Abs,
            "round" => MathFunc::Round,
            "floor" => MathFunc::Floor,
            "ceil" => MathFunc::Ceil,
            "rad" => MathFunc::Rad,
            "deg" => MathFunc::Deg,

            _ => return Err(()),
        })
    }
}

impl Display for MathFunc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MathFunc::Sin => write!(f, "sin"),
            MathFunc::Cos => write!(f, "cos"),
            MathFunc::Tan => write!(f, "tan"),
            MathFunc::Ln => write!(f, "ln"),
            MathFunc::Log10 => write!(f, "log10"),
            MathFunc::Log2 => write!(f, "log2"),
            MathFunc::Sqrt => write!(f, "sqrt"),
            MathFunc::Abs => write!(f, "abs"),
            MathFunc::Round => write!(f, "round"),
            MathFunc::Floor => write!(f, "floor"),
            MathFunc::Ceil => write!(f, "ceil"),
            MathFunc::Rad => write!(f, "rad"),
            MathFunc::Deg => write!(f, "deg"),
        }
    }
}

impl Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expression::Add(a, b) => write!(f, "{a} + {b}"),
            Expression::Sub(a, b) => write!(f, "{a} - {b}"),
            Expression::Mul(a, b) => write!(f, "{a} * {b}"),
            Expression::Div(a, b) => write!(f, "{a} / {b}"),
            Expression::Pow(a, b) => write!(f, "{a}^{b}"),
            Expression::Mod(a, b) => write!(f, "{a} % {b}"),
            Expression::Call(a, b) => write!(f, "{a}({b})"),
            Expression::VarOrConst(a) => write!(f, "{a}"),
            Expression::Lit(a) => write!(f, "{a}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Terminate {
    Yes { discard_token: bool },
    No,
}
