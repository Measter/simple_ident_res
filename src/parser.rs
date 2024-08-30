use std::slice::Iter;

use crate::{
    ast::{UnresolvedAST, UnresolvedIdent},
    database::{Database, ItemId, ItemKind},
    lexer::{Token, TokenKind},
};

struct Parser<'a> {
    token_iter: Iter<'a, Token>,
}

impl Parser<'_> {
    fn expect(&mut self, kind: TokenKind) -> &Token {
        if self.peek() == kind {
            return self.token_iter.next().unwrap();
        }

        panic!("Expected token {:?}, found {:?}", kind, self.peek());
    }

    fn peek(&self) -> TokenKind {
        self.token_iter
            .clone()
            .next()
            .map(|t| t.kind)
            .unwrap_or(TokenKind::Eof)
    }
}

pub(crate) fn parse(database: &mut Database, tokens: &[Token]) {
    let mut parser = Parser {
        token_iter: tokens.iter(),
    };

    // Parsing top-level modules.
    loop {
        if parser.peek() == TokenKind::Eof {
            break;
        }
        parser.expect(TokenKind::Module);
        parse_module(database, &mut parser, None);
    }
}

fn parse_module(database: &mut Database, parser: &mut Parser, parent_id: Option<ItemId>) {
    // Keyword is already parsed
    let name = parser.expect(TokenKind::Ident).lexeme.clone();
    let module_id = database.new_item(name, ItemKind::Module, parent_id);

    parse_module_block(database, parser, module_id);
}

fn parse_module_block(database: &mut Database, parser: &mut Parser, parent_id: ItemId) {
    parser.expect(TokenKind::BraceLeft);

    loop {
        match parser.peek() {
            TokenKind::Function => {
                parser.expect(TokenKind::Function);
                parse_function(database, parser, parent_id);
            }
            TokenKind::Module => {
                parser.expect(TokenKind::Module);
                parse_module(database, parser, Some(parent_id));
            }
            TokenKind::Using => {
                parser.expect(TokenKind::Using);
                parse_using(database, parser, parent_id);
            }
            TokenKind::BraceRight => break,
            t => panic!("{:?}", t),
        }
    }

    parser.expect(TokenKind::BraceRight);
}

fn parse_using(database: &mut Database, parser: &mut Parser, item_id: ItemId) {
    // Keyword is already parsed.
    let ident = parse_ident(parser);
    parser.expect(TokenKind::Semicolon);
    database.add_import(item_id, ident);
}

fn parse_function(database: &mut Database, parser: &mut Parser, parent_id: ItemId) {
    // Keyword is already parsed.
    let name = parser.expect(TokenKind::Ident).lexeme.clone();
    let func_id = database.new_item(name, ItemKind::Function, Some(parent_id));

    parser.expect(TokenKind::ParenLeft);
    parser.expect(TokenKind::ParenRight);

    parse_function_block(database, parser, func_id);
}

fn parse_function_block(database: &mut Database, parser: &mut Parser, func_id: ItemId) {
    parser.expect(TokenKind::BraceLeft);

    let mut ast = Vec::new();

    loop {
        match parser.peek() {
            TokenKind::Ident => {
                // We're just assuming these are all calls.
                let ident = parse_ident(parser);
                parser.expect(TokenKind::ParenLeft);
                parser.expect(TokenKind::ParenRight);
                parser.expect(TokenKind::Semicolon);
                ast.push(UnresolvedAST::Call { ident });
            }
            TokenKind::Using => {
                parser.expect(TokenKind::Using);
                parse_using(database, parser, func_id);
            }
            TokenKind::BraceRight => break,
            t => panic!("{:?}", t),
        }
    }

    database.set_unresolved_body(func_id, ast);

    parser.expect(TokenKind::BraceRight);
}

fn parse_ident(parser: &mut Parser) -> UnresolvedIdent {
    let mut parts = vec![parser.expect(TokenKind::Ident).lexeme.clone()];

    while parser.peek() == TokenKind::Dot {
        parser.expect(TokenKind::Dot);
        parts.push(parser.expect(TokenKind::Ident).lexeme.clone());
    }

    UnresolvedIdent { parts }
}
