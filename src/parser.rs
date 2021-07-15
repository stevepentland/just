use crate::common::*;

use TokenKind::*;

/// Just language parser
///
/// The parser is a (hopefully) straightforward recursive descent parser.
///
/// It uses a few tokens of lookahead to disambiguate different constructs.
///
/// The `expect_*` and `presume_`* methods are similar in that they assert the
/// type of unparsed tokens and consume them. However, upon encountering an
/// unexpected token, the `expect_*` methods return an unexpected token error,
/// whereas the `presume_*` tokens return an internal error.
///
/// The `presume_*` methods are used when the token stream has been inspected in
/// some other way, and thus encountering an unexpected token is a bug in Just,
/// and not a syntax error.
///
/// All methods starting with `parse_*` parse and return a language construct.
///
/// The parser tracks an expected set of tokens as it parses. This set contains
/// all tokens which would have been accepted at the current point in the parse.
/// Whenever the parser tests for a token that would be accepted, but does not
/// find it, it adds that token to the set. When the parser accepts a token, the
/// set is cleared. If the parser finds a token which is unexpected, the
/// contents of the set is printed in the resultant error message.
pub(crate) struct Parser<'tokens, 'src> {
  /// Source tokens
  tokens:   &'tokens [Token<'src>],
  /// Index of the next un-parsed token
  next:     usize,
  /// Current expected tokens
  expected: BTreeSet<TokenKind>,
}

impl<'tokens, 'src> Parser<'tokens, 'src> {
  /// Parse `tokens` into an `Module`
  pub(crate) fn parse(tokens: &'tokens [Token<'src>]) -> CompilationResult<'src, Module<'src>> {
    Self::new(tokens).parse_justfile()
  }

  /// Construct a new Paser from a token stream
  fn new(tokens: &'tokens [Token<'src>]) -> Parser<'tokens, 'src> {
    Parser {
      next: 0,
      expected: BTreeSet::new(),
      tokens,
    }
  }

  fn error(
    &self,
    kind: CompilationErrorKind<'src>,
  ) -> CompilationResult<'src, CompilationError<'src>> {
    Ok(self.next()?.error(kind))
  }

  /// Construct an unexpected token error with the token returned by
  /// `Parser::next`
  fn unexpected_token(&self) -> CompilationResult<'src, CompilationError<'src>> {
    self.error(CompilationErrorKind::UnexpectedToken {
      expected: self.expected.iter().cloned().collect::<Vec<TokenKind>>(),
      found:    self.next()?.kind,
    })
  }

  fn internal_error(
    &self,
    message: impl Into<String>,
  ) -> CompilationResult<'src, CompilationError<'src>> {
    self.error(CompilationErrorKind::Internal {
      message: message.into(),
    })
  }

  /// An iterator over the remaining significant tokens
  fn rest(&self) -> impl Iterator<Item = Token<'src>> + 'tokens {
    self.tokens[self.next..]
      .iter()
      .cloned()
      .filter(|token| token.kind != Whitespace)
  }

  /// The next significant token
  fn next(&self) -> CompilationResult<'src, Token<'src>> {
    if let Some(token) = self.rest().next() {
      Ok(token)
    } else {
      Err(self.internal_error("`Parser::next()` called after end of token stream")?)
    }
  }

  /// Check if the next significant token is of kind `kind`
  fn next_is(&mut self, kind: TokenKind) -> bool {
    self.next_are(&[kind])
  }

  /// Check if the next significant tokens are of kinds `kinds`
  ///
  /// The first token in `kinds` will be added to the expected token set.
  fn next_are(&mut self, kinds: &[TokenKind]) -> bool {
    if let Some(&kind) = kinds.first() {
      self.expected.insert(kind);
    }

    let mut rest = self.rest();
    for kind in kinds {
      match rest.next() {
        Some(token) =>
          if token.kind != *kind {
            return false;
          },
        None => return false,
      }
    }
    true
  }

  /// Get the `n`th next significant token
  fn get(&self, n: usize) -> CompilationResult<'src, Token<'src>> {
    match self.rest().nth(n) {
      Some(token) => Ok(token),
      None => Err(self.internal_error("`Parser::get()` advanced past end of token stream")?),
    }
  }

  /// Advance past one significant token, clearing the expected token set.
  fn advance(&mut self) -> CompilationResult<'src, Token<'src>> {
    self.expected.clear();

    for skipped in &self.tokens[self.next..] {
      self.next += 1;

      if skipped.kind != Whitespace {
        return Ok(*skipped);
      }
    }

    Err(self.internal_error("`Parser::advance()` advanced past end of token stream")?)
  }

  /// Return the next token if it is of kind `expected`, otherwise, return an
  /// unexpected token error
  fn expect(&mut self, expected: TokenKind) -> CompilationResult<'src, Token<'src>> {
    if let Some(token) = self.accept(expected)? {
      Ok(token)
    } else {
      Err(self.unexpected_token()?)
    }
  }

  /// Return an unexpected token error if the next token is not an EOL
  fn expect_eol(&mut self) -> CompilationResult<'src, ()> {
    self.accept(Comment)?;

    if self.next_is(Eof) {
      return Ok(());
    }

    self.expect(Eol).map(|_| ())
  }

  fn expect_keyword(&mut self, expected: Keyword) -> CompilationResult<'src, ()> {
    let identifier = self.expect(Identifier)?;
    let found = identifier.lexeme();

    if expected == found {
      Ok(())
    } else {
      Err(identifier.error(CompilationErrorKind::ExpectedKeyword {
        expected: vec![expected],
        found,
      }))
    }
  }

  /// Return an internal error if the next token is not of kind `Identifier`
  /// with lexeme `lexeme`.
  fn presume_keyword(&mut self, keyword: Keyword) -> CompilationResult<'src, ()> {
    let next = self.advance()?;

    if next.kind != Identifier {
      Err(self.internal_error(format!(
        "Presumed next token would have kind {}, but found {}",
        Identifier, next.kind
      ))?)
    } else if keyword != next.lexeme() {
      Err(self.internal_error(format!(
        "Presumed next token would have lexeme \"{}\", but found \"{}\"",
        keyword,
        next.lexeme(),
      ))?)
    } else {
      Ok(())
    }
  }

  /// Return an internal error if the next token is not of kind `kind`.
  fn presume(&mut self, kind: TokenKind) -> CompilationResult<'src, Token<'src>> {
    let next = self.advance()?;

    if next.kind != kind {
      Err(self.internal_error(format!(
        "Presumed next token would have kind {:?}, but found {:?}",
        kind, next.kind
      ))?)
    } else {
      Ok(next)
    }
  }

  /// Return an internal error if the next token is not one of kinds `kinds`.
  fn presume_any(&mut self, kinds: &[TokenKind]) -> CompilationResult<'src, Token<'src>> {
    let next = self.advance()?;
    if !kinds.contains(&next.kind) {
      Err(self.internal_error(format!(
        "Presumed next token would be {}, but found {}",
        List::or(kinds),
        next.kind
      ))?)
    } else {
      Ok(next)
    }
  }

  /// Accept and return a token of kind `kind`
  fn accept(&mut self, kind: TokenKind) -> CompilationResult<'src, Option<Token<'src>>> {
    if self.next_is(kind) {
      Ok(Some(self.advance()?))
    } else {
      Ok(None)
    }
  }

  /// Return an error if the next token is of kind `forbidden`
  fn forbid<F>(&self, forbidden: TokenKind, error: F) -> CompilationResult<'src, ()>
  where
    F: FnOnce(Token) -> CompilationError,
  {
    let next = self.next()?;

    if next.kind == forbidden {
      Err(error(next))
    } else {
      Ok(())
    }
  }

  /// Accept a token of kind `Identifier` and parse into a `Name`
  fn accept_name(&mut self) -> CompilationResult<'src, Option<Name<'src>>> {
    if self.next_is(Identifier) {
      Ok(Some(self.parse_name()?))
    } else {
      Ok(None)
    }
  }

  fn accepted_keyword(&mut self, keyword: Keyword) -> CompilationResult<'src, bool> {
    let next = self.next()?;

    if next.kind == Identifier && next.lexeme() == keyword.lexeme() {
      self.advance()?;
      Ok(true)
    } else {
      Ok(false)
    }
  }

  /// Accept a dependency
  fn accept_dependency(&mut self) -> CompilationResult<'src, Option<UnresolvedDependency<'src>>> {
    if let Some(recipe) = self.accept_name()? {
      Ok(Some(UnresolvedDependency {
        arguments: Vec::new(),
        recipe,
      }))
    } else if self.accepted(ParenL)? {
      let recipe = self.parse_name()?;

      let mut arguments = Vec::new();

      while !self.accepted(ParenR)? {
        arguments.push(self.parse_expression()?);
      }

      Ok(Some(UnresolvedDependency { recipe, arguments }))
    } else {
      Ok(None)
    }
  }

  /// Accept and return `true` if next token is of kind `kind`
  fn accepted(&mut self, kind: TokenKind) -> CompilationResult<'src, bool> {
    Ok(self.accept(kind)?.is_some())
  }

  /// Parse a justfile, consumes self
  fn parse_justfile(mut self) -> CompilationResult<'src, Module<'src>> {
    fn pop_doc_comment<'src>(
      items: &mut Vec<Item<'src>>,
      eol_since_last_comment: bool,
    ) -> Option<&'src str> {
      if !eol_since_last_comment {
        if let Some(Item::Comment(contents)) = items.last() {
          let doc = Some(contents[1..].trim_start());
          items.pop();
          return doc;
        }
      }

      None
    }

    let mut items = Vec::new();

    let mut eol_since_last_comment = false;

    loop {
      let next = self.next()?;

      if let Some(comment) = self.accept(Comment)? {
        items.push(Item::Comment(comment.lexeme().trim_end()));
        self.expect_eol()?;
        eol_since_last_comment = false;
      } else if self.accepted(Eol)? {
        eol_since_last_comment = true;
      } else if self.accepted(Eof)? {
        break;
      } else if self.next_is(Identifier) {
        match Keyword::from_lexeme(next.lexeme()) {
          Some(Keyword::Alias) =>
            if self.next_are(&[Identifier, Identifier, Equals]) {
              return Err(self.get(2)?.error(CompilationErrorKind::DeprecatedEquals));
            } else if self.next_are(&[Identifier, Identifier, ColonEquals]) {
              items.push(Item::Alias(self.parse_alias()?));
            } else {
              let doc = pop_doc_comment(&mut items, eol_since_last_comment);
              items.push(Item::Recipe(self.parse_recipe(doc, false)?));
            },
          Some(Keyword::Export) =>
            if self.next_are(&[Identifier, Identifier, Equals]) {
              return Err(self.get(2)?.error(CompilationErrorKind::DeprecatedEquals));
            } else if self.next_are(&[Identifier, Identifier, ColonEquals]) {
              self.presume_keyword(Keyword::Export)?;
              items.push(Item::Assignment(self.parse_assignment(true)?));
            } else {
              let doc = pop_doc_comment(&mut items, eol_since_last_comment);
              items.push(Item::Recipe(self.parse_recipe(doc, false)?));
            },
          Some(Keyword::Set) =>
            if self.next_are(&[Identifier, Identifier, ColonEquals])
              || self.next_are(&[Identifier, Identifier, Eol])
              || self.next_are(&[Identifier, Identifier, Eof])
            {
              items.push(Item::Set(self.parse_set()?));
            } else {
              let doc = pop_doc_comment(&mut items, eol_since_last_comment);
              items.push(Item::Recipe(self.parse_recipe(doc, false)?));
            },
          _ =>
            if self.next_are(&[Identifier, Equals]) {
              return Err(self.get(1)?.error(CompilationErrorKind::DeprecatedEquals));
            } else if self.next_are(&[Identifier, ColonEquals]) {
              items.push(Item::Assignment(self.parse_assignment(false)?));
            } else {
              let doc = pop_doc_comment(&mut items, eol_since_last_comment);
              items.push(Item::Recipe(self.parse_recipe(doc, false)?));
            },
        }
      } else if self.accepted(At)? {
        let doc = pop_doc_comment(&mut items, eol_since_last_comment);
        items.push(Item::Recipe(self.parse_recipe(doc, true)?));
      } else {
        return Err(self.unexpected_token()?);
      }
    }

    if self.next != self.tokens.len() {
      Err(self.internal_error(format!(
        "Parse completed with {} unparsed tokens",
        self.tokens.len() - self.next,
      ))?)
    } else {
      Ok(Module {
        warnings: Vec::new(),
        items,
      })
    }
  }

  /// Parse an alias, e.g `alias name := target`
  fn parse_alias(&mut self) -> CompilationResult<'src, Alias<'src, Name<'src>>> {
    self.presume_keyword(Keyword::Alias)?;
    let name = self.parse_name()?;
    self.presume_any(&[Equals, ColonEquals])?;
    let target = self.parse_name()?;
    self.expect_eol()?;
    Ok(Alias { name, target })
  }

  /// Parse an assignment, e.g. `foo := bar`
  fn parse_assignment(&mut self, export: bool) -> CompilationResult<'src, Assignment<'src>> {
    let name = self.parse_name()?;
    self.presume_any(&[Equals, ColonEquals])?;
    let value = self.parse_expression()?;
    self.expect_eol()?;
    Ok(Assignment {
      export,
      name,
      value,
    })
  }

  /// Parse an expression, e.g. `1 + 2`
  fn parse_expression(&mut self) -> CompilationResult<'src, Expression<'src>> {
    if self.accepted_keyword(Keyword::If)? {
      let lhs = self.parse_expression()?;

      let inverted = self.accepted(BangEquals)?;

      if !inverted {
        self.expect(EqualsEquals)?;
      }

      let rhs = self.parse_expression()?;

      self.expect(BraceL)?;

      let then = self.parse_expression()?;

      self.expect(BraceR)?;

      self.expect_keyword(Keyword::Else)?;

      self.expect(BraceL)?;

      let otherwise = self.parse_expression()?;

      self.expect(BraceR)?;

      return Ok(Expression::Conditional {
        lhs: Box::new(lhs),
        rhs: Box::new(rhs),
        then: Box::new(then),
        otherwise: Box::new(otherwise),
        inverted,
      });
    }

    let value = self.parse_value()?;

    if self.accepted(Plus)? {
      let lhs = Box::new(value);
      let rhs = Box::new(self.parse_expression()?);
      Ok(Expression::Concatination { lhs, rhs })
    } else {
      Ok(value)
    }
  }

  /// Parse a value, e.g. `(bar)`
  fn parse_value(&mut self) -> CompilationResult<'src, Expression<'src>> {
    if self.next_is(StringToken) {
      Ok(Expression::StringLiteral {
        string_literal: self.parse_string_literal()?,
      })
    } else if self.next_is(Backtick) {
      let next = self.next()?;
      let kind = StringKind::from_string_or_backtick(next)?;
      let contents =
        &next.lexeme()[kind.delimiter_len()..next.lexeme().len() - kind.delimiter_len()];
      let token = self.advance()?;
      let contents = if kind.indented() {
        unindent(contents)
      } else {
        contents.to_owned()
      };

      if contents.starts_with("#!") {
        return Err(next.error(CompilationErrorKind::BacktickShebang));
      }

      Ok(Expression::Backtick { contents, token })
    } else if self.next_is(Identifier) {
      let name = self.parse_name()?;

      if self.next_is(ParenL) {
        let arguments = self.parse_sequence()?;
        Ok(Expression::Call {
          thunk: Thunk::resolve(name, arguments)?,
        })
      } else {
        Ok(Expression::Variable { name })
      }
    } else if self.next_is(ParenL) {
      self.presume(ParenL)?;
      let contents = Box::new(self.parse_expression()?);
      self.expect(ParenR)?;
      Ok(Expression::Group { contents })
    } else {
      Err(self.unexpected_token()?)
    }
  }

  /// Parse a string literal, e.g. `"FOO"`
  fn parse_string_literal(&mut self) -> CompilationResult<'src, StringLiteral<'src>> {
    let token = self.expect(StringToken)?;

    let kind = StringKind::from_string_or_backtick(token)?;

    let delimiter_len = kind.delimiter_len();

    let raw = &token.lexeme()[delimiter_len..token.lexeme().len() - delimiter_len];

    let unindented = if kind.indented() {
      unindent(raw)
    } else {
      raw.to_owned()
    };

    let cooked = if kind.processes_escape_sequences() {
      let mut cooked = String::new();
      let mut escape = false;
      for c in unindented.chars() {
        if escape {
          match c {
            'n' => cooked.push('\n'),
            'r' => cooked.push('\r'),
            't' => cooked.push('\t'),
            '\\' => cooked.push('\\'),
            '\n' => {},
            '"' => cooked.push('"'),
            other => {
              return Err(
                token.error(CompilationErrorKind::InvalidEscapeSequence { character: other }),
              );
            },
          }
          escape = false;
        } else if c == '\\' {
          escape = true;
        } else {
          cooked.push(c);
        }
      }
      cooked
    } else {
      unindented
    };

    Ok(StringLiteral { kind, raw, cooked })
  }

  /// Parse a name from an identifier token
  fn parse_name(&mut self) -> CompilationResult<'src, Name<'src>> {
    self.expect(Identifier).map(Name::from_identifier)
  }

  /// Parse sequence of comma-separated expressions
  fn parse_sequence(&mut self) -> CompilationResult<'src, Vec<Expression<'src>>> {
    self.presume(ParenL)?;

    let mut elements = Vec::new();

    while !self.next_is(ParenR) {
      elements.push(self.parse_expression()?);

      if !self.accepted(Comma)? {
        break;
      }
    }

    self.expect(ParenR)?;

    Ok(elements)
  }

  /// Parse a recipe
  fn parse_recipe(
    &mut self,
    doc: Option<&'src str>,
    quiet: bool,
  ) -> CompilationResult<'src, UnresolvedRecipe<'src>> {
    let name = self.parse_name()?;

    let mut positional = Vec::new();

    while self.next_is(Identifier) || self.next_is(Dollar) {
      positional.push(self.parse_parameter(ParameterKind::Singular)?);
    }

    let kind = if self.accepted(Plus)? {
      ParameterKind::Plus
    } else if self.accepted(Asterisk)? {
      ParameterKind::Star
    } else {
      ParameterKind::Singular
    };

    let variadic = if kind.is_variadic() {
      let variadic = self.parse_parameter(kind)?;

      self.forbid(Identifier, |token| {
        token.error(CompilationErrorKind::ParameterFollowsVariadicParameter {
          parameter: token.lexeme(),
        })
      })?;

      Some(variadic)
    } else {
      None
    };

    self.expect(Colon)?;

    let mut dependencies = Vec::new();

    while let Some(dependency) = self.accept_dependency()? {
      dependencies.push(dependency);
    }

    self.expect_eol()?;

    let body = self.parse_body()?;

    Ok(Recipe {
      private: name.lexeme().starts_with('_'),
      shebang: body.first().map(Line::is_shebang).unwrap_or(false),
      parameters: positional.into_iter().chain(variadic).collect(),
      doc,
      name,
      quiet,
      dependencies,
      body,
    })
  }

  /// Parse a recipe parameter
  fn parse_parameter(&mut self, kind: ParameterKind) -> CompilationResult<'src, Parameter<'src>> {
    let export = self.accepted(Dollar)?;

    let name = self.parse_name()?;

    let default = if self.accepted(Equals)? {
      Some(self.parse_value()?)
    } else {
      None
    };

    Ok(Parameter {
      name,
      kind,
      default,
      export,
    })
  }

  /// Parse the body of a recipe
  fn parse_body(&mut self) -> CompilationResult<'src, Vec<Line<'src>>> {
    let mut lines = Vec::new();

    if self.accepted(Indent)? {
      while !self.accepted(Dedent)? {
        let line = if self.accepted(Eol)? {
          Line {
            fragments: Vec::new(),
          }
        } else {
          let mut fragments = Vec::new();

          while !(self.accepted(Eol)? || self.next_is(Dedent)) {
            if let Some(token) = self.accept(Text)? {
              fragments.push(Fragment::Text { token });
            } else if self.accepted(InterpolationStart)? {
              fragments.push(Fragment::Interpolation {
                expression: self.parse_expression()?,
              });
              self.expect(InterpolationEnd)?;
            } else {
              return Err(self.unexpected_token()?);
            }
          }

          Line { fragments }
        };

        lines.push(line);
      }
    }

    while lines.last().map(Line::is_empty).unwrap_or(false) {
      lines.pop();
    }

    Ok(lines)
  }

  /// Parse a boolean setting value
  fn parse_set_bool(&mut self) -> CompilationResult<'src, bool> {
    if !self.accepted(ColonEquals)? {
      return Ok(true);
    }

    let identifier = self.expect(Identifier)?;

    let value = if Keyword::True == identifier.lexeme() {
      true
    } else if Keyword::False == identifier.lexeme() {
      false
    } else {
      return Err(identifier.error(CompilationErrorKind::ExpectedKeyword {
        expected: vec![Keyword::True, Keyword::False],
        found:    identifier.lexeme(),
      }));
    };

    Ok(value)
  }

  /// Parse a setting
  fn parse_set(&mut self) -> CompilationResult<'src, Set<'src>> {
    self.presume_keyword(Keyword::Set)?;
    let name = Name::from_identifier(self.presume(Identifier)?);
    let lexeme = name.lexeme();

    if Keyword::DotenvLoad == lexeme {
      let value = self.parse_set_bool()?;
      return Ok(Set {
        value: Setting::DotenvLoad(value),
        name,
      });
    } else if Keyword::Export == lexeme {
      let value = self.parse_set_bool()?;
      return Ok(Set {
        value: Setting::Export(value),
        name,
      });
    } else if Keyword::PositionalArguments == lexeme {
      let value = self.parse_set_bool()?;
      return Ok(Set {
        value: Setting::PositionalArguments(value),
        name,
      });
    }

    self.expect(ColonEquals)?;

    if name.lexeme() == Keyword::Shell.lexeme() {
      self.expect(BracketL)?;

      let command = self.parse_string_literal()?;

      let mut arguments = Vec::new();

      if self.accepted(Comma)? {
        while !self.next_is(BracketR) {
          arguments.push(self.parse_string_literal()?);

          if !self.accepted(Comma)? {
            break;
          }
        }
      }

      self.expect(BracketR)?;

      Ok(Set {
        value: Setting::Shell(setting::Shell { command, arguments }),
        name,
      })
    } else {
      Err(name.error(CompilationErrorKind::UnknownSetting {
        setting: name.lexeme(),
      }))
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use pretty_assertions::assert_eq;
  use CompilationErrorKind::*;

  macro_rules! test {
    {
      name: $name:ident,
      text: $text:expr,
      tree: $tree:tt,
    } => {
      #[test]
      fn $name() {
        let text: String = $text.into();
        let want = tree!($tree);
        test(&text, want);
      }
    }
  }

  fn test(text: &str, want: Tree) {
    let unindented = unindent(text);
    let tokens = Lexer::lex(&unindented).expect("lexing failed");
    let justfile = Parser::parse(&tokens).expect("parsing failed");
    let have = justfile.tree();
    if have != want {
      println!("parsed text: {}", unindented);
      println!("expected:    {}", want);
      println!("but got:     {}", have);
      println!("tokens:      {:?}", tokens);
      panic!();
    }
  }

  macro_rules! error {
    (
      name:   $name:ident,
      input:  $input:expr,
      offset: $offset:expr,
      line:   $line:expr,
      column: $column:expr,
      width:  $width:expr,
      kind:   $kind:expr,
    ) => {
      #[test]
      fn $name() {
        error($input, $offset, $line, $column, $width, $kind);
      }
    };
  }

  fn error(
    src: &str,
    offset: usize,
    line: usize,
    column: usize,
    length: usize,
    kind: CompilationErrorKind,
  ) {
    let tokens = Lexer::lex(src).expect("Lexing failed in parse test...");

    match Parser::parse(&tokens) {
      Ok(_) => panic!("Parsing unexpectedly succeeded"),
      Err(have) => {
        let want = CompilationError {
          token: Token {
            kind: have.token.kind,
            src,
            offset,
            line,
            column,
            length,
          },
          kind,
        };
        assert_eq!(have, want);
      },
    }
  }

  test! {
    name: empty,
    text: "",
    tree: (justfile),
  }

  test! {
    name: empty_multiline,
    text: "





    ",
    tree: (justfile),
  }

  test! {
    name: whitespace,
    text: " ",
    tree: (justfile),
  }

  test! {
    name: alias_single,
    text: "alias t := test",
    tree: (justfile (alias t test)),
  }

  test! {
    name: aliases_multiple,
    text: "alias t := test\nalias b := build",
    tree: (
      justfile
      (alias t test)
      (alias b build)
    ),
  }

  test! {
    name: alias_equals,
    text: "alias t := test",
    tree: (justfile
      (alias t test)
    ),
  }

  test! {
    name: export,
    text: r#"export x := "hello""#,
    tree: (justfile (assignment #export x "hello")),
  }

  test! {
    name: export_equals,
    text: r#"export x := "hello""#,
    tree: (justfile
      (assignment #export x "hello")
    ),
  }

  test! {
    name: assignment,
    text: r#"x := "hello""#,
    tree: (justfile (assignment x "hello")),
  }

  test! {
    name: assignment_equals,
    text: r#"x := "hello""#,
    tree: (justfile
      (assignment x "hello")
    ),
  }

  test! {
    name: backtick,
    text: "x := `hello`",
    tree: (justfile (assignment x (backtick "hello"))),
  }

  test! {
    name: variable,
    text: "x := y",
    tree: (justfile (assignment x y)),
  }

  test! {
    name: group,
    text: "x := (y)",
    tree: (justfile (assignment x (y))),
  }

  test! {
    name: addition_single,
    text: "x := a + b",
    tree: (justfile (assignment x (+ a b))),
  }

  test! {
    name: addition_chained,
    text: "x := a + b + c",
    tree: (justfile (assignment x (+ a (+ b c)))),
  }

  test! {
    name: call_one_arg,
    text: "x := env_var(y)",
    tree: (justfile (assignment x (call env_var y))),
  }

  test! {
    name: call_multiple_args,
    text: "x := env_var_or_default(y, z)",
    tree: (justfile (assignment x (call env_var_or_default y z))),
  }

  test! {
    name: call_trailing_comma,
    text: "x := env_var(y,)",
    tree: (justfile (assignment x (call env_var y))),
  }

  test! {
    name: recipe,
    text: "foo:",
    tree: (justfile (recipe foo)),
  }

  test! {
    name: recipe_multiple,
    text: "
      foo:
      bar:
      baz:
    ",
    tree: (justfile (recipe foo) (recipe bar) (recipe baz)),
  }

  test! {
    name: recipe_quiet,
    text: "@foo:",
    tree: (justfile (recipe #quiet foo)),
  }

  test! {
    name: recipe_parameter_single,
    text: "foo bar:",
    tree: (justfile (recipe foo (params (bar)))),
  }

  test! {
    name: recipe_parameter_multiple,
    text: "foo bar baz:",
    tree: (justfile (recipe foo (params (bar) (baz)))),
  }

  test! {
    name: recipe_default_single,
    text: r#"foo bar="baz":"#,
    tree: (justfile (recipe foo (params (bar "baz")))),
  }

  test! {
    name: recipe_default_multiple,
    text: r#"foo bar="baz" bob="biz":"#,
    tree: (justfile (recipe foo (params (bar "baz") (bob "biz")))),
  }

  test! {
    name: recipe_plus_variadic,
    text: r#"foo +bar:"#,
    tree: (justfile (recipe foo (params +(bar)))),
  }

  test! {
    name: recipe_star_variadic,
    text: r#"foo *bar:"#,
    tree: (justfile (recipe foo (params *(bar)))),
  }

  test! {
    name: recipe_variadic_string_default,
    text: r#"foo +bar="baz":"#,
    tree: (justfile (recipe foo (params +(bar "baz")))),
  }

  test! {
    name: recipe_variadic_variable_default,
    text: r#"foo +bar=baz:"#,
    tree: (justfile (recipe foo (params +(bar baz)))),
  }

  test! {
    name: recipe_variadic_addition_group_default,
    text: r#"foo +bar=(baz + bob):"#,
    tree: (justfile (recipe foo (params +(bar ((+ baz bob)))))),
  }

  test! {
    name: recipe_dependency_single,
    text: "foo: bar",
    tree: (justfile (recipe foo (deps bar))),
  }

  test! {
    name: recipe_dependency_multiple,
    text: "foo: bar baz",
    tree: (justfile (recipe foo (deps bar baz))),
  }

  test! {
    name: recipe_dependency_parenthesis,
    text: "foo: (bar)",
    tree: (justfile (recipe foo (deps bar))),
  }

  test! {
    name: recipe_dependency_argument_string,
    text: "foo: (bar 'baz')",
    tree: (justfile (recipe foo (deps (bar "baz")))),
  }

  test! {
    name: recipe_dependency_argument_identifier,
    text: "foo: (bar baz)",
    tree: (justfile (recipe foo (deps (bar baz)))),
  }

  test! {
    name: recipe_dependency_argument_concatination,
    text: "foo: (bar 'a' + 'b' 'c' + 'd')",
    tree: (justfile (recipe foo (deps (bar (+ 'a' 'b') (+ 'c' 'd'))))),
  }

  test! {
    name: recipe_line_single,
    text: "foo:\n bar",
    tree: (justfile (recipe foo (body ("bar")))),
  }

  test! {
    name: recipe_line_multiple,
    text: "foo:\n bar\n baz\n {{\"bob\"}}biz",
    tree: (justfile (recipe foo (body ("bar") ("baz") (("bob") "biz")))),
  }

  test! {
    name: recipe_line_interpolation,
    text: "foo:\n bar{{\"bob\"}}biz",
    tree: (justfile (recipe foo (body ("bar" ("bob") "biz")))),
  }

  test! {
    name: comment,
    text: "# foo",
    tree: (justfile (comment "# foo")),
  }

  test! {
    name: comment_before_alias,
    text: "# foo\nalias x := y",
    tree: (justfile (comment "# foo") (alias x y)),
  }

  test! {
    name: comment_after_alias,
    text: "alias x := y # foo",
    tree: (justfile (alias x y)),
  }

  test! {
    name: comment_assignment,
    text: "x := y # foo",
    tree: (justfile (assignment x y)),
  }

  test! {
    name: comment_export,
    text: "export x := y # foo",
    tree: (justfile (assignment #export x y)),
  }

  test! {
    name: comment_recipe,
    text: "foo: # bar",
    tree: (justfile (recipe foo)),
  }

  test! {
    name: comment_recipe_dependencies,
    text: "foo: bar # baz",
    tree: (justfile (recipe foo (deps bar))),
  }

  test! {
    name: doc_comment_single,
    text: "
      # foo
      bar:
    ",
    tree: (justfile (recipe "foo" bar)),
  }

  test! {
    name: doc_comment_recipe_clear,
    text: "
      # foo
      bar:
      baz:
    ",
    tree: (justfile (recipe "foo" bar) (recipe baz)),
  }

  test! {
    name: doc_comment_middle,
    text: "
      bar:
      # foo
      baz:
    ",
    tree: (justfile (recipe bar) (recipe "foo" baz)),
  }

  test! {
    name: doc_comment_assignment_clear,
    text: "
      # foo
      x := y
      bar:
    ",
    tree: (justfile (comment "# foo") (assignment x y) (recipe bar)),
  }

  test! {
    name: doc_comment_empty_line_clear,
    text: "
      # foo

      bar:
    ",
    tree: (justfile (comment "# foo") (recipe bar)),
  }

  test! {
    name: string_escape_tab,
    text: r#"x := "foo\tbar""#,
    tree: (justfile (assignment x "foo\tbar")),
  }

  test! {
    name: string_escape_newline,
    text: r#"x := "foo\nbar""#,
    tree: (justfile (assignment x "foo\nbar")),
  }

  test! {
    name: string_escape_suppress_newline,
    text: r#"
      x := "foo\
      bar"
    "#,
    tree: (justfile (assignment x "foobar")),
  }

  test! {
    name: string_escape_carriage_return,
    text: r#"x := "foo\rbar""#,
    tree: (justfile (assignment x "foo\rbar")),
  }

  test! {
    name: string_escape_slash,
    text: r#"x := "foo\\bar""#,
    tree: (justfile (assignment x "foo\\bar")),
  }

  test! {
    name: string_escape_quote,
    text: r#"x := "foo\"bar""#,
    tree: (justfile (assignment x "foo\"bar")),
  }

  test! {
    name: indented_string_raw_with_dedent,
    text: "
      x := '''
        foo\\t
        bar\\n
      '''
    ",
    tree: (justfile (assignment x "foo\\t\nbar\\n\n")),
  }

  test! {
    name: indented_string_raw_no_dedent,
    text: "
      x := '''
      foo\\t
        bar\\n
      '''
    ",
    tree: (justfile (assignment x "foo\\t\n  bar\\n\n")),
  }

  test! {
    name: indented_string_cooked,
    text: r#"
      x := """
        \tfoo\t
        \tbar\n
      """
    "#,
    tree: (justfile (assignment x "\tfoo\t\n\tbar\n\n")),
  }

  test! {
    name: indented_string_cooked_no_dedent,
    text: r#"
      x := """
      \tfoo\t
        \tbar\n
      """
    "#,
    tree: (justfile (assignment x "\tfoo\t\n  \tbar\n\n")),
  }

  test! {
    name: indented_backtick,
    text: r#"
      x := ```
        \tfoo\t
        \tbar\n
      ```
    "#,
    tree: (justfile (assignment x (backtick "\\tfoo\\t\n\\tbar\\n\n"))),
  }

  test! {
    name: indented_backtick_no_dedent,
    text: r#"
      x := ```
      \tfoo\t
        \tbar\n
      ```
    "#,
    tree: (justfile (assignment x (backtick "\\tfoo\\t\n  \\tbar\\n\n"))),
  }

  test! {
    name: recipe_variadic_with_default_after_default,
    text: r#"
      f a=b +c=d:
    "#,
    tree: (justfile (recipe f (params (a b) +(c d)))),
  }

  test! {
    name: parameter_default_concatination_variable,
    text: r#"
      x := "10"

      f y=(`echo hello` + x) +z="foo":
    "#,
    tree: (justfile
      (assignment x "10")
      (recipe f (params (y ((+ (backtick "echo hello") x))) +(z "foo")))
    ),
  }

  test! {
    name: parameter_default_multiple,
    text: r#"
      x := "10"
      f y=(`echo hello` + x) +z=("foo" + "bar"):
    "#,
    tree: (justfile
      (assignment x "10")
      (recipe f (params (y ((+ (backtick "echo hello") x))) +(z ((+ "foo" "bar")))))
    ),
  }

  test! {
    name: parse_raw_string_default,
    text: r#"

      foo a='b\t':


    "#,
    tree: (justfile (recipe foo (params (a "b\\t")))),
  }

  test! {
    name: parse_alias_after_target,
    text: r"
      foo:
        echo a
      alias f := foo
    ",
    tree: (justfile
      (recipe foo (body ("echo a")))
      (alias f foo)
    ),
  }

  test! {
    name: parse_alias_before_target,
    text: "
      alias f := foo
      foo:
        echo a
      ",
    tree: (justfile
      (alias f foo)
      (recipe foo (body ("echo a")))
    ),
  }

  test! {
    name: parse_alias_with_comment,
    text: "
      alias f := foo #comment
      foo:
        echo a
    ",
    tree: (justfile
      (alias f foo)
      (recipe foo (body ("echo a")))
    ),
  }

  test! {
    name: parse_assignment_with_comment,
    text: "
      f := foo #comment
      foo:
        echo a
    ",
    tree: (justfile
      (assignment f foo)
      (recipe foo (body ("echo a")))
    ),
  }

  test! {
    name: parse_complex,
    text: "
      x:
      y:
      z:
      foo := \"xx\"
      bar := foo
      goodbye := \"y\"
      hello a b    c   : x y    z #hello
        #! blah
        #blarg
        {{ foo + bar}}abc{{ goodbye\t  + \"x\" }}xyz
        1
        2
        3
    ",
    tree: (justfile
      (recipe x)
      (recipe y)
      (recipe z)
      (assignment foo "xx")
      (assignment bar foo)
      (assignment goodbye "y")
      (recipe hello
        (params (a) (b) (c))
        (deps x y z)
        (body
          ("#! blah")
          ("#blarg")
          (((+ foo bar)) "abc" ((+ goodbye "x")) "xyz")
          ("1")
          ("2")
          ("3")
        )
      )
    ),
  }

  test! {
    name: parse_shebang,
    text: "
      practicum := 'hello'
      install:
      \t#!/bin/sh
      \tif [[ -f {{practicum}} ]]; then
      \t\treturn
      \tfi
      ",
    tree: (justfile
      (assignment practicum "hello")
      (recipe install
        (body
         ("#!/bin/sh")
         ("if [[ -f " (practicum) " ]]; then")
         ("\treturn")
         ("fi")
        )
      )
    ),
  }

  test! {
    name: parse_simple_shebang,
    text: "a:\n #!\n  print(1)",
    tree: (justfile
      (recipe a (body ("#!") (" print(1)")))
    ),
  }

  test! {
    name: parse_assignments,
    text: r#"
      a := "0"
      c := a + b + a + b
      b := "1"
    "#,
    tree: (justfile
      (assignment a "0")
      (assignment c (+ a (+ b (+ a b))))
      (assignment b "1")
    ),
  }

  test! {
    name: parse_assignment_backticks,
    text: "
      a := `echo hello`
      c := a + b + a + b
      b := `echo goodbye`
    ",
    tree: (justfile
      (assignment a (backtick "echo hello"))
      (assignment c (+ a (+ b (+ a b))))
      (assignment b (backtick "echo goodbye"))
    ),
  }

  test! {
    name: parse_interpolation_backticks,
    text: r#"
      a:
        echo {{  `echo hello` + "blarg"   }} {{   `echo bob`   }}
    "#,
    tree: (justfile
      (recipe a
        (body ("echo " ((+ (backtick "echo hello") "blarg")) " " ((backtick "echo bob"))))
      )
    ),
  }

  test! {
    name: eof_test,
    text: "x:\ny:\nz:\na b c: x y z",
    tree: (justfile
      (recipe x)
      (recipe y)
      (recipe z)
      (recipe a (params (b) (c)) (deps x y z))
    ),
  }

  test! {
    name: string_quote_escape,
    text: r#"a := "hello\"""#,
    tree: (justfile
      (assignment a "hello\"")
    ),
  }

  test! {
    name: string_escapes,
    text: r#"a := "\n\t\r\"\\""#,
    tree: (justfile (assignment a "\n\t\r\"\\")),
  }

  test! {
    name: parameters,
    text: "
      a b c:
        {{b}} {{c}}
    ",
    tree: (justfile (recipe a (params (b) (c)) (body ((b) " " (c))))),
  }

  test! {
    name: unary_functions,
    text: "
      x := arch()

      a:
        {{os()}} {{os_family()}}
    ",
    tree: (justfile
      (assignment x (call arch))
      (recipe a (body (((call os)) " " ((call os_family)))))
    ),
  }

  test! {
    name: env_functions,
    text: r#"
      x := env_var('foo',)

      a:
        {{env_var_or_default('foo' + 'bar', 'baz',)}} {{env_var(env_var("baz"))}}
    "#,
    tree: (justfile
      (assignment x (call env_var "foo"))
      (recipe a
        (body
          (
            ((call env_var_or_default (+ "foo" "bar") "baz"))
            " "
            ((call env_var (call env_var "baz")))
          )
        )
      )
    ),
  }

  test! {
    name: parameter_default_string,
    text: r#"
      f x="abc":
    "#,
    tree: (justfile (recipe f (params (x "abc")))),
  }

  test! {
    name: parameter_default_raw_string,
    text: r"
      f x='abc':
    ",
    tree: (justfile (recipe f (params (x "abc")))),
  }

  test! {
    name: parameter_default_backtick,
    text: "
      f x=`echo hello`:
    ",
    tree: (justfile
      (recipe f (params (x (backtick "echo hello"))))
    ),
  }

  test! {
    name: parameter_default_concatination_string,
    text: r#"
      f x=(`echo hello` + "foo"):
    "#,
    tree: (justfile (recipe f (params (x ((+ (backtick "echo hello") "foo")))))),
  }

  test! {
    name: concatination_in_group,
    text: "x := ('0' + '1')",
    tree: (justfile (assignment x ((+ "0" "1")))),
  }

  test! {
    name: string_in_group,
    text: "x := ('0'   )",
    tree: (justfile (assignment x ("0"))),
  }

  test! {
    name: escaped_dos_newlines,
    text: "
      @spam:\r
      \t{ \\\r
      \t\tfiglet test; \\\r
      \t\tcargo build --color always 2>&1; \\\r
      \t\tcargo test  --color always -- --color always 2>&1; \\\r
      \t} | less\r
    ",
    tree: (justfile
      (recipe #quiet spam
        (body
         ("{ \\")
         ("\tfiglet test; \\")
         ("\tcargo build --color always 2>&1; \\")
         ("\tcargo test  --color always -- --color always 2>&1; \\")
         ("} | less")
        )
      )
    ),
  }

  test! {
    name: empty_body,
    text: "a:",
    tree: (justfile (recipe a)),
  }

  test! {
    name: single_line_body,
    text: "a:\n foo",
    tree: (justfile (recipe a (body ("foo")))),
  }

  test! {
    name: trimmed_body,
    text: "a:\n foo\n \n \n \nb:\n  ",
    tree: (justfile (recipe a (body ("foo"))) (recipe b)),
  }

  test! {
    name: set_export_implicit,
    text: "set export",
    tree: (justfile (set export true)),
  }

  test! {
    name: set_export_true,
    text: "set export := true",
    tree: (justfile (set export true)),
  }

  test! {
    name: set_export_false,
    text: "set export := false",
    tree: (justfile (set export false)),
  }

  test! {
    name: set_dotenv_load_implicit,
    text: "set dotenv-load",
    tree: (justfile (set dotenv_load true)),
  }

  test! {
    name: set_dotenv_load_true,
    text: "set dotenv-load := true",
    tree: (justfile (set dotenv_load true)),
  }

  test! {
    name: set_dotenv_load_false,
    text: "set dotenv-load := false",
    tree: (justfile (set dotenv_load false)),
  }

  test! {
    name: set_positional_arguments_implicit,
    text: "set positional-arguments",
    tree: (justfile (set positional_arguments true)),
  }

  test! {
    name: set_positional_arguments_true,
    text: "set positional-arguments := true",
    tree: (justfile (set positional_arguments true)),
  }

  test! {
    name: set_positional_arguments_false,
    text: "set positional-arguments := false",
    tree: (justfile (set positional_arguments false)),
  }

  test! {
    name: set_shell_no_arguments,
    text: "set shell := ['tclsh']",
    tree: (justfile (set shell "tclsh")),
  }

  test! {
    name: set_shell_no_arguments_cooked,
    text: "set shell := [\"tclsh\"]",
    tree: (justfile (set shell "tclsh")),
  }

  test! {
    name: set_shell_no_arguments_trailing_comma,
    text: "set shell := ['tclsh',]",
    tree: (justfile (set shell "tclsh")),
  }

  test! {
    name: set_shell_with_one_argument,
    text: "set shell := ['bash', '-cu']",
    tree: (justfile (set shell "bash" "-cu")),
  }

  test! {
    name: set_shell_with_one_argument_trailing_comma,
    text: "set shell := ['bash', '-cu',]",
    tree: (justfile (set shell "bash" "-cu")),
  }

  test! {
    name: set_shell_with_two_arguments,
    text: "set shell := ['bash', '-cu', '-l']",
    tree: (justfile (set shell "bash" "-cu" "-l")),
  }

  test! {
    name: conditional,
    text: "a := if b == c { d } else { e }",
    tree: (justfile (assignment a (if b == c d e))),
  }

  test! {
    name: conditional_inverted,
    text: "a := if b != c { d } else { e }",
    tree: (justfile (assignment a (if b != c d e))),
  }

  test! {
    name: conditional_concatinations,
    text: "a := if b0 + b1 == c0 + c1 { d0 + d1 } else { e0 + e1 }",
    tree: (justfile (assignment a (if (+ b0 b1) == (+ c0 c1) (+ d0 d1) (+ e0 e1)))),
  }

  test! {
    name: conditional_nested_lhs,
    text: "a := if if b == c { d } else { e } == c { d } else { e }",
    tree: (justfile (assignment a (if (if b == c d e) == c d e))),
  }

  test! {
    name: conditional_nested_rhs,
    text: "a := if c == if b == c { d } else { e } { d } else { e }",
    tree: (justfile (assignment a (if c == (if b == c d e) d e))),
  }

  test! {
    name: conditional_nested_then,
    text: "a := if b == c { if b == c { d } else { e } } else { e }",
    tree: (justfile (assignment a (if b == c (if b == c d e) e))),
  }

  test! {
    name: conditional_nested_otherwise,
    text: "a := if b == c { d } else { if b == c { d } else { e } }",
    tree: (justfile (assignment a (if b == c d (if b == c d e)))),
  }

  error! {
    name:   alias_syntax_multiple_rhs,
    input:  "alias foo := bar baz",
    offset: 17,
    line:   0,
    column: 17,
    width:  3,
    kind:   UnexpectedToken { expected: vec![Comment, Eof, Eol], found: Identifier },
  }

  error! {
    name:   alias_syntax_no_rhs,
    input:  "alias foo := \n",
    offset: 13,
    line:   0,
    column: 13,
    width:  1,
    kind:   UnexpectedToken {expected: vec![Identifier], found:Eol},
  }

  error! {
    name:   missing_colon,
    input:  "a b c\nd e f",
    offset:  5,
    line:   0,
    column: 5,
    width:  1,
    kind:   UnexpectedToken{
      expected: vec![Asterisk, Colon, Dollar, Equals, Identifier, Plus],
      found:    Eol
    },
  }

  error! {
    name:   missing_default_eol,
    input:  "hello arg=\n",
    offset:  10,
    line:   0,
    column: 10,
    width:  1,
    kind:   UnexpectedToken {
      expected: vec![
        Backtick,
        Identifier,
        ParenL,
        StringToken,
      ],
      found: Eol
    },
  }

  error! {
    name:   missing_default_eof,
    input:  "hello arg=",
    offset:  10,
    line:   0,
    column: 10,
    width:  0,
    kind:   UnexpectedToken {
      expected: vec![
        Backtick,
        Identifier,
        ParenL,
        StringToken,
      ],
      found: Eof,
    },
  }

  error! {
    name:   missing_eol,
    input:  "a b c: z =",
    offset:  9,
    line:   0,
    column: 9,
    width:  1,
    kind:   UnexpectedToken{expected: vec![Comment, Eof, Eol, Identifier, ParenL], found: Equals},
  }

  error! {
    name:   unexpected_brace,
    input:  "{{",
    offset:  0,
    line:   0,
    column: 0,
    width:  1,
    kind: UnexpectedToken {
      expected: vec![At, Comment, Eof, Eol, Identifier],
      found: BraceL,
    },
  }

  error! {
    name:   unclosed_parenthesis_in_expression,
    input:  "x := foo(",
    offset: 9,
    line:   0,
    column: 9,
    width:  0,
    kind: UnexpectedToken{
      expected: vec![
        Backtick,
        Identifier,
        ParenL,
        ParenR,
        StringToken,
      ],
      found: Eof,
    },
  }

  error! {
    name:   unclosed_parenthesis_in_interpolation,
    input:  "a:\n echo {{foo(}}",
    offset:  15,
    line:   1,
    column: 12,
    width:  2,
    kind:   UnexpectedToken{
      expected: vec![
        Backtick,
        Identifier,
        ParenL,
        ParenR,
        StringToken,
      ],
      found: InterpolationEnd,
    },
  }

  error! {
    name:   plus_following_parameter,
    input:  "a b c+:",
    offset: 6,
    line:   0,
    column: 6,
    width:  1,
    kind:   UnexpectedToken{expected: vec![Dollar, Identifier], found: Colon},
  }

  error! {
    name:   invalid_escape_sequence,
    input:  r#"foo := "\b""#,
    offset: 7,
    line:   0,
    column: 7,
    width:  4,
    kind:   InvalidEscapeSequence{character: 'b'},
  }

  error! {
    name:   bad_export,
    input:  "export a",
    offset:  8,
    line:   0,
    column: 8,
    width:  0,
    kind:   UnexpectedToken {
      expected: vec![Asterisk, Colon, Dollar, Equals, Identifier, Plus],
      found:    Eof
    },
  }

  error! {
    name:   parameter_follows_variadic_parameter,
    input:  "foo +a b:",
    offset: 7,
    line:   0,
    column: 7,
    width:  1,
    kind:   ParameterFollowsVariadicParameter{parameter: "b"},
  }

  error! {
    name:   parameter_after_variadic,
    input:  "foo +a bbb:",
    offset: 7,
    line:   0,
    column: 7,
    width:  3,
    kind:   ParameterFollowsVariadicParameter{parameter: "bbb"},
  }

  error! {
    name:   concatination_in_default,
    input:  "foo a=c+d e:",
    offset: 10,
    line:   0,
    column: 10,
    width:  1,
    kind:   ParameterFollowsVariadicParameter{parameter: "e"},
  }

  error! {
    name:   set_shell_empty,
    input:  "set shell := []",
    offset: 14,
    line:   0,
    column: 14,
    width:  1,
    kind:   UnexpectedToken {
      expected: vec![
        StringToken,
      ],
      found: BracketR,
    },
  }

  error! {
    name:   set_shell_non_literal_first,
    input:  "set shell := ['bar' + 'baz']",
    offset: 20,
    line:   0,
    column: 20,
    width:  1,
    kind:   UnexpectedToken {
      expected: vec![BracketR, Comma],
      found: Plus,
    },
  }

  error! {
    name:   set_shell_non_literal_second,
    input:  "set shell := ['biz', 'bar' + 'baz']",
    offset: 27,
    line:   0,
    column: 27,
    width:  1,
    kind:   UnexpectedToken {
      expected: vec![BracketR, Comma],
      found: Plus,
    },
  }

  error! {
    name:   set_shell_bad_comma,
    input:  "set shell := ['bash',",
    offset: 21,
    line:   0,
    column: 21,
    width:  0,
    kind:   UnexpectedToken {
      expected: vec![
        BracketR,
        StringToken,
      ],
      found: Eof,
    },
  }

  error! {
    name:   set_shell_bad,
    input:  "set shell := ['bash'",
    offset: 20,
    line:   0,
    column: 20,
    width:  0,
    kind:   UnexpectedToken {
      expected: vec![BracketR, Comma],
      found: Eof,
    },
  }

  error! {
    name:   set_unknown,
    input:  "set shall := []",
    offset: 4,
    line:   0,
    column: 4,
    width:  5,
    kind:   UnknownSetting {
      setting: "shall",
    },
  }

  error! {
    name:   set_shell_non_string,
    input:  "set shall := []",
    offset: 4,
    line:   0,
    column: 4,
    width:  5,
    kind:   UnknownSetting {
      setting: "shall",
    },
  }

  error! {
    name:   unknown_function,
    input:  "a := foo()",
    offset: 5,
    line:   0,
    column: 5,
    width:  3,
    kind:   UnknownFunction{function: "foo"},
  }

  error! {
    name:   unknown_function_in_interpolation,
    input:  "a:\n echo {{bar()}}",
    offset: 11,
    line:   1,
    column: 8,
    width:  3,
    kind:   UnknownFunction{function: "bar"},
  }

  error! {
    name:   unknown_function_in_default,
    input:  "a f=baz():",
    offset: 4,
    line:   0,
    column: 4,
    width:  3,
    kind:   UnknownFunction{function: "baz"},
  }

  error! {
    name: function_argument_count_nullary,
    input: "x := arch('foo')",
    offset: 5,
    line: 0,
    column: 5,
    width: 4,
    kind: FunctionArgumentCountMismatch {
      function: "arch",
      found: 1,
      expected: 0,
    },
  }

  error! {
    name: function_argument_count_unary,
    input: "x := env_var()",
    offset: 5,
    line: 0,
    column: 5,
    width: 7,
    kind: FunctionArgumentCountMismatch {
      function: "env_var",
      found: 0,
      expected: 1,
    },
  }

  error! {
    name: function_argument_count_binary,
    input: "x := env_var_or_default('foo')",
    offset: 5,
    line: 0,
    column: 5,
    width: 18,
    kind: FunctionArgumentCountMismatch {
      function: "env_var_or_default",
      found: 1,
      expected: 2,
    },
  }
}
