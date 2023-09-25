use super::tokenizer::TokenPosition;

pub trait Source: std::fmt::Debug {
    type Error: std::error::Error;

    fn get_next_char(&mut self) -> Result<Option<char>, Self::Error>;
    fn get_name(&self) -> &str;

    fn get_text(&self, pos: std::ops::Range<usize>) -> Option<&str>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticLevel {
    Info,
    Warning,
    Error,
    Abort,
}

#[derive(Debug, Clone)]
pub struct Diagnostic<'s, S> {
    position: TokenPosition<'s, S>,
    message: String,
    level: DiagnosticLevel,
}

impl<'s, S> Diagnostic<'s, S> {
    pub fn new(position: TokenPosition<'s, S>, message: String, level: DiagnosticLevel) -> Self {
        Self {
            position,
            message,
            level,
        }
    }

    pub fn position(&self) -> &TokenPosition<'s, S> {
        &self.position
    }
    pub fn message(&self) -> &str {
        self.message.as_ref()
    }
    pub fn level(&self) -> &DiagnosticLevel {
        &self.level
    }
}

#[derive(Debug)]
pub struct StringSource<'name, 'text> {
    name: &'name str,
    text: &'text str,
    chars: Vec<char>,
    pos: usize,
}

impl<'a, 'b> StringSource<'a, 'b> {
    pub fn new(name: &'a str, text: &'b str) -> Self {
        Self {
            name,
            text,
            chars: text.chars().collect(),
            pos: 0,
        }
    }
}

impl Source for StringSource<'_, '_> {
    type Error = std::convert::Infallible;

    fn get_next_char(&mut self) -> Result<Option<char>, Self::Error> {
        if self.pos >= self.chars.len() {
            return Ok(None);
        }

        let c = self.chars[self.pos];
        self.pos += 1;
        Ok(Some(c))
    }

    fn get_name(&self) -> &str {
        self.name
    }

    fn get_text<'s>(&self, pos: std::ops::Range<usize>) -> Option<&str> {
        Some(&self.text[pos])
    }
}

pub struct FileSource<'a> {
    file: std::io::BufReader<std::fs::File>,
    name: &'a str,
    prev: String,
}

impl<'a> FileSource<'a> {
    pub fn new(file: &'a str) -> std::io::Result<Self> {
        Ok(Self {
            file: std::io::BufReader::new(std::fs::File::open(file)?),
            prev: String::new(),
            name: file,
        })
    }
}
impl std::fmt::Debug for FileSource<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileSource")
            .field("name", &self.name)
            .finish()
    }
}
impl Source for FileSource<'_> {
    type Error = std::io::Error;

    fn get_name(&self) -> &str {
        self.name
    }

    fn get_next_char(&mut self) -> Result<Option<char>, Self::Error> {
        let mut b = [0];

        match std::io::Read::read_exact(&mut self.file, &mut b) {
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            _ => (),
        }

        let c = b[0] as char;
        self.prev.push(c);

        Ok(Some(c))
    }

    fn get_text(&self, pos: std::ops::Range<usize>) -> Option<&str> {
        Some(&self.prev[pos])
    }
}

#[derive(Default)]
pub struct StdinSource {
    line: Vec<char>,
    prev: String,
    done: bool,
}
impl StdinSource {
    pub fn new() -> Self {
        Self {
            line: vec![],
            prev: String::new(),
            done: false,
        }
    }
}
impl std::fmt::Debug for StdinSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StdinSource").finish()
    }
}
impl Source for StdinSource {
    type Error = std::io::Error;

    fn get_next_char(&mut self) -> Result<Option<char>, Self::Error> {
        if self.done {
            return Ok(None);
        }

        let mut s = String::new();
        while self.line.is_empty() {
            s.clear();

            std::io::stdin().read_line(&mut s)?;

            if s.is_empty() {
                self.done = true;
                return Ok(None);
            }

            self.line.extend(s.chars().rev());
        }

        Ok(Some(self.line.remove(self.line.len() - 1)))
    }

    fn get_name(&self) -> &str {
        "stdin"
    }

    fn get_text<'s>(&self, pos: std::ops::Range<usize>) -> Option<&str> {
        Some(&self.prev[pos])
    }
}

impl<E: std::error::Error> Source for &mut dyn Source<Error = E> {
    type Error = E;

    fn get_next_char(&mut self) -> Result<Option<char>, Self::Error> {
        (**self).get_next_char()
    }

    fn get_name(&self) -> &str {
        (**self).get_name()
    }

    fn get_text(&self, pos: std::ops::Range<usize>) -> Option<&str> {
        (**self).get_text(pos)
    }
}
