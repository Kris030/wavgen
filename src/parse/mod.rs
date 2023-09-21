use self::{
    source::{Source, StringSource},
    tokenizer::{FloatValue, IntegerValue, Token, TokenType},
};
use crate::gen::Song;
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
        t_type: TokenType::IntLiteral(IntegerValue::Integer(channels)),
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
        t_type: TokenType::FloatLiteral(FloatValue::Double(volume)),
        ..
    }) = tokenizer.get_next()?
    else {
        todo!()
    };

    loop {
        let wave = match tokenizer.get_next()? {
            Some(Token {
                t_type: TokenType::Identifier,
                position,
            }) => ,
            _ => todo!(),
        };
    }

    Ok(Song {
        channels: channels as usize,
        sources,
        length,
        name,
    })
}
