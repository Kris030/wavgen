use self::{
    source::{Source, StringSource},
    tokenizer::{Token, TokenType, Tokenizer},
};
use crate::gen::{self, Song};
use thiserror::Error as ThisError;

pub mod printing;
pub mod source;
pub mod tokenizer;

#[derive(Debug, ThisError)]
pub enum ParserError<S> {
    #[error("No name was provided")]
    MissingName,

    #[error(transparent)]
    TokenizerError(#[from] tokenizer::TokenizerError<S>),
}

pub fn get_song<'src, 'name: 'src>(
    source_name: &'name str,
    src: &'src str,
) -> Result<Song, ParserError<<StringSource<'name, 'src> as Source>::Error>> {
    let mut tokenizer = tokenizer::Tokenizer::new(StringSource::new(source_name, src));

    let mut sources = vec![];

    let name = match tokenizer.get_next()? {
        Some(Token {
            t_type: TokenType::StringLiteral(s),
            ..
        }) => s,
        _ => return Err(ParserError::MissingName),
    };

    let Some(Token {
        t_type: TokenType::DurationLiteral(length),
        ..
    }) = tokenizer.get_next()?
    else {
        todo!()
    };

    let Some(Token {
        t_type: TokenType::OnKw,
        ..
    }) = tokenizer.get_next()?
    else {
        todo!()
    };

    let Some(Token {
        t_type: TokenType::IntLiteral(channels),
        ..
    }) = tokenizer.get_next()?
    else {
        todo!()
    };

    let Some(Token {
        t_type: TokenType::AtSign,
        ..
    }) = tokenizer.get_next()?
    else {
        todo!()
    };

    let Some(Token {
        t_type: TokenType::FloatLiteral(volume),
        ..
    }) = tokenizer.get_next()?
    else {
        todo!()
    };

    while let Some(s) = get_source(&mut tokenizer)? {
        sources.push(s);
    }

    Ok(Song {
        channels: channels as usize,
        sources,
        length,
        name,
    })
}

fn get_source<S>(
    tokenizer: &mut Tokenizer<'_, S>,
) -> Result<Option<gen::Source>, ParserError<S::Error>>
where
    S: Source,
{
    macro_rules! next {
        () => {{
            match tokenizer.get_next()? {
                Some(t) => t,
                None => return Ok(None),
            }
        }};

        ($p:pat) => {{
            match tokenizer.get_next()? {
                Some(t @ $p) => t,
                _ => return Ok(None),
            }
        }};

        ($($p:pat => $e:expr,)+) => {{
            match tokenizer.get_next()? {
                $(Some($p) => $e,)+
                _ => return Ok(None),
            }
        }};
    }

    // let t = next!();
    // let t = next!(Token {
    //     t_type: TokenType::Colon,
    //     ..
    // });
    // let t = next! {
    //     Token {
    //         t_type: TokenType::FreqLiteral(freq),
    //         ..
    //     } => freq,

    //     Token {
    //         t_type: TokenType::DurationLiteral(d),
    //         ..
    //     } => panic!(),
    // };

    let wave_type = next! {
        Token {
            t_type: TokenType::Identifier,
            position,
        } => position,
    }
    .get_text()
    .unwrap();

    let freq = next! {
        Token {
            t_type: TokenType::FreqLiteral(f),
            ..
        } => f,
    };

    let from = next! {
        Token {
            t_type: TokenType::DurationLiteral(d),
            ..
        } => d,
    };

    next!(Token {
        t_type: TokenType::Colon,
        ..
    });

    let to = next! {
        Token {
            t_type: TokenType::DurationLiteral(d),
            ..
        } => d,
    };

    todo!()
}
