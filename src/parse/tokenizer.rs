use thiserror::Error as ThisError;

use super::source::{Diagnostic, DiagnosticLevel, Source};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    Identifier,
    Whitespace,

    IntLiteral(u64),
    FloatLiteral(f64),
    StringLiteral(String),
    CharLiteral(char),
    DurationLiteral(f64),
    FreqLiteral(f64),

    SingleLineComment,
    MultiLineComment,

    // ------------------------ KEYWORDS ------------------------
    OnKw,
    FromKw,
    ToKw,

    // ------------------------ OPERATORS ------------------------
    // unary
    DoublePlus,
    DoubleMinus,
    Tilda,
    Bang,
    // unary

    // binary
    Plus,
    Minus,
    Star,
    Slash,
    BangEquals,
    RightShift,
    LesserThan,
    DoubleStar,
    DoubleAnd,
    DoubleOr,
    DoubleEquals,
    GreaterThan,
    Caret,
    Percent,
    SingleAnd,
    SingleOr,
    LeftShift,
    // binary

    // assignment
    Equals,
    PlusEquals,
    MinusEquals,
    StarEquals,
    SlashEquals,
    TildaEquals,
    DoubleStarEquals,
    DoubleAndEquals,
    DoubleOrEquals,
    LesserThanEquals,
    GreaterThanEquals,
    CaretEquals,
    PercentEquals,
    SingleAndEquals,
    SingleOrEquals,
    LeftShiftEquals,
    RightShiftEquals,
    // assignment

    // ------------------------ MISC -----------------------------LeftParenthesis
    LeftParenthesis,
    RightParenthesis,

    LeftSquareBraces,
    RightSquareBraces,

    LeftCurlyBraces,
    RightCurlyBraces,

    FatArrow,
    Colon,
    Dot,
    DoubleDot,
    TripleDot,
    HashSymbol,
    AtSign,
    Comma,
    QuestionMark,
    Semicolon,
    Dollar,
    DoubleDollar,
    Underscore,
}

#[derive(Clone)]
pub struct Token<'s, S> {
    pub(crate) position: TokenPosition<'s, S>,
    pub(crate) t_type: TokenType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TokenPosition<'s, S> {
    pub(crate) source: &'s S,

    pub(crate) start: usize,
    pub(crate) end: usize,

    pub(crate) line: usize,
    pub(crate) column: usize,
}
impl<'s, S: Source> TokenPosition<'s, S> {
    pub fn len(&self) -> usize {
        self.end - self.start
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn source(&self) -> &S {
        self.source
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn column(&self) -> usize {
        self.column
    }

    pub fn get_text(&self) -> Option<&str> {
        self.source.get_text(self.into())
    }
}
impl<'s, S> From<&TokenPosition<'s, S>> for std::ops::Range<usize> {
    fn from(value: &TokenPosition<'s, S>) -> Self {
        value.start..(value.end + 1)
    }
}

impl<'s, S> Token<'s, S> {
    pub fn new(t_type: TokenType, position: TokenPosition<'s, S>) -> Self {
        Self { t_type, position }
    }

    pub fn position(&self) -> &TokenPosition<'s, S> {
        &self.position
    }

    pub fn t_type(&self) -> &TokenType {
        &self.t_type
    }
}

#[derive(Debug)]
pub struct Tokenizer<'s, S> {
    diagnostics: Vec<Diagnostic<'s, S>>,
    emit_whitespace: bool,
    buffer: Vec<char>,
    lines: Vec<usize>,
    column: usize,
    absolute_pos: usize,
    source: S,
}

#[derive(Debug, ThisError)]
pub enum TokenizerError<S> {
    #[error("Invalid char '{0}'")]
    Invalid(char),

    #[error("Unfinished char literal")]
    UnfinishedCharLiteral,

    #[error("Unfinished string literal")]
    UnfinishedStringLiteral,

    #[error(transparent)]
    Source(#[from] S),
}

// TODO: LazyLock
static KEYWORDS: std::sync::OnceLock<std::collections::HashMap<&'static str, TokenType>> =
    std::sync::OnceLock::new();
fn init_keywords() -> std::collections::HashMap<&'static str, TokenType> {
    let mut h = std::collections::HashMap::new();

    h.insert("on", TokenType::OnKw);
    h.insert("from", TokenType::FromKw);
    h.insert("to", TokenType::ToKw);

    h.insert("_", TokenType::Underscore);

    h
}

impl<'s, S: Source> Tokenizer<'s, S> {
    pub fn new(source: S) -> Self {
        KEYWORDS.get_or_init(init_keywords);

        Self {
            emit_whitespace: false,
            diagnostics: vec![],
            buffer: vec![],
            lines: vec![0],
            column: 0,
            source,
            absolute_pos: 0,
        }
    }

    pub fn get_position(&self, start: usize, start_line: usize) -> TokenPosition<'s, S> {
        TokenPosition {
            source: unsafe { std::mem::transmute(&self.source) },

            start,
            end: self.absolute_pos - 1,

            line: start_line,
            column: start - self.lines[start_line],
        }
    }

    fn get_char(&mut self) -> Result<Option<char>, S::Error> {
        let c = if let Some(c) = self.buffer.pop() {
            c
        } else if let Some(c) = self.source.get_next_char()? {
            c
        } else {
            return Ok(None);
        };

        self.absolute_pos += 1;
        match c {
            '\n' => {
                self.lines.push(self.absolute_pos);
                self.column = 0;
            }

            '\t' => self.column += 4,

            _ => self.column += 1,
        }

        Ok(Some(c))
    }

    fn add_buffer(&mut self, b: Option<char>) {
        let Some(b) = b else { return };

        self.buffer.push(b);
        self.absolute_pos -= 1;
        match b {
            '\n' => {
                self.lines.pop().unwrap();
                self.column = 0;
            }

            '\t' => self.column -= 4,

            _ => self.column -= 1,
        }
    }

    pub fn get_next(&mut self) -> Result<Option<Token<'s, S>>, TokenizerError<S::Error>> {
        if self.emit_whitespace {
            self.get_next_with_whitespace()
        } else {
            loop {
                match self.get_next_with_whitespace()? {
                    Some(t) if t.t_type == TokenType::Whitespace => continue,
                    t => return Ok(t),
                };
            }
        }
    }

    fn get_next_with_whitespace(
        &mut self,
    ) -> Result<Option<Token<'s, S>>, TokenizerError<S::Error>> {
        let start_line = self.lines.len() - 1;
        let start = self.absolute_pos;

        let first_char = match self.get_char()? {
            Some(c) => c,
            None => return Ok(None),
        };

        let t_type = match first_char {
            d @ '0'..='9' => {
                let num = self.get_num(d)?;
                self.get_num_like(num)?
            }

            '+' => match self.get_char()? {
                Some('+') => TokenType::DoublePlus,
                Some('=') => TokenType::PlusEquals,
                c => {
                    self.add_buffer(c);
                    TokenType::Plus
                }
            },

            '-' => match self.get_char()? {
                Some('-') => TokenType::DoubleMinus,
                Some('=') => TokenType::MinusEquals,
                c => {
                    self.add_buffer(c);
                    TokenType::Minus
                }
            },

            '*' => match self.get_char()? {
                Some('*') => match self.get_char()? {
                    Some('=') => TokenType::DoubleStarEquals,
                    c => {
                        self.add_buffer(c);
                        TokenType::DoubleStar
                    }
                },

                Some('=') => TokenType::StarEquals,

                c => {
                    self.add_buffer(c);
                    TokenType::Star
                }
            },

            '.' => match self.get_char()? {
                Some('.') => match self.get_char()? {
                    Some('.') => TokenType::TripleDot,
                    c => {
                        self.add_buffer(c);
                        TokenType::DoubleDot
                    }
                },

                c => {
                    self.add_buffer(c);
                    TokenType::Dot
                }
            },

            '$' => match self.get_char()? {
                Some('$') => TokenType::DoubleDollar,
                c => {
                    self.add_buffer(c);
                    TokenType::Dollar
                }
            },

            '(' => TokenType::LeftParenthesis,
            ')' => TokenType::RightParenthesis,

            '[' => TokenType::LeftSquareBraces,
            ']' => TokenType::RightSquareBraces,

            '{' => TokenType::LeftCurlyBraces,
            '}' => TokenType::RightCurlyBraces,

            '~' => match self.get_char()? {
                Some('=') => TokenType::TildaEquals,
                c => {
                    self.add_buffer(c);
                    TokenType::Tilda
                }
            },

            '%' => match self.get_char()? {
                Some('=') => TokenType::PercentEquals,
                c => {
                    self.add_buffer(c);
                    TokenType::Percent
                }
            },

            '!' => match self.get_char()? {
                Some('=') => TokenType::BangEquals,
                c => {
                    self.add_buffer(c);
                    TokenType::Bang
                }
            },

            ':' => TokenType::Colon,
            '#' => TokenType::HashSymbol,
            '@' => TokenType::AtSign,
            ',' => TokenType::Comma,
            '?' => TokenType::QuestionMark,
            ';' => TokenType::Semicolon,

            '^' => match self.get_char()? {
                Some('=') => TokenType::CaretEquals,
                c => {
                    self.add_buffer(c);
                    TokenType::Caret
                }
            },

            '=' => match self.get_char()? {
                Some('=') => TokenType::DoubleEquals,
                Some('>') => TokenType::FatArrow,
                c => {
                    self.add_buffer(c);
                    TokenType::Equals
                }
            },

            '/' => match self.get_char()? {
                Some('=') => TokenType::SlashEquals,

                Some('/') => {
                    loop {
                        if let Some('\n') = self.get_char()? {
                            self.add_buffer(Some('\n'));
                            break;
                        }
                    }

                    TokenType::SingleLineComment
                }

                Some('*') => {
                    let mut level = 1;
                    loop {
                        match self.get_char()? {
                            Some('/') => {
                                if let Some('*') = self.get_char()? {
                                    level += 1;
                                }
                            }

                            Some('*') => {
                                if let Some('/') = self.get_char()? {
                                    level -= 1;
                                    if level == 0 {
                                        break;
                                    }
                                }
                            }

                            Some(_) => (),

                            None => {
                                let position = self.get_position(start, start_line);
                                let value = Diagnostic::new(
                                    position,
                                    String::from("Unclosed multiline comment"),
                                    DiagnosticLevel::Info,
                                );
                                self.diagnostics.push(value);
                            }
                        }
                    }

                    TokenType::MultiLineComment
                }

                c => {
                    self.add_buffer(c);
                    TokenType::Slash
                }
            },

            id @ ('a'..='z' | 'A'..='Z' | '_') => {
                let mut id = id.to_string();

                loop {
                    match self.get_char()? {
                        Some(c @ ('a'..='z' | 'A'..='Z' | '_')) => id.push(c),

                        c => {
                            self.add_buffer(c);

                            break if let Some(kw) = KEYWORDS.get().unwrap().get(id.as_str()) {
                                kw.clone()
                            } else {
                                TokenType::Identifier
                            };
                        }
                    }
                }
            }

            ' ' | '\n' | '\t' | '\r' => loop {
                match self.get_char()? {
                    Some(' ' | '\t' | '\r' | '\n') => (),

                    c => {
                        self.add_buffer(c);

                        break TokenType::Whitespace;
                    }
                }
            },

            '\'' => {
                let c = match self.get_char()? {
                    Some('\\') => match self.get_char()? {
                        Some(c) => unescape(c),
                        None => return Err(TokenizerError::UnfinishedCharLiteral),
                    },

                    Some(c) => c,

                    None => return Err(TokenizerError::UnfinishedCharLiteral),
                };

                match self.get_char()? {
                    Some('\'') => TokenType::CharLiteral(c),
                    _ => return Err(TokenizerError::UnfinishedCharLiteral),
                }
            }

            '"' => {
                let mut s = String::new();

                loop {
                    match self.get_char()? {
                        Some('\\') => match self.get_char()? {
                            Some(c) => s.push(unescape(c)),
                            None => return Err(TokenizerError::UnfinishedStringLiteral),
                        },

                        Some('"') => break,
                        Some(c) => s.push(c),

                        None => return Err(TokenizerError::UnfinishedStringLiteral),
                    }
                }

                TokenType::StringLiteral(s)
            }

            '<' => match self.get_char()? {
                Some('<') => match self.get_char()? {
                    Some('=') => TokenType::LeftShiftEquals,
                    c => {
                        self.add_buffer(c);
                        TokenType::LeftShift
                    }
                },
                Some('=') => TokenType::LesserThanEquals,
                c => {
                    self.add_buffer(c);
                    TokenType::LesserThan
                }
            },

            '>' => match self.get_char()? {
                Some('>') => match self.get_char()? {
                    Some('=') => TokenType::RightShiftEquals,
                    c => {
                        self.add_buffer(c);
                        TokenType::RightShift
                    }
                },
                Some('=') => TokenType::GreaterThanEquals,
                c => {
                    self.add_buffer(c);
                    TokenType::GreaterThan
                }
            },

            '&' => match self.get_char()? {
                Some('&') => match self.get_char()? {
                    Some('=') => TokenType::DoubleAndEquals,
                    c => {
                        self.add_buffer(c);
                        TokenType::DoubleAnd
                    }
                },
                Some('=') => TokenType::SingleAndEquals,
                c => {
                    self.add_buffer(c);
                    TokenType::SingleAnd
                }
            },

            '|' => match self.get_char()? {
                Some('|') => match self.get_char()? {
                    Some('=') => TokenType::DoubleOrEquals,
                    c => {
                        self.add_buffer(c);
                        TokenType::DoubleOr
                    }
                },
                Some('=') => TokenType::SingleOrEquals,
                c => {
                    self.add_buffer(c);
                    TokenType::SingleOr
                }
            },

            chr => return Err(TokenizerError::Invalid(chr)),
        };

        Ok(Some(Token::new(
            t_type,
            self.get_position(start, start_line),
        )))
    }

    pub fn emit_whitespace(&self) -> bool {
        self.emit_whitespace
    }
    pub fn set_emit_whitespace(&mut self, emit_whitespace: bool) {
        self.emit_whitespace = emit_whitespace;
    }

    fn get_num(&mut self, d: char) -> Result<TokenType, TokenizerError<S::Error>> {
        let mut s = vec![d as u8 - b'0'];

        let dot_or_bogus = loop {
            match self.get_char()? {
                Some(c @ '0'..='9') => s.push(c as u8 - b'0'),
                c => break c,
            }
        };

        let mut int_part = 0;
        let mut x = 1;
        for i in (0..s.len()).rev() {
            int_part += s[i] as u64 * x;
            x *= 10;
        }

        match dot_or_bogus {
            Some('.') => self.get_float_or_int(int_part, &mut s),

            c => {
                self.add_buffer(c);
                Ok(TokenType::IntLiteral(int_part))
            }
        }
    }

    fn get_float_or_int(
        &mut self,
        int_part: u64,
        s: &mut Vec<u8>,
    ) -> Result<TokenType, TokenizerError<S::Error>> {
        s.clear();

        let cc = loop {
            match self.get_char()? {
                Some(c @ '0'..='9') => s.push(c as u8 - b'0'),

                c => break c,
            }
        };

        self.add_buffer(cc);

        // we already had a dot and the next character wasn't a digit
        if s.is_empty() {
            self.add_buffer(Some('.'));

            return Ok(TokenType::IntLiteral(int_part));
        }

        let mut fract = 0.;
        let mut x = 1.;
        for i in s {
            x *= 0.1;
            fract += *i as f64 * x;
        }

        Ok(TokenType::FloatLiteral(int_part as f64 + fract))
    }

    fn get_num_like(&mut self, num: TokenType) -> Result<TokenType, TokenizerError<S::Error>> {
        let v = match num {
            TokenType::IntLiteral(v) => v as f64,
            TokenType::FloatLiteral(v) => v,

            _ => unreachable!(),
        };

        match self.get_char()? {
            Some('s') => Ok(TokenType::DurationLiteral(v)),

            Some('m') => match self.get_char()? {
                Some('s') => Ok(TokenType::DurationLiteral(v * 0.0001)),

                c => {
                    self.add_buffer(c);
                    Ok(TokenType::DurationLiteral(v * 1000.))
                }
            },

            Some('h') => match self.get_char()? {
                Some('z') => Ok(TokenType::FreqLiteral(v * 0.0001)),

                c => {
                    self.add_buffer(Some('h'));
                    self.add_buffer(c);
                    Ok(TokenType::DurationLiteral(v * 1000.))
                }
            },

            c => {
                self.add_buffer(c);
                Ok(num)
            }
        }
    }
}

pub fn unescape(c: char) -> char {
    match c {
        '0' => '\0',
        't' => '\t',
        'n' => '\n',
        c => c,
    }
}
