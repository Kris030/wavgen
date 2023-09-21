use super::{
    source::Source,
    tokenizer::{FloatValue, IntegerValue, Token, TokenPosition, TokenType},
};

impl<'s, S: Source> std::fmt::Display for TokenPosition<'s, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}[{}-{} | Line {}, Col {}-{}]",
            self.source.get_name(),
            self.start,
            self.end,
            self.line,
            self.column,
            self.column + self.len()
        )
    }
}

impl<'s, S: Source + std::fmt::Debug> std::fmt::Debug for Token<'s, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Token")
            .field("position", &self.position.to_string())
            .field("type", &self.t_type)
            .finish()
    }
}
impl<'s, S: Source> std::fmt::Display for Token<'s, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.t_type {
            TokenType::IntLiteral(v) => match v {
                IntegerValue::Integer(v) => write!(f, "{v}"),
            },
            TokenType::FloatLiteral(v) => match v {
                FloatValue::Double(v) => write!(f, "{v}"),
            },
            TokenType::CharLiteral(v) => write!(f, "'{}'", escape(&v.to_string())),
            TokenType::StringLiteral(v) => write!(f, "'{}'", escape(v)),
            TokenType::DurationLiteral(d) => write!(f, "{d:?}"),

            TokenType::Whitespace
            | TokenType::Identifier
            | TokenType::SingleLineComment
            | TokenType::MultiLineComment => {
                if let Some(t) = self.position.source.get_text((&(self.position)).into()) {
                    write!(f, "{t}")
                } else {
                    write!(f, "[identifier]")
                }
            }

            TokenType::Plus => write!(f, "+"),
            TokenType::Minus => write!(f, "-"),
            TokenType::Star => write!(f, "*"),
            TokenType::Slash => write!(f, "/"),
            TokenType::Dot => write!(f, "."),
            TokenType::Underscore => write!(f, "_"),
            TokenType::Dollar => write!(f, "$"),

            TokenType::LeftParenthesis => write!(f, "("),
            TokenType::RightParenthesis => write!(f, ")"),

            TokenType::LeftSquareBraces => write!(f, "["),
            TokenType::RightSquareBraces => write!(f, "]"),

            TokenType::LeftCurlyBraces => write!(f, "{{"),
            TokenType::RightCurlyBraces => write!(f, "}}"),

            TokenType::OnKw => write!(f, "on"),
            TokenType::FromKw => write!(f, "from"),
            TokenType::ToKw => write!(f, "to"),

            TokenType::DoublePlus => write!(f, "++"),
            TokenType::DoubleMinus => write!(f, "--"),
            TokenType::Tilda => write!(f, "~"),
            TokenType::Bang => write!(f, "!"),
            TokenType::BangEquals => write!(f, "!="),
            TokenType::RightShift => write!(f, ">>"),
            TokenType::LesserThan => write!(f, "<"),
            TokenType::DoubleStar => write!(f, "**"),
            TokenType::DoubleAnd => write!(f, "&&"),
            TokenType::DoubleOr => write!(f, "||"),
            TokenType::DoubleEquals => write!(f, "=="),
            TokenType::GreaterThan => write!(f, ">"),
            TokenType::Caret => write!(f, "^"),
            TokenType::Percent => write!(f, "%"),
            TokenType::SingleAnd => write!(f, "&"),
            TokenType::SingleOr => write!(f, "|"),
            TokenType::LeftShift => write!(f, "<<"),
            TokenType::Equals => write!(f, "="),
            TokenType::PlusEquals => write!(f, "+="),
            TokenType::MinusEquals => write!(f, "-="),
            TokenType::StarEquals => write!(f, "*="),
            TokenType::SlashEquals => write!(f, "/="),
            TokenType::TildaEquals => write!(f, "~="),
            TokenType::DoubleStarEquals => write!(f, "**="),
            TokenType::DoubleAndEquals => write!(f, "&&="),
            TokenType::DoubleOrEquals => write!(f, "||="),
            TokenType::LesserThanEquals => write!(f, "<="),
            TokenType::GreaterThanEquals => write!(f, ">="),
            TokenType::CaretEquals => write!(f, "^="),
            TokenType::PercentEquals => write!(f, "%="),
            TokenType::SingleAndEquals => write!(f, "&="),
            TokenType::SingleOrEquals => write!(f, "|="),
            TokenType::LeftShiftEquals => write!(f, "<<="),
            TokenType::RightShiftEquals => write!(f, ">>="),
            TokenType::FatArrow => write!(f, "=>"),
            TokenType::Colon => write!(f, ":"),
            TokenType::DoubleDot => write!(f, ".."),
            TokenType::TripleDot => write!(f, "..."),
            TokenType::HashSymbol => write!(f, "#"),
            TokenType::AtSign => write!(f, "@"),
            TokenType::Comma => write!(f, ","),
            TokenType::QuestionMark => write!(f, "?"),
            TokenType::Semicolon => write!(f, ";"),
            TokenType::DoubleDollar => write!(f, "$$"),
        }
    }
}

pub fn escape(s: &str) -> String {
    let mut r = String::new();

    for c in s.chars() {
        match c {
            '\'' => r += "\\'",
            '\"' => r += "\\\"",
            '\\' => r += "\\\\",
            '\t' => r += "\\t",
            '\r' => r += "\\r",
            '\n' => r += "\\n",
            '\0' => r += "\\0",

            _ => r.push(c),
        }
    }

    r
}
