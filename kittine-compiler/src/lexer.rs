//! Tokenizer for the Kittine (.kitty) language.
//!
//! Kittine mixes a handful of custom multi-character operators with an
//! embedded JSX-like syntax, so the lexer is a single hand-written scanner
//! that always prefers the longest matching operator before falling back to
//! single characters. Every token carries its source line/column so the
//! parser can reconstruct Python-style indentation blocks for `if>` /
//! `orif>` / `else>` bodies (which use indentation, not braces).

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Custom Kittine operators
    LeftVarBracket, // <{
    OpAssign,       // >>  (also doubles as equality in `if>` conditions)
    KeywordIf,      // if>
    KeywordOrif,    // orif>
    KeywordElse,    // else>
    KeywordCraft,   // craft<
    KeywordWarn,    // warn<
    KeywordError,   // error<
    KeywordPounce,  // pounce> subject  Variant(binding)? >> .. else> ..

    // Type-tag sigils: #n (Num), #w (Word), #f (Flag), each optionally
    // followed by `[]` for an array of that type. Deliberately shorter than
    // every Rust type they lower to (f64/String/bool/Vec<..>) and need no
    // closing delimiter, unlike the retired `<<Type>>` form.
    TypeNum,  // #n
    TypeWord, // #w
    TypeFlag, // #f
    /// `#t` — the generic type-parameter placeholder inside a `litter`/
    /// `breed`'s own `<#t>` declaration and field/variant types (e.g.
    /// `litter Box<#t> { value #t }`). Distinct from `TypeNum`/`TypeWord`/
    /// `TypeFlag`, which name a concrete scalar; this names "whatever type
    /// this specific litter/breed was instantiated with."
    TypeGeneric,

    // JSX punctuation
    Lt,      // <
    Gt,      // >
    SlashGt, // />
    LtSlash, // </

    // Comparison operators (`<` / `>` above double as these when they
    // appear after an already-parsed left-hand side; see `parser::
    // parse_comparison_tail`)
    LtEq,  // <=
    GtEq,  // >=
    BangEq, // !=

    // Logical operators (combine comparisons in a condition or expression)
    AmpAmp,   // &&
    PipePipe, // ||
    Amp,      // & (a real Rust reference -- see ast::Expr::Ref)
    Pipe,     // | (a closure literal's param-list delimiter -- see ast::Expr::Closure)

    // Keywords
    KeywordFunc,
    KeywordReturn,
    KeywordSpin, // spin<{item}> in <list> }{ .. }{
    KeywordPurr,    // purr name(..) #t { .. }
    KeywordPrivate, // private func/purr ..
    KeywordImport, // import { A, B } from '...'
    KeywordFrom,
    KeywordExport, // export import { A } from '...' -- re-export
    KeywordHold,   // hold name >> expr -- plain (non-reactive) local binding
    KeywordLitter, // litter Name (<#t>)? { field (#t)?, .. } -- a struct
    KeywordBreed,  // breed Name (<#t>)? { Variant (type)?, .. } -- an enum
    KeywordClaw,   // claw Name { method(params) type, .. } -- a trait
    KeywordBare,   // bare Claw for Target { purr method(..) .. }* -- an impl
    KeywordFor,    // the "for" in "bare Claw for Target"

    // Literals / identifiers
    Ident(String),
    Number(f64),
    Str(String),
    Bool(bool), // yes> / no>

    // Punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket, // [
    RBracket, // ]
    Comma,
    Dot,
    Colon,      // : (struct-literal field: value)
    ColonColon, // :: (path-qualified expressions: Type::method(), Type::CONST)
    Eq, // = (JSX attribute assignment)
    Plus,
    Minus,
    Star,
    Slash,
    Bang,

    Eof,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::LeftVarBracket => write!(f, "<{{"),
            TokenKind::OpAssign => write!(f, ">>"),
            TokenKind::KeywordIf => write!(f, "if>"),
            TokenKind::KeywordOrif => write!(f, "orif>"),
            TokenKind::KeywordElse => write!(f, "else>"),
            TokenKind::KeywordCraft => write!(f, "craft<"),
            TokenKind::KeywordWarn => write!(f, "warn<"),
            TokenKind::KeywordError => write!(f, "error<"),
            TokenKind::KeywordPounce => write!(f, "pounce>"),
            TokenKind::TypeNum => write!(f, "#n"),
            TokenKind::TypeWord => write!(f, "#w"),
            TokenKind::TypeFlag => write!(f, "#f"),
            TokenKind::TypeGeneric => write!(f, "#t"),
            TokenKind::Lt => write!(f, "<"),
            TokenKind::Gt => write!(f, ">"),
            TokenKind::SlashGt => write!(f, "/>"),
            TokenKind::LtSlash => write!(f, "</"),
            TokenKind::LtEq => write!(f, "<="),
            TokenKind::GtEq => write!(f, ">="),
            TokenKind::BangEq => write!(f, "!="),
            TokenKind::AmpAmp => write!(f, "&&"),
            TokenKind::Amp => write!(f, "&"),
            TokenKind::PipePipe => write!(f, "||"),
            TokenKind::Pipe => write!(f, "|"),
            TokenKind::KeywordFunc => write!(f, "func"),
            TokenKind::KeywordReturn => write!(f, "return"),
            TokenKind::KeywordSpin => write!(f, "spin"),
            TokenKind::KeywordPurr => write!(f, "purr"),
            TokenKind::KeywordPrivate => write!(f, "private"),
            TokenKind::KeywordImport => write!(f, "import"),
            TokenKind::KeywordFrom => write!(f, "from"),
            TokenKind::KeywordExport => write!(f, "export"),
            TokenKind::KeywordHold => write!(f, "hold"),
            TokenKind::KeywordLitter => write!(f, "litter"),
            TokenKind::KeywordBreed => write!(f, "breed"),
            TokenKind::KeywordClaw => write!(f, "claw"),
            TokenKind::KeywordBare => write!(f, "bare"),
            TokenKind::KeywordFor => write!(f, "for"),
            TokenKind::Ident(s) => write!(f, "{s}"),
            TokenKind::Number(n) => write!(f, "{n}"),
            TokenKind::Str(s) => write!(f, "\"{s}\""),
            TokenKind::Bool(true) => write!(f, "yes>"),
            TokenKind::Bool(false) => write!(f, "no>"),
            TokenKind::LParen => write!(f, "("),
            TokenKind::RParen => write!(f, ")"),
            TokenKind::LBrace => write!(f, "{{"),
            TokenKind::RBrace => write!(f, "}}"),
            TokenKind::LBracket => write!(f, "["),
            TokenKind::RBracket => write!(f, "]"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Dot => write!(f, "."),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::ColonColon => write!(f, "::"),
            TokenKind::Eq => write!(f, "="),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Bang => write!(f, "!"),
            TokenKind::Eof => write!(f, "<eof>"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

#[derive(Debug)]
pub struct LexError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "lex error at {}:{}: {}",
            self.line, self.col, self.message
        )
    }
}

impl Lexer {
    pub fn new(src: &str) -> Self {
        Lexer {
            chars: src.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn starts_with(&self, s: &str) -> bool {
        let chars: Vec<char> = s.chars().collect();
        for (i, c) in chars.iter().enumerate() {
            if self.peek_at(i) != Some(*c) {
                return false;
            }
        }
        true
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c == ' ' || c == '\t' || c == '\r' || c == '\n' => {
                    self.advance();
                }
                Some('/') if self.peek_at(1) == Some('/') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    fn read_string(&mut self, quote: char) -> Result<String, LexError> {
        let (line, col) = (self.line, self.col);
        self.advance(); // consume opening quote
        let mut out = String::new();
        loop {
            match self.peek() {
                None => {
                    return Err(LexError {
                        message: "unterminated string literal".into(),
                        line,
                        col,
                    });
                }
                Some(c) if c == quote => {
                    self.advance();
                    break;
                }
                Some('\\') => {
                    self.advance();
                    if let Some(escaped) = self.advance() {
                        match escaped {
                            'n' => out.push('\n'),
                            't' => out.push('\t'),
                            '\\' => out.push('\\'),
                            other if other == quote => out.push(quote),
                            other => out.push(other),
                        }
                    }
                }
                Some(c) => {
                    out.push(c);
                    self.advance();
                }
            }
        }
        Ok(out)
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            let (line, col) = (self.line, self.col);
            let Some(c) = self.peek() else {
                tokens.push(Token {
                    kind: TokenKind::Eof,
                    line,
                    col,
                });
                break;
            };

            // String literals: '...' and "..." are fully interchangeable.
            if c == '\'' || c == '"' {
                let s = self.read_string(c)?;
                tokens.push(Token {
                    kind: TokenKind::Str(s),
                    line,
                    col,
                });
                continue;
            }

            // Longest-match custom operators first.
            if self.starts_with("<{") {
                self.advance();
                self.advance();
                tokens.push(Token {
                    kind: TokenKind::LeftVarBracket,
                    line,
                    col,
                });
                continue;
            }
            if self.starts_with(">>") {
                self.advance();
                self.advance();
                tokens.push(Token {
                    kind: TokenKind::OpAssign,
                    line,
                    col,
                });
                continue;
            }
            if self.starts_with("</") {
                self.advance();
                self.advance();
                tokens.push(Token {
                    kind: TokenKind::LtSlash,
                    line,
                    col,
                });
                continue;
            }
            if self.starts_with("/>") {
                self.advance();
                self.advance();
                tokens.push(Token {
                    kind: TokenKind::SlashGt,
                    line,
                    col,
                });
                continue;
            }
            if self.starts_with("<=") {
                self.advance();
                self.advance();
                tokens.push(Token {
                    kind: TokenKind::LtEq,
                    line,
                    col,
                });
                continue;
            }
            if self.starts_with(">=") {
                self.advance();
                self.advance();
                tokens.push(Token {
                    kind: TokenKind::GtEq,
                    line,
                    col,
                });
                continue;
            }
            if self.starts_with("!=") {
                self.advance();
                self.advance();
                tokens.push(Token {
                    kind: TokenKind::BangEq,
                    line,
                    col,
                });
                continue;
            }
            if self.starts_with("::") {
                self.advance();
                self.advance();
                tokens.push(Token {
                    kind: TokenKind::ColonColon,
                    line,
                    col,
                });
                continue;
            }
            if self.starts_with("&&") {
                self.advance();
                self.advance();
                tokens.push(Token {
                    kind: TokenKind::AmpAmp,
                    line,
                    col,
                });
                continue;
            }
            if self.starts_with("&") {
                self.advance();
                tokens.push(Token {
                    kind: TokenKind::Amp,
                    line,
                    col,
                });
                continue;
            }
            if self.starts_with("||") {
                self.advance();
                self.advance();
                tokens.push(Token {
                    kind: TokenKind::PipePipe,
                    line,
                    col,
                });
                continue;
            }
            if self.starts_with("|") {
                self.advance();
                tokens.push(Token {
                    kind: TokenKind::Pipe,
                    line,
                    col,
                });
                continue;
            }

            // Type-tag sigils: #n / #w / #f. Read directly off the second
            // character the same way `if>`/`craft<` fuse a keyword with its
            // trailing punctuation below, rather than lexing `#` and the
            // letter as two separate tokens.
            if c == '#' {
                let kind = match self.peek_at(1) {
                    Some('n') => TokenKind::TypeNum,
                    Some('w') => TokenKind::TypeWord,
                    Some('f') => TokenKind::TypeFlag,
                    Some('t') => TokenKind::TypeGeneric,
                    other => {
                        return Err(LexError {
                            message: format!(
                                "unknown type sigil '#{}': expected #n (Num), #w (Word), #f (Flag), or #t (a litter/breed's own generic type parameter)",
                                other.map(String::from).unwrap_or_default()
                            ),
                            line,
                            col,
                        });
                    }
                };
                self.advance();
                self.advance();
                tokens.push(Token { kind, line, col });
                continue;
            }

            // Word-like tokens: identifiers, keywords, and the fused
            // keyword+punctuation forms (`if>`, `orif>`, `else>`, `craft<`).
            if c.is_alphabetic() || c == '_' {
                let mut word = String::new();
                while let Some(c) = self.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        word.push(c);
                        self.advance();
                    } else {
                        break;
                    }
                }
                let kind = match word.as_str() {
                    "func" => TokenKind::KeywordFunc,
                    "return" => TokenKind::KeywordReturn,
                    "spin" => TokenKind::KeywordSpin,
                    "purr" => TokenKind::KeywordPurr,
                    "private" => TokenKind::KeywordPrivate,
                    "import" => TokenKind::KeywordImport,
                    "from" => TokenKind::KeywordFrom,
                    "export" => TokenKind::KeywordExport,
                    "hold" => TokenKind::KeywordHold,
                    "litter" => TokenKind::KeywordLitter,
                    "breed" => TokenKind::KeywordBreed,
                    "claw" => TokenKind::KeywordClaw,
                    "bare" => TokenKind::KeywordBare,
                    "for" => TokenKind::KeywordFor,
                    "pounce" if self.peek() == Some('>') => {
                        self.advance();
                        TokenKind::KeywordPounce
                    }
                    "if" if self.peek() == Some('>') => {
                        self.advance();
                        TokenKind::KeywordIf
                    }
                    "orif" if self.peek() == Some('>') => {
                        self.advance();
                        TokenKind::KeywordOrif
                    }
                    "else" if self.peek() == Some('>') => {
                        self.advance();
                        TokenKind::KeywordElse
                    }
                    "craft" if self.peek() == Some('<') => {
                        self.advance();
                        TokenKind::KeywordCraft
                    }
                    "warn" if self.peek() == Some('<') => {
                        self.advance();
                        TokenKind::KeywordWarn
                    }
                    "error" if self.peek() == Some('<') => {
                        self.advance();
                        TokenKind::KeywordError
                    }
                    "yes" if self.peek() == Some('>') => {
                        self.advance();
                        TokenKind::Bool(true)
                    }
                    "no" if self.peek() == Some('>') => {
                        self.advance();
                        TokenKind::Bool(false)
                    }
                    _ => TokenKind::Ident(word),
                };
                tokens.push(Token { kind, line, col });
                continue;
            }

            // Numbers
            if c.is_ascii_digit() {
                let mut num = String::new();
                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() || c == '.' {
                        num.push(c);
                        self.advance();
                    } else {
                        break;
                    }
                }
                let value: f64 = num.parse().map_err(|_| LexError {
                    message: format!("invalid number literal '{num}'"),
                    line,
                    col,
                })?;
                tokens.push(Token {
                    kind: TokenKind::Number(value),
                    line,
                    col,
                });
                continue;
            }

            // Single-character tokens (with `<` / `>` as the fallback for
            // the two-char forms above).
            let kind = match c {
                '<' => TokenKind::Lt,
                '>' => TokenKind::Gt,
                '(' => TokenKind::LParen,
                ')' => TokenKind::RParen,
                '{' => TokenKind::LBrace,
                '}' => TokenKind::RBrace,
                '[' => TokenKind::LBracket,
                ']' => TokenKind::RBracket,
                ',' => TokenKind::Comma,
                '.' => TokenKind::Dot,
                ':' => TokenKind::Colon,
                '=' => TokenKind::Eq,
                '+' => TokenKind::Plus,
                '-' => TokenKind::Minus,
                '*' => TokenKind::Star,
                '/' => TokenKind::Slash,
                '!' => TokenKind::Bang,
                other => {
                    return Err(LexError {
                        message: format!("unexpected character '{other}'"),
                        line,
                        col,
                    });
                }
            };
            self.advance();
            tokens.push(Token { kind, line, col });
        }

        Ok(tokens)
    }
}

/// Convenience wrapper used by `main.rs`.
pub fn tokenize(src: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(src).tokenize()
}
