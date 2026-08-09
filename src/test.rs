use crate::{Ast, GroupDelim, InfixOp, Lexer, Parser, Punct, TokenTree, Value};

#[test]
fn basic_lex() {
    let s = "69 + - * / ! , foo";
    let mut l = Lexer::new(s);

    assert_eq!(l.next(), Some(TokenTree::IntLit(69)));
    assert_eq!(l.next(), Some(TokenTree::Punct(Punct::Plus)));
    assert_eq!(l.next(), Some(TokenTree::Punct(Punct::Minus)));
    assert_eq!(l.next(), Some(TokenTree::Punct(Punct::Star)));
    assert_eq!(l.next(), Some(TokenTree::Punct(Punct::Slash)));
    assert_eq!(l.next(), Some(TokenTree::Punct(Punct::Bang)));
    assert_eq!(l.next(), Some(TokenTree::Punct(Punct::Comma)));
    assert_eq!(l.next(), Some(TokenTree::Ident("foo".into())));
    assert_eq!(l.next(), None);
}

#[test]
fn tree_lex() {
    let s = "(1 + 2)[7 * 6]";
    let mut l = Lexer::new(s);

    assert_eq!(
        l.next(),
        Some(TokenTree::Group {
            delim: GroupDelim::Paren,
            tokens: vec![
                TokenTree::IntLit(1),
                TokenTree::Punct(Punct::Plus),
                TokenTree::IntLit(2)
            ]
        })
    );
    assert_eq!(
        l.next(),
        Some(TokenTree::Group {
            delim: GroupDelim::Bracket,
            tokens: vec![
                TokenTree::IntLit(7),
                TokenTree::Punct(Punct::Star),
                TokenTree::IntLit(6)
            ]
        })
    );
    assert_eq!(l.next(), None);
}

#[test]
fn parse_left_to_right() {
    let s = "1 + 2 + 3";
    let mut l = Lexer::new(s);
    let mut parser = Parser::new(&mut l);

    // (1 + 2) + 3
    let ast = parser.parse_expr().unwrap();
    assert_eq!(
        ast,
        Ast::BinOp {
            op: InfixOp::Add,
            operands: Box::new((
                Ast::BinOp {
                    op: InfixOp::Add,
                    operands: Box::new((
                        Ast::Atom(TokenTree::IntLit(1)),
                        Ast::Atom(TokenTree::IntLit(2)),
                    ))
                },
                Ast::Atom(TokenTree::IntLit(3)),
            ))
        }
    );

    assert_eq!(l.next(), None); // ensure we've gobbled all tokens
}

#[test]
fn parse_oop_left_to_right() {
    let s = "1 * 2 + 3";
    let mut l = Lexer::new(s);
    let mut parser = Parser::new(&mut l);

    // (1 * 2) + 3
    let ast = parser.parse_expr().unwrap();
    assert_eq!(
        ast,
        Ast::BinOp {
            op: InfixOp::Add,
            operands: Box::new((
                Ast::BinOp {
                    op: InfixOp::Mul,
                    operands: Box::new((
                        Ast::Atom(TokenTree::IntLit(1)),
                        Ast::Atom(TokenTree::IntLit(2)),
                    ))
                },
                Ast::Atom(TokenTree::IntLit(3)),
            ))
        }
    );

    assert_eq!(l.next(), None); // ensure we've gobbled all tokens
}

#[test]
fn parse_oop_right_to_left() {
    let s = "1 + 2 * 3";
    let mut l = Lexer::new(s);
    let mut parser = Parser::new(&mut l);

    // 1 + (2 * 3)
    let ast = parser.parse_expr().unwrap();
    assert_eq!(
        ast,
        Ast::BinOp {
            op: InfixOp::Add,
            operands: Box::new((
                Ast::Atom(TokenTree::IntLit(1)),
                Ast::BinOp {
                    op: InfixOp::Mul,
                    operands: Box::new((
                        Ast::Atom(TokenTree::IntLit(2)),
                        Ast::Atom(TokenTree::IntLit(3)),
                    ))
                },
            ))
        }
    );

    assert_eq!(l.next(), None); // ensure we've gobbled all tokens
}

#[test]
fn parse_function_call() {
    let s = "foo(1 + 2, bar(3, 4), 5)";
    let mut l = Lexer::new(s);
    let mut parser = Parser::new(&mut l);

    let ast = parser.parse_expr().unwrap();
    assert_eq!(
        ast,
        Ast::FunctionCall {
            fun: Box::new(Ast::Atom(TokenTree::Ident("foo".into()))),
            args: vec![
                Ast::BinOp {
                    op: InfixOp::Add,
                    operands: Box::new((
                        Ast::Atom(TokenTree::IntLit(1)),
                        Ast::Atom(TokenTree::IntLit(2)),
                    ))
                },
                Ast::FunctionCall {
                    fun: Box::new(Ast::Atom(TokenTree::Ident("bar".into()))),
                    args: vec![
                        Ast::Atom(TokenTree::IntLit(3)),
                        Ast::Atom(TokenTree::IntLit(4)),
                    ],
                },
                Ast::Atom(TokenTree::IntLit(5)),
            ]
        }
    );

    assert_eq!(l.next(), None); // ensure we've gobbled all tokens
}

#[test]
fn eval_complex() {
    let s = "1 + 2 * 3 / 4 + 6 * (3 + 2) + 4!";

    fn factorial(n: i32) -> i32 {
        let mut out = 1;
        for i in 1..=n {
            out *= i;
        }
        out
    }

    let mut l = Lexer::new(s);
    let mut parser = Parser::new(&mut l);
    let ast = parser.parse_expr().unwrap();
    let result = ast.eval(&mut Default::default()).unwrap();
    assert_eq!(
        result,
        Value::Integer(1 + 2 * 3 / 4 + 6 * (3 + 2) + factorial(4))
    );
}

macro_rules! str_and_calc {
    ($($tt: tt)*) => {
        (stringify!($($tt)*), $($tt)*)
    };
}

#[test]
fn eval_array() {
    let (s, expected) = str_and_calc!([1, 2, 3][2]);

    let mut l = Lexer::new(s);
    let mut parser = Parser::new(&mut l);
    let ast = parser.parse_expr().unwrap();
    let result = ast.eval(&mut Default::default()).unwrap();
    assert_eq!(result, Value::Integer(expected));
}

#[test]
fn eval_array_complex() {
    let (s, expected) = str_and_calc!([3 + 4, 6 + 9, 4 + 2][7 * 6 / 21 - 1]);

    let mut l = Lexer::new(s);
    let mut parser = Parser::new(&mut l);
    let ast = parser.parse_expr().unwrap();
    let result = ast.eval(&mut Default::default()).unwrap();
    assert_eq!(result, Value::Integer(expected));
}
