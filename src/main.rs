mod ast;
mod database;
mod lexer;
mod parser;

use database::Database;

fn main() {
    let contents = std::fs::read_to_string("example.foo").unwrap();
    let tokens = lexer::lex(&contents);

    let mut database = Database::new();

    parser::parse(&mut database, &tokens);

    database.print_headers();
    database.print_unresolved_ast();

    database.resolve_idents();

    database.print_resolved_ast();
}
