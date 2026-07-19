//! Recursive-descent parser that turns a token stream into a Kittine [`Program`].
//!
//! Two things make this grammar unusual compared to a typical toy language:
//!
//! 1. `func` bodies are brace-delimited, but the `if>` / `orif>` / `else>`
//!    control-flow blocks nested inside them are indentation-delimited
//!    (Python-style), with no braces at all. We handle this with the
//!    classic "off-side rule" trick: every statement token carries its
//!    source column, an `if>`/`orif>`/`else>` block captures the column of
//!    its own keyword as `base_col`, and keeps consuming statements while
//!    the next statement's column is greater than `base_col`. Sibling
//!    `orif>`/`else>` keywords are recognized because they sit back at
//!    exactly `base_col`.
//! 2. `<{name}> >> expr` is reused in three different grammatical
//!    positions: a top-level statement (declare-or-mutate a signal), an
//!    `if>` condition (equality test), and an inline JSX event-handler
//!    expression. All three share one primitive, [`Parser::parse_var_bracket_expr`],
//!    which is why the AST has a single `Expr::InlineAssign` node that
//!    callers reinterpret based on where they found it.

use crate::ast::*;
use crate::lexer::{Token, TokenKind};
use std::fmt;

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "parse error at {}:{}: {}",
            self.line, self.col, self.message
        )
    }
}

type PResult<T> = Result<T, ParseError>;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    // ---- token stream helpers -------------------------------------------------

    fn peek(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.pos.min(self.tokens.len() - 1)].clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        tok
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }

    /// Looks at the token after the current one without consuming anything.
    fn peek_next(&self) -> &TokenKind {
        let idx = (self.pos + 1).min(self.tokens.len() - 1);
        &self.tokens[idx].kind
    }

    fn err(&self, message: impl Into<String>) -> ParseError {
        let tok = self.peek();
        ParseError {
            message: message.into(),
            line: tok.line,
            col: tok.col,
        }
    }

    fn expect(&mut self, kind: TokenKind) -> PResult<Token> {
        if self.check(&kind) {
            Ok(self.advance())
        } else {
            Err(self.err(format!("expected '{kind}', found '{}'", self.peek().kind)))
        }
    }

    fn expect_ident(&mut self) -> PResult<String> {
        match self.peek().kind.clone() {
            TokenKind::Ident(name) => {
                self.advance();
                Ok(name)
            }
            other => Err(self.err(format!("expected identifier, found '{other}'"))),
        }
    }

    // ---- top level --------------------------------------------------------

    pub fn parse_program(&mut self) -> PResult<Program> {
        let mut imports = Vec::new();
        while matches!(
            self.peek().kind,
            TokenKind::KeywordImport | TokenKind::KeywordExport
        ) {
            imports.push(self.parse_import()?);
        }

        let mut items = Vec::new();
        while !matches!(self.peek().kind, TokenKind::Eof) {
            // `private` is optional and precedes `func`/`purr`; see
            // `Component::is_private`/`Function::is_private`.
            let is_private = matches!(self.peek().kind, TokenKind::KeywordPrivate);
            if is_private {
                self.advance();
            }
            let item = match self.peek().kind {
                TokenKind::KeywordFunc => Item::Component(self.parse_component(is_private)?),
                TokenKind::KeywordPurr => Item::Function(self.parse_function(is_private)?),
                _ => return Err(self.err("expected 'func' or 'purr' (optionally preceded by 'private') at the top level")),
            };
            items.push(item);
        }
        Ok(Program { imports, items })
    }

    /// Parses `(export)? import { Name, Name2 } from 'path/to/file.kitty'`.
    /// A leading `export` re-exports `names` under this file's own name —
    /// see `ast::Import::is_export`.
    fn parse_import(&mut self) -> PResult<Import> {
        let is_export = matches!(self.peek().kind, TokenKind::KeywordExport);
        if is_export {
            self.advance();
        }
        self.expect(TokenKind::KeywordImport)?;
        self.expect(TokenKind::LBrace)?;
        let mut names = Vec::new();
        if !matches!(self.peek().kind, TokenKind::RBrace) {
            names.push(self.expect_ident()?);
            while matches!(self.peek().kind, TokenKind::Comma) {
                self.advance();
                names.push(self.expect_ident()?);
            }
        }
        self.expect(TokenKind::RBrace)?;
        self.expect(TokenKind::KeywordFrom)?;
        let path = match self.peek().kind.clone() {
            TokenKind::Str(s) => {
                self.advance();
                s
            }
            other => {
                return Err(self.err(format!(
                    "expected a string path after 'from', found '{other}'"
                )));
            }
        };
        Ok(Import { names, path, is_export })
    }

    /// Parses a signature-position type tag: `#n`/`#w`/`#f`, no value
    /// follows (used for parameter and return-type annotations, as opposed
    /// to [`Parser::parse_type_tag`] which wraps a value). Also accepts a
    /// trailing `[]` for an array of that type (`#w[]`), returning
    /// `"Word[]"` etc. — the `[]` suffix mirrors the array-literal syntax
    /// (`[expr, ..]`) rather than inventing a new bracket convention. No
    /// closing delimiter is needed (unlike the retired `<<Type>>` form) —
    /// the sigil itself carries the type, so the tag is at most 4
    /// characters (`#w[]`) even for an array.
    fn parse_signature_type(&mut self) -> PResult<String> {
        let mut ty = match self.peek().kind {
            TokenKind::TypeNum => "Num".to_string(),
            TokenKind::TypeWord => "Word".to_string(),
            TokenKind::TypeFlag => "Flag".to_string(),
            _ => {
                return Err(self.err(format!(
                    "expected a type tag (#n for Num, #w for Word, or #f for Flag), found '{}'",
                    self.peek().kind
                )));
            }
        };
        self.advance();
        if matches!(self.peek().kind, TokenKind::LBracket) {
            self.advance();
            self.expect(TokenKind::RBracket)?;
            ty.push_str("[]");
        }
        Ok(ty)
    }

    /// `true` if the current token is a type-tag sigil (`#n`/`#w`/`#f`) —
    /// used to decide whether a param/return-type position has an explicit
    /// tag at all, since both are now optional (see `infer`).
    fn peek_is_type_sigil(&self) -> bool {
        matches!(
            self.peek().kind,
            TokenKind::TypeNum | TokenKind::TypeWord | TokenKind::TypeFlag
        )
    }

    /// Parses a single `(#t)? name` parameter — or the special untyped
    /// `children` shorthand, which needs no type tag since it isn't one of
    /// `Num`/`Word`/`Flag`: it's Leptos's `Children`, bound to whatever JSX
    /// content a caller nests inside `<ThisComponent>..</ThisComponent>`.
    /// A scalar (non-array) tag is optional — see `infer::apply`, which
    /// fills `ty` in with `Num`/`Word`/`Flag` after parsing by looking at
    /// how the name is used in the body, so `greet(name)` needs no sigil
    /// at all when the body already makes the type obvious. An array
    /// param (`#n[]`/`#w[]`/`#f[]`) still needs its tag written out —
    /// inference doesn't reach into array element types.
    fn parse_param(&mut self) -> PResult<Param> {
        if let TokenKind::Ident(name) = self.peek().kind.clone()
            && name == "children"
        {
            self.advance();
            return Ok(Param {
                ty: "Children".to_string(),
                name,
            });
        }
        let ty = if self.peek_is_type_sigil() {
            self.parse_signature_type()?
        } else {
            String::new()
        };
        let name = self.expect_ident()?;
        Ok(Param { ty, name })
    }

    /// Parses `(#t name, #t name, ..)`.
    fn parse_param_list(&mut self) -> PResult<Vec<Param>> {
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        if !matches!(self.peek().kind, TokenKind::RParen) {
            params.push(self.parse_param()?);
            while matches!(self.peek().kind, TokenKind::Comma) {
                self.advance();
                params.push(self.parse_param()?);
            }
        }
        self.expect(TokenKind::RParen)?;
        Ok(params)
    }

    /// Parses `purr name((#t)? param, ..) (#t)? { stmt* return (expr) }`.
    /// The return-type tag is optional too, same reasoning as
    /// `parse_param` — `infer::apply` fills it in from the shape of the
    /// `return (expr)` once parsing is done.
    fn parse_function(&mut self, is_private: bool) -> PResult<Function> {
        self.expect(TokenKind::KeywordPurr)?;
        let name = self.expect_ident()?;
        let params = self.parse_param_list()?;
        let return_type = if self.peek_is_type_sigil() {
            self.parse_signature_type()?
        } else {
            String::new()
        };
        self.expect(TokenKind::LBrace)?;

        let mut body = Vec::new();
        let mut return_expr = None;
        loop {
            match self.peek().kind {
                TokenKind::RBrace => {
                    self.advance();
                    break;
                }
                TokenKind::KeywordReturn => {
                    self.advance();
                    self.expect(TokenKind::LParen)?;
                    let expr = self.parse_expr()?;
                    self.expect(TokenKind::RParen)?;
                    return_expr = Some(expr);
                }
                TokenKind::Eof => {
                    return Err(self.err("unexpected end of file inside 'purr' function body"));
                }
                _ => {
                    body.push(self.parse_stmt()?);
                }
            }
        }

        let return_expr = return_expr
            .ok_or_else(|| self.err(format!("'purr {name}' must end with a 'return ( expr )'")))?;

        Ok(Function {
            name,
            params,
            return_type,
            body,
            return_expr,
            is_private,
        })
    }

    fn parse_component(&mut self, is_private: bool) -> PResult<Component> {
        self.expect(TokenKind::KeywordFunc)?;
        let name = self.expect_ident()?;
        let params = self.parse_param_list()?;
        self.expect(TokenKind::LBrace)?;

        let mut body = Vec::new();
        let mut return_view = None;
        loop {
            match self.peek().kind {
                TokenKind::RBrace => {
                    self.advance();
                    break;
                }
                TokenKind::KeywordReturn => {
                    self.advance();
                    self.expect(TokenKind::LParen)?;
                    let node = self.parse_jsx_node()?;
                    self.expect(TokenKind::RParen)?;
                    return_view = Some(node);
                }
                TokenKind::Eof => {
                    return Err(self.err("unexpected end of file inside component body"));
                }
                _ => {
                    body.push(self.parse_stmt()?);
                }
            }
        }

        Ok(Component {
            name,
            params,
            body,
            return_view,
            is_private,
        })
    }

    // ---- statements --------------------------------------------------------

    fn parse_stmt(&mut self) -> PResult<Stmt> {
        match self.peek().kind {
            TokenKind::LeftVarBracket => self.parse_var_assign_stmt(),
            TokenKind::KeywordCraft => self.parse_craft_stmt(),
            TokenKind::KeywordIf => self.parse_if_stmt(),
            TokenKind::KeywordSpin => self.parse_spin_stmt(),
            TokenKind::KeywordHold => self.parse_hold_stmt(),
            _ => {
                let expr = self.parse_expr()?;
                Ok(Stmt::Expr(expr))
            }
        }
    }

    /// Parses `hold name >> expr` — a plain (non-reactive) local binding.
    /// See `ast::Stmt::Hold`.
    fn parse_hold_stmt(&mut self) -> PResult<Stmt> {
        self.expect(TokenKind::KeywordHold)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::OpAssign)?;
        let value = self.parse_expr()?;
        Ok(Stmt::Hold { name, value })
    }

    /// Parses `spin<{item}> in list }{ stmt* }{`. The `}{` fence is not a
    /// single token — it is the ordinary `}` / `{` pair, read contextually
    /// as a matched pair of loop-body markers rather than as a brace block,
    /// so it never collides with `RBrace`/`LBrace` used elsewhere (e.g. two
    /// adjacent JSX `{ expr }` interpolations).
    fn parse_spin_stmt(&mut self) -> PResult<Stmt> {
        self.expect(TokenKind::KeywordSpin)?;
        self.expect(TokenKind::LeftVarBracket)?;
        let item = self.expect_ident()?;
        self.expect(TokenKind::RBrace)?;
        self.expect(TokenKind::Gt)?;

        let in_word = self.expect_ident()?;
        if in_word != "in" {
            return Err(self.err(format!("expected 'in' after 'spin<{{{item}}}>', found '{in_word}'")));
        }

        let list = self.parse_expr()?;
        self.expect_fence()?;

        let mut body = Vec::new();
        loop {
            if matches!(self.peek().kind, TokenKind::Eof) {
                return Err(self.err("unexpected end of file inside 'spin' loop body"));
            }
            if matches!(self.peek().kind, TokenKind::RBrace)
                && matches!(self.peek_next(), TokenKind::LBrace)
            {
                self.expect_fence()?;
                break;
            }
            body.push(self.parse_stmt()?);
        }

        Ok(Stmt::Spin { item, list, body })
    }

    /// Consumes the `}{` loop fence: a plain `}` immediately followed by `{`.
    fn expect_fence(&mut self) -> PResult<()> {
        self.expect(TokenKind::RBrace)?;
        self.expect(TokenKind::LBrace)?;
        Ok(())
    }

    fn parse_var_assign_stmt(&mut self) -> PResult<Stmt> {
        match self.parse_var_bracket_expr()? {
            Expr::InlineAssign { name, value } => Ok(Stmt::VarAssign {
                name,
                value: *value,
            }),
            _ => Err(self.err("expected '>>' after '<{ .. }>' in a statement")),
        }
    }

    fn parse_craft_stmt(&mut self) -> PResult<Stmt> {
        self.expect(TokenKind::KeywordCraft)?;
        // Not `parse_expr()`: a bare `>` comparison at the top level here
        // would be indistinguishable from `craft<...>`'s own closing `>`.
        // Wrap a `>` comparison in parens (`craft<(age > 18)>`) to use one —
        // parens are self-delimiting via `)`, so there's no ambiguity once
        // the comparison is inside them.
        let value = self.parse_or_no_trailing_gt()?;
        self.expect(TokenKind::Gt)?;
        Ok(Stmt::Craft { value })
    }

    /// Like [`Parser::parse_or`], but built on
    /// [`Parser::parse_equality_no_trailing_gt`] instead of
    /// [`Parser::parse_equality`] — for `craft<expr>`'s argument, where a
    /// bare trailing `>` is ambiguous with craft's own closing bracket.
    /// `&&`/`||` never touch a bare `>`, so combining restricted atoms this
    /// way introduces no new ambiguity.
    fn parse_or_no_trailing_gt(&mut self) -> PResult<Expr> {
        let mut left = self.parse_and_no_trailing_gt()?;
        while matches!(self.peek().kind, TokenKind::PipePipe) {
            self.advance();
            let right = self.parse_and_no_trailing_gt()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinOp::Or,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and_no_trailing_gt(&mut self) -> PResult<Expr> {
        let mut left = self.parse_equality_no_trailing_gt()?;
        while matches!(self.peek().kind, TokenKind::AmpAmp) {
            self.advance();
            let right = self.parse_equality_no_trailing_gt()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinOp::And,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    /// Parses `if> cond <block> (orif> cond <block>)* (else> <block>)?`
    /// using indentation to delimit each block, per the module doc comment.
    fn parse_if_stmt(&mut self) -> PResult<Stmt> {
        let if_tok = self.expect(TokenKind::KeywordIf)?;
        let base_col = if_tok.col;

        let cond = self.parse_condition()?;
        let body = self.parse_indented_block(base_col)?;
        let mut branches = vec![(cond, body)];
        let mut else_body = None;

        loop {
            let tok = self.peek().clone();
            match tok.kind {
                TokenKind::KeywordOrif if tok.col == base_col => {
                    self.advance();
                    let cond = self.parse_condition()?;
                    let body = self.parse_indented_block(base_col)?;
                    branches.push((cond, body));
                }
                TokenKind::KeywordElse if tok.col == base_col => {
                    self.advance();
                    let body = self.parse_indented_block(base_col)?;
                    else_body = Some(body);
                    break;
                }
                _ => break,
            }
        }

        Ok(Stmt::If {
            branches,
            else_body,
        })
    }

    /// A condition is one or more `<{name}> cmp value` atoms combined with
    /// `&&` / `||` (`&&` binds tighter, same as most languages — `a && b ||
    /// c` reads as `(a && b) || c`). Each atom always starts with
    /// `<{name}>`; `<{name}> >> expr` is interpreted as an equality test
    /// rather than an assignment, and any other comparison operator (`<`,
    /// `<=`, `>`, `>=`, `!=`) following the bare `<{name}>` read works the
    /// same way, via [`Parser::parse_comparison_tail`].
    fn parse_condition(&mut self) -> PResult<Expr> {
        let mut left = self.parse_condition_and()?;
        while matches!(self.peek().kind, TokenKind::PipePipe) {
            self.advance();
            let right = self.parse_condition_and()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinOp::Or,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_condition_and(&mut self) -> PResult<Expr> {
        let mut left = self.parse_condition_atom()?;
        while matches!(self.peek().kind, TokenKind::AmpAmp) {
            self.advance();
            let right = self.parse_condition_atom()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinOp::And,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    /// A single `<{name}> cmp_op value` condition atom.
    fn parse_condition_atom(&mut self) -> PResult<Expr> {
        match self.parse_var_bracket_expr()? {
            Expr::InlineAssign { name, value } => Ok(Expr::Binary {
                left: Box::new(Expr::VarRead(name)),
                op: BinOp::Eq,
                right: value,
            }),
            other => self.parse_comparison_tail(other),
        }
    }

    fn parse_indented_block(&mut self, base_col: usize) -> PResult<Vec<Stmt>> {
        let mut stmts = Vec::new();
        loop {
            let tok = self.peek();
            if matches!(tok.kind, TokenKind::Eof | TokenKind::RBrace) {
                break;
            }
            if tok.col <= base_col {
                break;
            }
            stmts.push(self.parse_stmt()?);
        }
        if stmts.is_empty() {
            return Err(self.err("expected an indented block"));
        }
        Ok(stmts)
    }

    // ---- expressions --------------------------------------------------------
    //
    // Precedence, low to high: `||` < `&&` < comparison (`>>` `<` `<=` `>`
    // `>=` `!=`) < `+ -` < `* /` < unary < primary.

    fn parse_expr(&mut self) -> PResult<Expr> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> PResult<Expr> {
        let mut left = self.parse_and()?;
        while matches!(self.peek().kind, TokenKind::PipePipe) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinOp::Or,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> PResult<Expr> {
        let mut left = self.parse_equality()?;
        while matches!(self.peek().kind, TokenKind::AmpAmp) {
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinOp::And,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> PResult<Expr> {
        let left = self.parse_additive()?;
        self.parse_comparison_tail(left)
    }

    /// Like [`Parser::parse_equality`], but never consumes a trailing bare
    /// `>` as a greater-than comparison — used for `craft<expr>`'s
    /// argument, where craft's own closing `>` would otherwise be
    /// ambiguous with one. A `>` comparison nested inside parens (e.g.
    /// `craft<(age > 18)>`) is unaffected: the parens are bounded by their
    /// own `)`, fully resolved via the ordinary [`Parser::parse_expr`]
    /// before this check ever runs.
    fn parse_equality_no_trailing_gt(&mut self) -> PResult<Expr> {
        let left = self.parse_additive()?;
        let op = match self.peek().kind {
            TokenKind::OpAssign => BinOp::Eq,
            TokenKind::Lt => BinOp::Lt,
            TokenKind::LtEq => BinOp::Le,
            TokenKind::GtEq => BinOp::Ge,
            TokenKind::BangEq => BinOp::Ne,
            _ => return Ok(left),
        };
        self.advance();
        let right = self.parse_additive()?;
        Ok(Expr::Binary {
            left: Box::new(left),
            op,
            right: Box::new(right),
        })
    }

    /// Given an already-parsed left-hand side, checks for a trailing
    /// comparison operator and folds it into a `Binary` — shared by the
    /// general expression grammar ([`Parser::parse_equality`]) and `if>`/
    /// `orif>` conditions ([`Parser::parse_condition`]).
    fn parse_comparison_tail(&mut self, left: Expr) -> PResult<Expr> {
        let op = match self.peek().kind {
            TokenKind::OpAssign => BinOp::Eq,
            TokenKind::Lt => BinOp::Lt,
            TokenKind::LtEq => BinOp::Le,
            TokenKind::Gt => BinOp::Gt,
            TokenKind::GtEq => BinOp::Ge,
            TokenKind::BangEq => BinOp::Ne,
            _ => return Ok(left),
        };
        self.advance();
        let right = self.parse_additive()?;
        Ok(Expr::Binary {
            left: Box::new(left),
            op,
            right: Box::new(right),
        })
    }

    fn parse_additive(&mut self) -> PResult<Expr> {
        let mut left = self.parse_term()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_term()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> PResult<Expr> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> PResult<Expr> {
        if matches!(self.peek().kind, TokenKind::Minus) {
            self.advance();
            let inner = self.parse_unary()?;
            return Ok(Expr::Binary {
                left: Box::new(Expr::Number(0.0)),
                op: BinOp::Sub,
                right: Box::new(inner),
            });
        }
        self.parse_postfix()
    }

    /// A primary, followed by zero or more `.method(arg, ..)` suffixes —
    /// for calling real Rust/Leptos APIs Kittine has no dedicated syntax
    /// for (e.g. `use_params_map().get().get("id").unwrap_or_default()`).
    /// Kittine doesn't track receiver types, so any method name parses;
    /// Rust's own type checker is the source of truth on whether the call
    /// is valid.
    fn parse_postfix(&mut self) -> PResult<Expr> {
        let mut expr = self.parse_primary()?;
        loop {
            if matches!(self.peek().kind, TokenKind::Dot) {
                self.advance();
                let method = self.expect_ident()?;
                let args = self.parse_paren_arg_list()?;
                expr = Expr::MethodCall {
                    receiver: Box::new(expr),
                    method,
                    args,
                };
            } else if matches!(self.peek().kind, TokenKind::LParen) {
                // Calling the *result* of `expr` (not a bare named
                // function — that's already handled inside `parse_primary`
                // when it's an `Ident` immediately followed by `(`). This
                // is what lets `use_navigate()('/', ..)` — calling the
                // closure `use_navigate()` returns — parse at all.
                let args = self.parse_paren_arg_list()?;
                expr = Expr::CallResult {
                    callee: Box::new(expr),
                    args,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    /// Parses `(arg, arg, ..)` — shared by [`Parser::parse_postfix`]'s two
    /// call-shaped suffixes.
    fn parse_paren_arg_list(&mut self) -> PResult<Vec<Expr>> {
        self.expect(TokenKind::LParen)?;
        let mut args = Vec::new();
        if !matches!(self.peek().kind, TokenKind::RParen) {
            args.push(self.parse_expr()?);
            while matches!(self.peek().kind, TokenKind::Comma) {
                self.advance();
                args.push(self.parse_expr()?);
            }
        }
        self.expect(TokenKind::RParen)?;
        Ok(args)
    }

    fn parse_primary(&mut self) -> PResult<Expr> {
        match self.peek().kind.clone() {
            TokenKind::LeftVarBracket => self.parse_var_bracket_expr(),
            TokenKind::Number(n) => {
                self.advance();
                Ok(Expr::Number(n))
            }
            TokenKind::Str(s) => {
                self.advance();
                Ok(Expr::Str(s))
            }
            TokenKind::Bool(b) => {
                self.advance();
                Ok(Expr::Bool(b))
            }
            TokenKind::Ident(name) => {
                self.advance();
                if matches!(self.peek().kind, TokenKind::ColonColon) {
                    let mut segments = vec![name];
                    while matches!(self.peek().kind, TokenKind::ColonColon) {
                        self.advance();
                        segments.push(self.expect_ident()?);
                    }
                    Ok(Expr::Path(segments))
                } else if matches!(self.peek().kind, TokenKind::LParen) {
                    self.parse_call(name)
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            TokenKind::LParen => {
                self.advance();
                let first = self.parse_expr()?;
                if matches!(self.peek().kind, TokenKind::Comma) {
                    // A comma makes this a tuple literal, not just a
                    // parenthesized (grouping) expression — needed to
                    // combine leptos_router path segments into one route:
                    // `(StaticSegment('user'), ParamSegment('id'))`.
                    let mut items = vec![first];
                    while matches!(self.peek().kind, TokenKind::Comma) {
                        self.advance();
                        if matches!(self.peek().kind, TokenKind::RParen) {
                            break; // tolerate a trailing comma
                        }
                        items.push(self.parse_expr()?);
                    }
                    self.expect(TokenKind::RParen)?;
                    Ok(Expr::Tuple(items))
                } else {
                    self.expect(TokenKind::RParen)?;
                    Ok(first)
                }
            }
            TokenKind::LBracket => self.parse_array_literal(),
            TokenKind::TypeNum | TokenKind::TypeWord | TokenKind::TypeFlag => {
                self.parse_type_tag()
            }
            other => Err(self.err(format!("unexpected token '{other}' in expression"))),
        }
    }

    /// Parses `(arg, arg, ..)` following an already-consumed function name,
    /// producing `Expr::Call`.
    fn parse_call(&mut self, name: String) -> PResult<Expr> {
        self.expect(TokenKind::LParen)?;
        let mut args = Vec::new();
        if !matches!(self.peek().kind, TokenKind::RParen) {
            args.push(self.parse_expr()?);
            while matches!(self.peek().kind, TokenKind::Comma) {
                self.advance();
                args.push(self.parse_expr()?);
            }
        }
        self.expect(TokenKind::RParen)?;
        Ok(Expr::Call { name, args })
    }

    /// Parses `[expr, expr, ..]`.
    fn parse_array_literal(&mut self) -> PResult<Expr> {
        self.expect(TokenKind::LBracket)?;
        let mut items = Vec::new();
        if !matches!(self.peek().kind, TokenKind::RBracket) {
            items.push(self.parse_expr()?);
            while matches!(self.peek().kind, TokenKind::Comma) {
                self.advance();
                items.push(self.parse_expr()?);
            }
        }
        self.expect(TokenKind::RBracket)?;
        Ok(Expr::Array(items))
    }

    /// Parses `#t expr`, the idiomatic Kittine type tag (`#n`/`#w`/`#f`,
    /// optionally `[]`-suffixed) applied to a value. The sigil is a single
    /// fused token (see `lexer::TokenKind::TypeNum` and friends), so unlike
    /// the retired `<<Type>>` form there's no closing bracket to look for.
    fn parse_type_tag(&mut self) -> PResult<Expr> {
        let start = self.peek().clone();
        let ty = self.parse_signature_type()?;
        let value = self.parse_unary()?;

        let matches_ty = match (ty.as_str(), &value) {
            ("Num", Expr::Number(_)) => true,
            ("Word", Expr::Str(_)) => true,
            ("Flag", Expr::Bool(_)) => true,
            ("Num[]" | "Word[]" | "Flag[]", Expr::Array(items)) => {
                array_elements_match(&ty, items)
            }
            (_, Expr::Number(_) | Expr::Str(_) | Expr::Bool(_) | Expr::Array(_)) => false,
            _ => true, // a variable read or computed expression — trust the annotation
        };
        if !matches_ty {
            let sigil = match ty.as_str() {
                "Num" | "Num[]" => "#n",
                "Word" | "Word[]" => "#w",
                _ => "#f",
            };
            return Err(ParseError {
                message: format!(
                    "type tag '{sigil}' ({ty}) does not match the value that follows"
                ),
                line: start.line,
                col: start.col,
            });
        }

        Ok(Expr::Typed {
            ty,
            value: Box::new(value),
        })
    }

    /// Parses `<{ident}>`, then optionally `>> expr` if present, producing
    /// either a bare `Expr::VarRead` or an `Expr::InlineAssign`.
    fn parse_var_bracket_expr(&mut self) -> PResult<Expr> {
        self.expect(TokenKind::LeftVarBracket)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::RBrace)?;
        self.expect(TokenKind::Gt)?;
        if matches!(self.peek().kind, TokenKind::OpAssign) {
            self.advance();
            let value = self.parse_additive()?;
            Ok(Expr::InlineAssign {
                name,
                value: Box::new(value),
            })
        } else {
            Ok(Expr::VarRead(name))
        }
    }

    // ---- JSX ----------------------------------------------------------------

    fn parse_jsx_node(&mut self) -> PResult<JsxNode> {
        match self.peek().kind.clone() {
            TokenKind::Lt => self.parse_jsx_element(),
            TokenKind::Str(s) => {
                self.advance();
                Ok(JsxNode::Text(s))
            }
            TokenKind::LeftVarBracket => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect(TokenKind::RBrace)?;
                self.expect(TokenKind::Gt)?;
                Ok(JsxNode::VarInterp(name))
            }
            TokenKind::LBrace => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(TokenKind::RBrace)?;
                Ok(JsxNode::ExprInterp(expr))
            }
            TokenKind::KeywordSpin => self.parse_jsx_spin(),
            other => Err(self.err(format!("unexpected token '{other}' in JSX"))),
        }
    }

    /// Parses `spin<{item}> in list }{ jsx_node* }{` in JSX position — the
    /// view-rendering counterpart of [`Parser::parse_spin_stmt`], which
    /// only handles `spin` as an imperative statement.
    fn parse_jsx_spin(&mut self) -> PResult<JsxNode> {
        self.expect(TokenKind::KeywordSpin)?;
        self.expect(TokenKind::LeftVarBracket)?;
        let item = self.expect_ident()?;
        self.expect(TokenKind::RBrace)?;
        self.expect(TokenKind::Gt)?;

        let in_word = self.expect_ident()?;
        if in_word != "in" {
            return Err(self.err(format!(
                "expected 'in' after 'spin<{{{item}}}>', found '{in_word}'"
            )));
        }

        let list = self.parse_expr()?;

        // An optional `key(expr)` clause, right before the `}{` fence.
        // `key` isn't a reserved word — it's recognized contextually here,
        // the same way `in` is above — so it stays available as an
        // ordinary identifier everywhere else.
        let key = if let TokenKind::Ident(name) = self.peek().kind.clone()
            && name == "key"
        {
            self.advance();
            self.expect(TokenKind::LParen)?;
            let key_expr = self.parse_expr()?;
            self.expect(TokenKind::RParen)?;
            Some(key_expr)
        } else {
            None
        };

        self.expect_fence()?;

        let mut body = Vec::new();
        loop {
            if matches!(self.peek().kind, TokenKind::Eof) {
                return Err(self.err("unexpected end of file inside 'spin' view loop"));
            }
            if matches!(self.peek().kind, TokenKind::RBrace)
                && matches!(self.peek_next(), TokenKind::LBrace)
            {
                self.expect_fence()?;
                break;
            }
            body.push(self.parse_jsx_node()?);
        }

        Ok(JsxNode::Spin { item, list, key, body })
    }

    fn parse_jsx_element(&mut self) -> PResult<JsxNode> {
        self.expect(TokenKind::Lt)?;
        let tag = self.expect_ident()?;

        let mut attrs = Vec::new();
        loop {
            match self.peek().kind.clone() {
                TokenKind::Gt => {
                    self.advance();
                    break;
                }
                TokenKind::SlashGt => {
                    self.advance();
                    return Ok(JsxNode::Element {
                        tag,
                        attrs,
                        children: Vec::new(),
                        self_closing: true,
                    });
                }
                TokenKind::Ident(attr_name) => {
                    self.advance();
                    // HTML/ARIA attribute names are routinely kebab-case
                    // (`data-*`, `aria-*`) — not valid identifiers on their
                    // own, so a bare `-` here can only be a continuation of
                    // the attribute name, never subtraction (this is a
                    // dedicated attribute-name position, not an expression).
                    let mut attr_name = attr_name;
                    while matches!(self.peek().kind, TokenKind::Minus) {
                        self.advance();
                        let part = self.expect_ident()?;
                        attr_name.push('-');
                        attr_name.push_str(&part);
                    }
                    self.expect(TokenKind::Eq)?;
                    let value = match self.peek().kind.clone() {
                        TokenKind::LBrace => {
                            self.advance();
                            let expr = self.parse_expr()?;
                            self.expect(TokenKind::RBrace)?;
                            JsxAttrValue::Expr(expr)
                        }
                        TokenKind::Str(s) => {
                            self.advance();
                            JsxAttrValue::Str(s)
                        }
                        other => {
                            return Err(self.err(format!(
                                "expected attribute value after '=', found '{other}'"
                            )));
                        }
                    };
                    attrs.push((attr_name, value));
                }
                other => {
                    return Err(self.err(format!("unexpected token '{other}' in JSX tag")));
                }
            }
        }

        let mut children = Vec::new();
        loop {
            if matches!(self.peek().kind, TokenKind::LtSlash) {
                break;
            }
            children.push(self.parse_jsx_node()?);
        }

        self.expect(TokenKind::LtSlash)?;
        let close_tag = self.expect_ident()?;
        if close_tag != tag {
            return Err(self.err(format!(
                "mismatched closing tag: expected '</{tag}>', found '</{close_tag}>'"
            )));
        }
        self.expect(TokenKind::Gt)?;

        Ok(JsxNode::Element {
            tag,
            attrs,
            children,
            self_closing: false,
        })
    }
}

pub fn parse(tokens: Vec<Token>) -> PResult<Program> {
    let mut program = Parser::new(tokens).parse_program()?;
    crate::infer::apply(&mut program);
    Ok(program)
}

/// `true` if every *literal* element of an array matches an array type
/// tag's element type (`"Num[]"` -> every literal element must be a
/// `Number`, etc.) — a non-literal element (a variable read, a call, a
/// nested computed expression) is trusted rather than checked, same as
/// the scalar type tag check.
fn array_elements_match(array_ty: &str, items: &[Expr]) -> bool {
    let element_ty = match array_ty {
        "Num[]" => "Num",
        "Word[]" => "Word",
        "Flag[]" => "Flag",
        _ => return true,
    };
    items.iter().all(|item| {
        matches!(
            (element_ty, item),
            ("Num", Expr::Number(_))
                | ("Word", Expr::Str(_))
                | ("Flag", Expr::Bool(_))
                | (_, Expr::Ident(_) | Expr::VarRead(_) | Expr::Call { .. } | Expr::MethodCall { .. } | Expr::CallResult { .. } | Expr::Tuple(_) | Expr::Path(_) | Expr::Binary { .. } | Expr::InlineAssign { .. } | Expr::Typed { .. })
        )
    })
}
