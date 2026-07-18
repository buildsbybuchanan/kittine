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
        while matches!(self.peek().kind, TokenKind::KeywordImport) {
            imports.push(self.parse_import()?);
        }

        let mut items = Vec::new();
        while !matches!(self.peek().kind, TokenKind::Eof) {
            let item = match self.peek().kind {
                TokenKind::KeywordFunc => Item::Component(self.parse_component()?),
                TokenKind::KeywordPurr => Item::Function(self.parse_function()?),
                _ => return Err(self.err("expected 'func' or 'purr' at the top level")),
            };
            items.push(item);
        }
        Ok(Program { imports, items })
    }

    /// Parses `import { Name, Name2 } from 'path/to/file.kitty'`.
    fn parse_import(&mut self) -> PResult<Import> {
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
        Ok(Import { names, path })
    }

    /// Parses a signature-position type tag `<<Type>>` (no value follows —
    /// used for parameter and return-type annotations, as opposed to
    /// [`Parser::parse_type_tag`] which wraps a value).
    /// Parses `<<Type>>` or `<<Type[]>>` (an array of `Type`), returning
    /// `"Type"` or `"Type[]"` respectively — the `[]` suffix mirrors the
    /// array-literal syntax (`[expr, ..]`) rather than inventing a new
    /// bracket convention.
    fn parse_signature_type(&mut self) -> PResult<String> {
        self.expect(TokenKind::Lt)?;
        self.expect(TokenKind::Lt)?;
        let mut ty = self.expect_ident()?;
        if !matches!(ty.as_str(), "Num" | "Word" | "Flag") {
            return Err(self.err(format!(
                "unknown type tag '<<{ty}>>': expected one of Num, Word, Flag (optionally followed by [] for an array)"
            )));
        }
        if matches!(self.peek().kind, TokenKind::LBracket) {
            self.advance();
            self.expect(TokenKind::RBracket)?;
            ty.push_str("[]");
        }
        self.expect(TokenKind::OpAssign)?;
        Ok(ty)
    }

    /// Parses a single `<<Type>> name` parameter — or the special
    /// untyped `children` shorthand, which needs no type tag since it
    /// isn't one of `Num`/`Word`/`Flag`: it's Leptos's `Children`, bound to
    /// whatever JSX content a caller nests inside `<ThisComponent>..</ThisComponent>`.
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
        let ty = self.parse_signature_type()?;
        let name = self.expect_ident()?;
        Ok(Param { ty, name })
    }

    /// Parses `(<<Type>> name, <<Type>> name, ..)`.
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

    /// Parses `purr name(<<Type>> param, ..) <<ReturnType>> { stmt* return (expr) }`.
    fn parse_function(&mut self) -> PResult<Function> {
        self.expect(TokenKind::KeywordPurr)?;
        let name = self.expect_ident()?;
        let params = self.parse_param_list()?;
        let return_type = self.parse_signature_type()?;
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
        })
    }

    fn parse_component(&mut self) -> PResult<Component> {
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
        })
    }

    // ---- statements --------------------------------------------------------

    fn parse_stmt(&mut self) -> PResult<Stmt> {
        match self.peek().kind {
            TokenKind::LeftVarBracket => self.parse_var_assign_stmt(),
            TokenKind::KeywordCraft => self.parse_craft_stmt(),
            TokenKind::KeywordIf => self.parse_if_stmt(),
            TokenKind::KeywordSpin => self.parse_spin_stmt(),
            _ => {
                let expr = self.parse_expr()?;
                Ok(Stmt::Expr(expr))
            }
        }
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
        let value = self.parse_equality_no_trailing_gt()?;
        self.expect(TokenKind::Gt)?;
        Ok(Stmt::Craft { value })
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

    /// A condition always starts with `<{name}>`. `<{name}> >> expr` is
    /// interpreted as an equality test rather than an assignment; any other
    /// comparison operator (`<`, `<=`, `>`, `>=`, `!=`) following the bare
    /// `<{name}>` read works the same way, via [`Parser::parse_comparison_tail`].
    fn parse_condition(&mut self) -> PResult<Expr> {
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
    // Precedence, low to high: comparison (`>>` `<` `<=` `>` `>=` `!=`)
    // < `+ -` < `* /` < unary < primary.

    fn parse_expr(&mut self) -> PResult<Expr> {
        self.parse_equality()
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
        self.parse_primary()
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
                if matches!(self.peek().kind, TokenKind::LParen) {
                    self.parse_call(name)
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            TokenKind::LParen => {
                self.advance();
                let inner = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                Ok(inner)
            }
            TokenKind::LBracket => self.parse_array_literal(),
            TokenKind::Lt => self.parse_type_tag(),
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

    /// Parses `<<Type>> expr`, the idiomatic Kittine type tag. `<<` and `>>`
    /// are not fused tokens — they fall out naturally from two `Lt`s and the
    /// existing `OpAssign` (`>>`) token, disambiguated purely by being read
    /// from inside an expression instead of a `<{name}>` or JSX position.
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
            return Err(ParseError {
                message: format!("type tag '<<{ty}>>' does not match the value that follows"),
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

        Ok(JsxNode::Spin { item, list, body })
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
    Parser::new(tokens).parse_program()
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
                | (_, Expr::Ident(_) | Expr::VarRead(_) | Expr::Call { .. } | Expr::Binary { .. } | Expr::InlineAssign { .. } | Expr::Typed { .. })
        )
    })
}
