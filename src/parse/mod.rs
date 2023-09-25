use std::fmt::Debug;

use self::{
    result::ParserResult,
    source::{Source, StringSource},
    tokenizer::{Number, Token, TokenType, Tokenizer},
};
use crate::gen::{self, Channels, Song};
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
    Unexpected(TokenType),

    #[error("Expected {expected:?}, found {found:?}")]
    UnexpectedExact {
        expected: TokenType,
        found: TokenType,
    },
}

pub fn get_song<'name, 'text>(
    source_name: &'name str,
    src: &'text str,
) -> Result<Song, ParserError<<StringSource<'name, 'text> as Source>::Error>> {
    let mut diagnostics = vec![];

    let source = StringSource::new(source_name, src);
    let tokenizer = tokenizer::Tokenizer::new(source, &mut diagnostics);

    match Parser::new(tokenizer).parse_song() {
        ParserResult::Some(t) => Ok(t),
        ParserResult::Err(_) => unreachable!(),

        ParserResult::Done => todo!(),
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

    pub fn parse_song<'src, 'name: 'src>(mut self) -> ParserResult<Song, ParserError<S::Error>> {
        let mut sources = vec![];

        let Token {
            ty: TokenType::StringLiteral(name),
            ..
        } = self.get_token()?
        else {
            return ParserResult::Err(ParserError::MissingName);
        };

        self.song_length_s = match self.get_token()?.ty {
            TokenType::NumberLiteral(length) => length.into(),

            _ => return ParserResult::Err(ParserError::MissingDuration),
        };

        self.eat_f(match_identifier("s"))?;

        self.song_channels = match self.parse_chan()? {
            Channels::One(channels) => channels,

            _ => return ParserResult::Err(ParserError::MissingChannels),
        };

        while let ParserResult::Some(s) = self.parse_source() {
            sources.push(s);
        }

        ParserResult::Some(Song {
            channels: self.song_channels,
            length_s: self.song_length_s,
            sources,
            name,
        })
    }

    fn get_token(&mut self) -> ParserResult<Token<'s, S>, ParserError<S::Error>> {
        if let Some(t) = self.buffer.pop() {
            return ParserResult::Some(t);
        }

        match self.tokenizer.get_next() {
            Ok(Some(v)) => ParserResult::Some(v),
            Ok(None) => ParserResult::Done,

            Err(e) => ParserResult::Err(ParserError::TokenizerError(e)),
        }
    }

    fn eat(&mut self, expected: TokenType) -> ParserResult<Token<'s, S>, ParserError<S::Error>> {
        let token = self.get_token()?;

        if token.ty == expected {
            ParserResult::Some(token)
        } else {
            let found = token.ty.clone();

            self.buffer.push(token);

            ParserResult::Err(ParserError::UnexpectedExact { expected, found })
        }
    }

    fn eat_f<F>(&mut self, f: F) -> ParserResult<Token<'s, S>, ParserError<S::Error>>
    where
        F: FnOnce(&Token<'s, S>) -> bool,
    {
        let token = self.get_token()?;

        if f(&token) {
            ParserResult::Some(token)
        } else {
            let ty = token.ty.clone();
            self.buffer.push(token);
            ParserResult::Err(ParserError::Unexpected(ty))
        }
    }

    fn parse_source(&mut self) -> ParserResult<gen::Source, ParserError<S::Error>> {
        let wave_type_t = self.eat(TokenType::Identifier)?;

        let wave_type = wave_type_t
            .position
            .get_text()
            .expect("Couldn't get identifier contents");

        self.eat(TokenType::LeftParenthesis)?;

        let freq = match self.get_token()?.ty {
            TokenType::NumberLiteral(n) => n.into(),
            ty => return ParserResult::Err(ParserError::Unexpected(ty)),
        };

        self.eat_f(match_identifier("hz"))?;
        self.eat(TokenType::Comma)?;

        let (start, end) = self.parse_timeframe(self.song_length_s)?;

        self.eat(TokenType::RightParenthesis)?;

        let channels = self.parse_chan()?;
        let volume = self.parse_vol()?;

        let mut effects = vec![];

        let len_s = (end - start) * self.song_length_s;
        if let ParserResult::Some(_) = self.eat(TokenType::LeftCurlyBraces) {
            while let ParserResult::Some(e) = self.parse_effect(len_s) {
                effects.push(e);
            }

            self.eat(TokenType::RightCurlyBraces)?;
        }

        let ty = match wave_type {
            "sin" | "sine" => gen::SourceType::Sine { freq, phase: 0. },
            "saw" => gen::SourceType::Saw { freq, phase: 0. },
            "tri" | "triangle" => gen::SourceType::Triangle { freq, phase: 0. },
            "square" => gen::SourceType::Square { freq, phase: 0. },

            _ => return ParserResult::Err(ParserError::Unexpected(wave_type_t.ty)),
        };

        ParserResult::Some(gen::Source {
            start,
            end,

            channels,
            volume,

            effects,

            ty,
        })
    }

    fn parse_effect(
        &mut self,
        parent_len_s: f64,
    ) -> ParserResult<gen::Effect, ParserError<S::Error>> {
        let name_t = self.eat(TokenType::Identifier)?;
        let name = name_t
            .position
            .get_text()
            .expect("Couldn't get identifier name");

        let (start, end) = self.parse_timeframe(parent_len_s)?;

        let ty = match name {
            "fade_in" => gen::EffectType::FadeIn,
            "fade_out" => gen::EffectType::FadeOut,

            _ => return ParserResult::Err(ParserError::Unexpected(name_t.ty)),
        };

        ParserResult::Some(gen::Effect { ty, start, end })
    }

    fn parse_chan(&mut self) -> ParserResult<Channels, ParserError<S::Error>> {
        self.eat(TokenType::OnKw)?;

        match self.get_token()?.ty {
            TokenType::NumberLiteral(Number::Integer(i)) => {
                ParserResult::Some(Channels::One(i as usize))
            }

            TokenType::Star => ParserResult::Some(Channels::All),

            ty => ParserResult::Err(ParserError::Unexpected(ty)),
        }
    }

    fn parse_vol(&mut self) -> ParserResult<f64, ParserError<S::Error>> {
        self.eat(TokenType::AtSign)?;

        match self.get_token()?.ty {
            TokenType::NumberLiteral(n) => ParserResult::Some(n.into()),

            ty => ParserResult::Err(ParserError::Unexpected(ty)),
        }
    }

    fn parse_time_unit(&mut self) -> ParserResult<f64, ParserError<S::Error>> {
        let t = self.eat(TokenType::Identifier)?;
        let txt = t.position.get_text().unwrap();

        ParserResult::Some(match txt {
            "h" => 3600.,
            "m" => 60.,

            "s" => 1.,
            "ms" => 0.001,

            "ns" => 1e-9,

            _ => {
                let ty = t.ty.clone();
                self.buffer.push(t);
                return ParserResult::Err(ParserError::Unexpected(ty));
            }
        })
    }

    fn parse_timeframe(
        &mut self,
        parent_len_s: f64,
    ) -> ParserResult<(f64, f64), ParserError<S::Error>> {
        let t = self.get_token()?;

        let mut need_colon = true;
        let start = match t.ty {
            TokenType::NumberLiteral(n) => {
                let mut n: f64 = n.into();

                if let ParserResult::Some(u) = self.parse_time_unit() {
                    n = (n * u) / parent_len_s;
                }

                n
            }

            TokenType::Colon => {
                need_colon = false;
                0.
            }

            ty => return ParserResult::Err(ParserError::Unexpected(ty)),
        };

        if need_colon {
            self.eat(TokenType::Colon)?;
        }

        let end = match self.get_token().to_res_opt()? {
            Some(Token {
                ty: TokenType::NumberLiteral(n),
                ..
            }) => {
                let mut n: f64 = n.into();

                if let ParserResult::Some(u) = self.parse_time_unit() {
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

        ParserResult::Some((start, end))
    }

    fn parse_expression(&mut self) -> ParserResult<Expression, ParserError<S::Error>> {
        let mut output_queue = vec![];
        let mut ops = vec![];

        let prec = |t: &Token<'s, S>| match t.ty {
            TokenType::DoubleStar => 3,
            TokenType::Slash => 2,
            TokenType::Star => 2,
            TokenType::Percent => 2,
            TokenType::Plus => 1,
            TokenType::Minus => 1,
            TokenType::LeftParenthesis => 0,

            _ => unreachable!(),
        };

        #[derive(Clone, Copy, PartialEq, Eq)]
        enum Assoc {
            Left,
            Right,
            NonAssoc,
        }
        let assoc = |t: &Token<'s, S>| match t.ty {
            TokenType::LeftParenthesis | TokenType::RightParenthesis => Assoc::NonAssoc,
            TokenType::DoubleStar => Assoc::Right,

            TokenType::NumberLiteral(_) => panic!("operator assoc called on an integer."),

            _ => Assoc::Left,
        };

        // while there are tokens to be read:
        //     read a token
        while let ParserResult::Some(t) = self.get_token() {
            match t.ty {
                // if the token is:
                // - a number:
                //     put it into the output queue
                TokenType::NumberLiteral(_) => output_queue.push(t),

                // - a function:
                //  push it onto the operator stack
                TokenType::Identifier => match t.position.get_text().unwrap() {
                    f @ ("sin" | "cos" | "tan" | "ln" | "lg" | "log10" | "log2" | "sqrt"
                    | "abs" | "round" | "floor" | "ceil" | "rad" | "deg") => ops.push(t),

                    s => output_queue.push(t),
                },

                // - an operator o1:
                TokenType::Plus
                | TokenType::Minus
                | TokenType::Star
                | TokenType::Slash
                | TokenType::Caret
                | TokenType::LeftParenthesis => {
                    // while (
                    //     there is an operator o2 at the top of the operator stack which is not a left parenthesis,
                    //     and (o2 has greater precedence than o1 or (o1 and o2 have the same precedence and o1 is left-associative))
                    // ):
                    while ops
                        .last()
                        .map_or(false, |op| op.ty != TokenType::LeftParenthesis)
                        && (prec(ops.last().unwrap()) > prec(&t)
                            || (prec(&t) == prec(ops.last().unwrap()) && assoc(&t) == Assoc::Left))
                    {
                        // pop o2 from the operator stack into the output queue
                        output_queue.push(ops.pop().unwrap());
                    }
                    // push o1 onto the operator stack
                    ops.push(t);
                }

                // - a ",":
                TokenType::Comma => {
                    // while the operator at the top of the operator stack is not a left parenthesis:
                    while ops
                        .last()
                        .map_or(false, |op| op.ty != TokenType::LeftParenthesis)
                    {
                        // pop the operator from the operator stack into the output queue
                        output_queue.push(ops.pop().unwrap());
                    }
                }

                // - a left parenthesis (i.e. "("):
                TokenType::LeftParenthesis => {
                    // push it onto the operator stack
                    ops.push(t);
                }

                // - a right parenthesis (i.e. ")"):
                TokenType::RightParenthesis => {
                    // while the operator at the top of the operator stack is not a left parenthesis:

                    while ops
                        .last()
                        .map_or(false, |op| op.ty != TokenType::LeftParenthesis)
                    {
                        // {assert the operator stack is not empty}
                        // /* If the stack runs out without finding a left parenthesis, then there are mismatched parentheses. */
                        if ops.is_empty() {
                            todo!()
                        }

                        // pop the operator from the operator stack into the output queue
                        output_queue.push(ops.pop().unwrap());

                        // {assert there is a left parenthesis at the top of the operator stack}
                        // pop the left parenthesis from the operator stack and discard it
                        if let Some(Token {
                            ty: TokenType::LeftParenthesis,
                            ..
                        }) = ops.pop()
                        {
                            // do nothing?
                        } else {
                            todo!()
                        }

                        // if there is a function token at the top of the operator stack, then:
                        if !ops.is_empty()
                            && ops.last().unwrap().ty == TokenType::Identifier
                            && matches!(
                                t.position.get_text().unwrap(),
                                "sin"
                                    | "cos"
                                    | "tan"
                                    | "ln"
                                    | "lg"
                                    | "log10"
                                    | "log2"
                                    | "sqrt"
                                    | "abs"
                                    | "round"
                                    | "floor"
                                    | "ceil"
                                    | "rad"
                                    | "deg"
                            )
                        {
                            // pop the function from the operator stack into the output queue
                            output_queue.push(ops.pop().unwrap());
                        }
                    }
                }
                _ => todo!(),
            }
        }

        // /* After the while loop, pop the remaining items from the operator stack into the output queue. */
        // while there are tokens on the operator stack:
        while let Some(t) = ops.pop() {
            // /* If the operator token on the top of the stack is a parenthesis, then there are mismatched parentheses. */
            //     {assert the operator on top of the stack is not a (left) parenthesis}
            //     pop the operator from the operator stack onto the output queue
            if t.ty == TokenType::LeftParenthesis {
                todo!()
            }

            output_queue.push(t);
        }

        todo!()
    }
}

fn match_identifier<'name, 's, S: Source>(
    name: &'name str,
) -> impl 'name + FnOnce(&Token<'s, S>) -> bool {
    move |t| match t.ty {
        TokenType::Identifier => match t.position.get_text() {
            Some(t) => t == name,
            None => false,
        },

        _ => false,
    }
}

pub enum Expression {
    Add(Box<Expression>, Box<Expression>),
    Sub(Box<Expression>, Box<Expression>),

    Mul(Box<Expression>, Box<Expression>),
    Div(Box<Expression>, Box<Expression>),

    Pow(Box<Expression>, Box<Expression>),
    Mod(Box<Expression>, Box<Expression>),

    Call(String, Box<Expression>),

    VarOrConst(String),
    Lit(Number),
}

impl Expression {
    pub fn evaluate(&self, gi: gen::GenInfo) -> f64 {
        match self {
            Self::Add(a, b) => a.evaluate(gi) + b.evaluate(gi),
            Self::Sub(a, b) => a.evaluate(gi) - b.evaluate(gi),

            Self::Mul(a, b) => a.evaluate(gi) * b.evaluate(gi),
            Self::Div(a, b) => a.evaluate(gi) / b.evaluate(gi),

            Self::Mod(a, b) => a.evaluate(gi) % b.evaluate(gi),

            Self::Pow(a, b) => a.evaluate(gi).powf(b.evaluate(gi)),

            Self::Call(name, arg) => {
                let arg = arg.evaluate(gi);
                match &name[..] {
                    "sin" => f64::sin(arg),
                    "cos" => f64::cos(arg),
                    "tan" => f64::tan(arg),

                    "ln" => f64::ln(arg),
                    "lg" | "log10" => f64::log10(arg),
                    "log2" => f64::log2(arg),

                    "sqrt" => f64::sqrt(arg),

                    "abs" => f64::abs(arg),
                    "round" => f64::round(arg),
                    "floor" => f64::floor(arg),
                    "ceil" => f64::ceil(arg),
                    "rad" => f64::to_radians(arg),
                    "deg" => f64::to_degrees(arg),

                    _var => {
                        todo!()
                    }
                }
            }

            Self::VarOrConst(name) => match &name[..] {
                "pi" | "π" => std::f64::consts::PI,
                "e" => std::f64::consts::E,

                "channel" => gi.channel as f64,
                "t" => gi.t,

                _var => {
                    todo!()
                }
            },

            &Self::Lit(a) => a.into(),
        }
    }
}
