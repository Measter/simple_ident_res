use logos::Logos;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Logos)]
#[logos(skip "[ \t\r\n]*")]
pub enum TokenKind {
    #[token("{")]
    BraceLeft,

    #[token("}")]
    BraceRight,

    #[token(".")]
    Dot,

    #[regex("[a-zA-Z][a-zA-Z0-9_]+")]
    Ident,

    #[token("function")]
    Function,

    #[token("module")]
    Module,

    #[token("(")]
    ParenLeft,

    #[token(")")]
    ParenRight,

    #[token(";")]
    Semicolon,

    #[token("using")]
    Using,

    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
}

pub fn lex(source: &str) -> Vec<Token> {
    TokenKind::lexer(source)
        .spanned()
        .map(|(tk, span)| Token {
            kind: tk.unwrap(),
            lexeme: source[span].to_owned(),
        })
        .collect()
}
