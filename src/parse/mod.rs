use self::{
    source::{Source, StringSource},
    tokenizer::{Token, TokenType, Tokenizer, TokenizerError},
};
use crate::gen::{self, Channels, Song};
use thiserror::Error as ThisError;

pub mod printing;
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
}

pub fn get_song<'src, 'name: 'src>(
    source_name: &'name str,
    src: &'src str,
) -> Result<Song, ParserError<<StringSource<'name, 'src> as Source>::Error>> {
    let mut diagnostics = vec![];

    let source = StringSource::new(source_name, src);
    let tokenizer = tokenizer::Tokenizer::new(source, &mut diagnostics);

    Parser::new(tokenizer).parse_song()
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

    pub fn parse_song<'src, 'name: 'src>(mut self) -> Result<Song, ParserError<S::Error>> {
        let mut sources = vec![];

        let Some(Token {
            ty: TokenType::StringLiteral(name),
            ..
        }) = self.get_token()?
        else {
            return Err(ParserError::MissingName);
        };

        self.song_length_s = match self.get_token()? {
            Some(Token {
                ty: TokenType::DurationLiteral(length),
                ..
            }) => length.as_secs_f64(),

            _ => return Err(ParserError::MissingDuration),
        };

        self.song_channels = match self.parse_chan()? {
            Some(Channels::One(channels)) => channels,

            _ => return Err(ParserError::MissingChannels),
        };

        while let Some(s) = self.parse_source()? {
            sources.push(s);
        }

        Ok(Song {
            channels: self.song_channels,
            length_s: self.song_length_s,
            sources,
            name,
        })
    }

    fn get_token(&mut self) -> Result<Option<Token<'s, S>>, TokenizerError<S::Error>> {
        if let Some(t) = self.buffer.pop() {
            Ok(Some(t))
        } else {
            self.tokenizer.get_next()
        }
    }

    fn eat(&mut self, ty: TokenType) -> Result<Option<Token<'s, S>>, ParserError<S::Error>> {
        Ok(match self.get_token()? {
            Some(token) => {
                if token.ty == ty {
                    Some(token)
                } else {
                    self.buffer.push(token);
                    None
                }
            }

            None => None,
        })
    }

    /* fn eat_f(
        &mut self,
        f: impl FnOnce(&Token<'s, S>) -> bool,
    ) -> Result<Option<Token<'s, S>>, ParserError<S::Error>> {
        Ok(match self.get_token()? {
            Some(token) => {
                if f(&token) {
                    Some(token)
                } else {
                    self.buffer.push(token);
                    None
                }
            }

            None => None,
        })
    } */

    fn parse_source(&mut self) -> Result<Option<gen::Source>, ParserError<S::Error>> {
        let Some(wave_type_pos) = self.eat(TokenType::Identifier)? else {
            return Ok(None);
        };

        let wave_type = wave_type_pos
            .position
            .get_text()
            .expect("Couldn't get identifier contents");

        let freq = match self.tokenizer.get_next()? {
            Some(Token {
                ty: TokenType::FreqLiteral(f),
                ..
            }) => f,

            _ => return Ok(None),
        };

        let Some((start, end)) = self.parse_timeframe(self.song_length_s)? else {
            return Ok(None);
        };

        let Some(channels) = self.parse_chan()? else {
            return Ok(None);
        };
        let Some(volume) = self.parse_vol()? else {
            return Ok(None);
        };

        let mut effects = vec![];

        let len_s = (end - start) * self.song_length_s;
        if self.eat(TokenType::LeftCurlyBraces)?.is_some() {
            while let Some(e) = self.parse_effect(len_s)? {
                effects.push(e);
            }

            if self.eat(TokenType::RightCurlyBraces)?.is_none() {
                return Ok(None);
            }
        }

        let ty = match wave_type {
            "sine" => gen::SourceType::Sine { freq, phase: 0. },
            "saw" => gen::SourceType::Saw { freq, phase: 0. },
            "tri" | "triangle" => gen::SourceType::Triangle { freq, phase: 0. },
            "sqaure" => gen::SourceType::Square { freq, phase: 0. },

            _ => return Ok(None),
        };

        Ok(Some(gen::Source {
            start,
            end,

            channels,
            volume,

            effects,

            ty,
        }))
    }

    fn parse_effect(
        &mut self,
        parent_len_s: f64,
    ) -> Result<Option<gen::Effect>, ParserError<S::Error>> {
        let Some(name) = self.eat(TokenType::Identifier)? else {
            return Ok(None);
        };
        let name = name
            .position
            .get_text()
            .expect("Couldn't get identifier name");

        let Some((start, end)) = self.parse_timeframe(parent_len_s)? else {
            return Ok(None);
        };

        let ty = match name {
            "fade_in" => gen::EffectType::FadeIn,
            "fade_out" => gen::EffectType::FadeOut,

            _ => return Ok(None),
        };

        Ok(Some(gen::Effect { ty, start, end }))
    }

    /* fn parse_list() -> Result<Vec<Token<'s, S>>, ParserError<S::Error>>
    where
        S: Source,
    {
        todo!()
    } */

    fn parse_chan(&mut self) -> Result<Option<Channels>, ParserError<S::Error>> {
        if self.eat(TokenType::OnKw)?.is_none() {
            return Ok(None);
        };

        match self.tokenizer.get_next()?.map(|t| t.ty) {
            Some(TokenType::IntLiteral(i)) => Ok(Some(Channels::One(i as usize))),
            Some(TokenType::Star) => Ok(Some(Channels::All)),

            _ => Ok(None),
        }
    }

    fn parse_vol(&mut self) -> Result<Option<f64>, ParserError<S::Error>> {
        if self.eat(TokenType::AtSign)?.is_none() {
            return Ok(None);
        };

        match self.tokenizer.get_next()? {
            Some(Token {
                ty: TokenType::IntLiteral(i),
                ..
            }) => Ok(Some(i as f64)),

            Some(Token {
                ty: TokenType::FloatLiteral(f),
                ..
            }) => Ok(Some(f)),

            _ => Ok(None),
        }
    }

    fn parse_time(&mut self, parent_len_s: f64) -> Result<Option<f64>, ParserError<S::Error>> {
        let Some(t) = self.tokenizer.get_next()? else {
            return Ok(None);
        };

        #[allow(clippy::let_and_return)]
        Ok(Some(match t.ty {
            TokenType::DurationLiteral(d) => {
                let d = d.as_secs_f64() / parent_len_s;
                d
            }
            TokenType::FloatLiteral(d) => {
                let d = d;
                d
            }
            TokenType::IntLiteral(d) => {
                let d = d as f64;
                d
            }

            _ => return Ok(None),
        }))
    }

    fn parse_timeframe(
        &mut self,
        parent_len_s: f64,
    ) -> Result<Option<(f64, f64)>, ParserError<S::Error>> {
        let Some(start) = self.parse_time(parent_len_s)? else {
            return Ok(None);
        };

        if self.eat(TokenType::Colon)?.is_none() {
            return Ok(None);
        }

        let Some(end) = self.parse_time(parent_len_s)? else {
            return Ok(None);
        };

        Ok(Some((start, end)))
    }
}

/*
fn parse_inner<T, E>() -> A<T, E> {
    todo!()
}
fn parse<T, E>() -> A<T, E> {
    parse_inner()?;

    todo!()
}

enum A<T, E> {
    Some(T),
    Err(E),
    Done,
}

impl<T, E> std::ops::Try for A<T, E> {
    type Output = T;
    type Residual = Option<E>;

    fn from_output(output: Self::Output) -> Self {
        A::Some(output)
    }

    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            A::Some(v) => ControlFlow::Continue(v),
            A::Err(e) => ControlFlow::Break(Some(e)),
            A::Done => ControlFlow::Break(None),
        }
    }
}

impl<T, E> std::ops::FromResidual for A<T, E> {
    fn from_residual(residual: <Self as std::ops::Try>::Residual) -> Self {
        match residual {
            Some(e) => Self::Err(e),
            None => Self::Done,
        }
    }
}
 */
