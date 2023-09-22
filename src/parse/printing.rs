use super::{
    source::Source,
    tokenizer::{Token, TokenPosition, TokenType},
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
        use TokenType::*;
        match &self.t_type {
            IntLiteral(v) => write!(f, "{v}"),
            FloatLiteral(v) => write!(f, "{v}"),

            CharLiteral(v) => write!(f, "'{}'", escape(&v.to_string())),
            StringLiteral(v) => write!(f, "'{}'", escape(v)),
            DurationLiteral(d) => write!(f, "{d:?}"),
            FreqLiteral(fq) => write!(f, "{fq}hz"),

            Whitespace | Identifier | SingleLineComment | MultiLineComment => {
                if let Some(t) = self.position.source.get_text((&(self.position)).into()) {
                    write!(f, "{t}")
                } else {
                    write!(f, "[identifier]")
                }
            }

            Plus => write!(f, "+"),
            Minus => write!(f, "-"),
            Star => write!(f, "*"),
            Slash => write!(f, "/"),
            Dot => write!(f, "."),
            Underscore => write!(f, "_"),
            Dollar => write!(f, "$"),

            LeftParenthesis => write!(f, "("),
            RightParenthesis => write!(f, ")"),

            LeftSquareBraces => write!(f, "["),
            RightSquareBraces => write!(f, "]"),

            LeftCurlyBraces => write!(f, "{{"),
            RightCurlyBraces => write!(f, "}}"),

            OnKw => write!(f, "on"),
            FromKw => write!(f, "from"),
            ToKw => write!(f, "to"),

            DoublePlus => write!(f, "++"),
            DoubleMinus => write!(f, "--"),
            Tilda => write!(f, "~"),
            Bang => write!(f, "!"),
            BangEquals => write!(f, "!="),
            RightShift => write!(f, ">>"),
            LesserThan => write!(f, "<"),
            DoubleStar => write!(f, "**"),
            DoubleAnd => write!(f, "&&"),
            DoubleOr => write!(f, "||"),
            DoubleEquals => write!(f, "=="),
            GreaterThan => write!(f, ">"),
            Caret => write!(f, "^"),
            Percent => write!(f, "%"),
            SingleAnd => write!(f, "&"),
            SingleOr => write!(f, "|"),
            LeftShift => write!(f, "<<"),
            Equals => write!(f, "="),
            PlusEquals => write!(f, "+="),
            MinusEquals => write!(f, "-="),
            StarEquals => write!(f, "*="),
            SlashEquals => write!(f, "/="),
            TildaEquals => write!(f, "~="),
            DoubleStarEquals => write!(f, "**="),
            DoubleAndEquals => write!(f, "&&="),
            DoubleOrEquals => write!(f, "||="),
            LesserThanEquals => write!(f, "<="),
            GreaterThanEquals => write!(f, ">="),
            CaretEquals => write!(f, "^="),
            PercentEquals => write!(f, "%="),
            SingleAndEquals => write!(f, "&="),
            SingleOrEquals => write!(f, "|="),
            LeftShiftEquals => write!(f, "<<="),
            RightShiftEquals => write!(f, ">>="),
            FatArrow => write!(f, "=>"),
            Colon => write!(f, ":"),
            DoubleDot => write!(f, ".."),
            TripleDot => write!(f, "..."),
            HashSymbol => write!(f, "#"),
            AtSign => write!(f, "@"),
            Comma => write!(f, ","),
            QuestionMark => write!(f, "?"),
            Semicolon => write!(f, ";"),
            DoubleDollar => write!(f, "$$"),
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
