use std::{collections::HashMap, fmt::Display, io::Write, iter::Peekable};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Punct {
    Plus,
    Minus,
    Star,
    Slash,
    Bang,
    Comma,
}

impl Punct {
    fn is_op(&self) -> bool {
        match self {
            Punct::Plus | Punct::Minus | Punct::Star | Punct::Slash | Punct::Bang => true,
            Punct::Comma => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GroupDelim {
    /// `()`
    Paren,
    /// `[]`
    Bracket,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TokenTree {
    Ident(String),
    IntLit(u32),
    Punct(Punct),
    Group {
        delim: GroupDelim,
        tokens: Vec<TokenTree>,
    },
}

#[derive(Debug, Clone)]
struct Lexer<'a> {
    content: &'a str,
    position: usize,
}

impl<'a> Lexer<'a> {
    fn new(content: &'a str) -> Self {
        Self {
            content,
            position: 0,
        }
    }

    fn skip_whitespace(&mut self) {
        self.position +=
            self.content[self.position..].len() - self.content[self.position..].trim_start().len();
    }

    fn peek_char(&mut self) -> Option<char> {
        self.content[self.position..].chars().next()
    }

    fn take_char(&mut self) -> Option<char> {
        let c = self.peek_char()?;
        self.position += c.len_utf8();
        Some(c)
    }

    fn untake_char(&mut self, c: char) {
        self.position -= c.len_utf8();
        assert_eq!(self.peek_char(), Some(c));
    }

    fn take_number(&mut self) -> TokenTree {
        let rest = &self.content[self.position..];
        let idx = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        self.position += idx;
        let num = &rest[..idx];
        let num = num.parse::<u32>().expect("TODO");
        TokenTree::IntLit(num)
    }

    fn take_ident(&mut self) -> &'a str {
        let rest = &self.content[self.position..];
        let idx = rest
            .find(|c: char| !matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_'))
            .unwrap_or(rest.len());
        self.position += idx;
        &rest[..idx]
    }

    fn take_group(&mut self, end: char) -> Option<Vec<TokenTree>> {
        let mut out = Vec::new();
        loop {
            let next = self.take_char()?;
            if next == end {
                break;
            }
            self.untake_char(next);
            out.push(self.next_token()?);
        }
        Some(out)
    }

    fn next_token(&mut self) -> Option<TokenTree> {
        self.skip_whitespace();

        Some(match self.take_char()? {
            c @ ('a'..='z' | 'A'..='Z') => {
                self.untake_char(c);
                TokenTree::Ident(self.take_ident().into())
            }
            '+' => TokenTree::Punct(Punct::Plus),
            '-' => TokenTree::Punct(Punct::Minus),
            '*' => TokenTree::Punct(Punct::Star),
            '/' => TokenTree::Punct(Punct::Slash),
            '!' => TokenTree::Punct(Punct::Bang),
            ',' => TokenTree::Punct(Punct::Comma),
            '(' => TokenTree::Group {
                delim: GroupDelim::Paren,
                tokens: self.take_group(')')?,
            },
            '[' => TokenTree::Group {
                delim: GroupDelim::Bracket,
                tokens: self.take_group(']')?,
            },
            c @ '0'..='9' => {
                self.untake_char(c);
                self.take_number()
            }
            c => {
                self.untake_char(c);
                panic!("Unexpected character: {:?}", c)
            }
        })
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = TokenTree;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrefixOp {
    Neg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PostfixOp {
    Factorial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InfixOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Ast {
    Atom(TokenTree),
    ArrayLit(Vec<Ast>),
    PrefixOp {
        op: PrefixOp,
        operand: Box<Ast>,
    },
    PostfixOp {
        op: PostfixOp,
        operand: Box<Ast>,
    },
    BinOp {
        op: InfixOp,
        operands: Box<(Ast, Ast)>,
    },
    FunctionCall {
        fun: Box<Ast>,
        args: Vec<Ast>,
    },
    Index {
        arr: Box<Ast>,
        idx: Box<Ast>,
    },
}

impl Display for Ast {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ast::Atom(token) => write!(f, "{:?}", token),
            Ast::ArrayLit(items) => {
                write!(f, "(<arr>")?;
                for a in items {
                    write!(f, " {}", a)?;
                }
                write!(f, ")")
            }
            Ast::PrefixOp { op, operand } => write!(f, "(<pre>{:?} {})", op, operand),
            Ast::PostfixOp { op, operand } => write!(f, "(<post>{:?} {})", op, operand),
            Ast::BinOp { op, operands } => {
                write!(f, "(<binop>{:?} {} {})", op, operands.0, operands.1)
            }
            Ast::FunctionCall { fun, args } => {
                write!(f, "(<fun>{}", fun)?;
                for a in args {
                    write!(f, " {}", a)?;
                }
                write!(f, ")")
            }
            Ast::Index { arr, idx } => {
                write!(f, "(<idx>{} {})", arr, idx)
            }
        }
    }
}

#[allow(unpredictable_function_pointer_comparisons)] // not really important here
#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum Value {
    #[default]
    Unit,
    Integer(i32),
    Function(fn(&[Value]) -> Value),
    Array(Vec<Value>),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Unit => write!(f, "()"),
            Value::Integer(n) => write!(f, "{}", n),
            Value::Function(fun) => write!(f, "<function {:?}>", fun),
            Value::Array(a) => {
                write!(f, "[")?;
                for (i, a) in a.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", a)?;
                }
                write!(f, "]")
            }
        }
    }
}

#[derive(Debug)]
enum EvalError {
    ExpectedInt(Value),
    ExpectedArray(Value),
    ExpectedFunction(Value),
    OutOfBounds { max: usize, idx: i32 },
    UndefinedVariable(String),
}

impl Ast {
    fn eval(&self, variables: &mut HashMap<String, Value>) -> Result<Value, EvalError> {
        match self {
            Ast::Atom(token) => match token {
                TokenTree::Ident(ident) => variables
                    .get(ident)
                    .ok_or_else(|| EvalError::UndefinedVariable(ident.clone()))
                    .cloned(),
                TokenTree::IntLit(n) => Ok(Value::Integer(*n as _)),
                _ => unreachable!(),
            },
            Ast::ArrayLit(items) => {
                let mut items2 = Vec::with_capacity(items.len());
                for i in items {
                    items2.push(i.eval(variables)?);
                }
                Ok(Value::Array(items2))
            }
            Ast::PrefixOp { op, operand } => match op {
                PrefixOp::Neg => match operand.eval(variables)? {
                    Value::Integer(n) => Ok(Value::Integer(n as _)),
                    v => Err(EvalError::ExpectedInt(v)),
                },
            },
            Ast::PostfixOp { op, operand } => match op {
                PostfixOp::Factorial => {
                    let n = match operand.eval(variables)? {
                        Value::Integer(n) => n,
                        v => return Err(EvalError::ExpectedInt(v)),
                    };

                    let mut out = 1;
                    for i in 1..=n {
                        out *= i;
                    }
                    Ok(Value::Integer(out))
                }
            },
            Ast::BinOp { op, operands } => {
                let lhs = match operands.0.eval(variables)? {
                    Value::Integer(n) => n,
                    v => return Err(EvalError::ExpectedInt(v)),
                };
                let rhs = match operands.1.eval(variables)? {
                    Value::Integer(n) => n,
                    v => return Err(EvalError::ExpectedInt(v)),
                };
                let n = match op {
                    InfixOp::Add => lhs + rhs,
                    InfixOp::Sub => lhs - rhs,
                    InfixOp::Mul => lhs * rhs,
                    InfixOp::Div => lhs / rhs,
                };
                Ok(Value::Integer(n))
            }
            Ast::FunctionCall { fun, args } => {
                let fun = match fun.eval(variables)? {
                    Value::Function(fun) => fun,
                    v => return Err(EvalError::ExpectedFunction(v)),
                };

                let mut args2 = Vec::with_capacity(args.len());
                for i in args {
                    args2.push(i.eval(variables)?);
                }

                Ok(fun(&args2))
            }
            Ast::Index { arr, idx } => {
                let arr = match arr.eval(variables)? {
                    Value::Array(arr) => arr,
                    v => return Err(EvalError::ExpectedArray(v)),
                };
                let idx = match idx.eval(variables)? {
                    Value::Integer(n) => n,
                    v => return Err(EvalError::ExpectedInt(v)),
                };

                arr.get(idx as usize)
                    .ok_or(EvalError::OutOfBounds {
                        max: arr.len(),
                        idx,
                    })
                    .cloned()
            }
        }
    }

    fn print(&self, indent: usize) {
        fn print_indent(indent: usize) {
            print!("{:>gap$}", "", gap = indent * 4);
        }
        match self {
            Ast::Atom(token_tree) => print!("{:?}", token_tree),
            Ast::ArrayLit(asts) => {
                println!("ArrayLit [");
                for a in asts {
                    print_indent(indent + 1);
                    a.print(indent + 1);
                    println!(",");
                }
                print_indent(indent);
                print!("]");
            }
            Ast::PrefixOp { op, operand } => {
                println!("PrefixOp {{");
                print_indent(indent + 1);
                println!("op: {:?},", op);
                print_indent(indent + 1);
                print!("operand: ");
                operand.print(indent + 1);
                println!(",");
                print_indent(indent);
                print!("}}");
            }
            Ast::PostfixOp { op, operand } => {
                println!("PostfixOp {{");
                print_indent(indent + 1);
                println!("op: {:?},", op);
                print_indent(indent + 1);
                print!("operand: ");
                operand.print(indent + 1);
                println!(",");
                print_indent(indent);
                print!("}}");
            }
            Ast::BinOp { op, operands } => {
                println!("BinOp {{");
                print_indent(indent + 1);
                println!("op: {:?},", op);
                print_indent(indent + 1);
                print!("lhs: ");
                operands.0.print(indent + 1);
                println!(",");
                print_indent(indent + 1);
                print!("rhs: ");
                operands.1.print(indent + 1);
                println!(",");
                print_indent(indent);
                print!("}}");
            }
            Ast::FunctionCall { fun, args } => {
                println!("FunctionCall {{");
                print_indent(indent + 1);
                print!("fun: ");
                fun.print(indent + 1);
                println!(",");
                print_indent(indent + 1);
                println!("args: [");
                for a in args {
                    print_indent(indent + 2);
                    a.print(indent + 2);
                    println!(",");
                }
                print_indent(indent + 1);
                println!("],");
                print_indent(indent);
                print!("}}");
            }
            Ast::Index { arr, idx } => {
                println!("Index {{");
                print_indent(indent + 1);
                print!("arr: ");
                arr.print(indent + 1);
                println!(",");
                print_indent(indent + 1);
                print!("idx: ");
                idx.print(indent + 1);
                println!(",");
                print_indent(indent);
                print!("}}");
            }
        }
    }
}

#[derive(Debug, Clone)]
struct Parser<I: Iterator<Item = TokenTree>> {
    lexer: Peekable<I>,
}

impl<I> Parser<I>
where
    I: Iterator<Item = TokenTree>,
{
    fn new(lexer: I) -> Self {
        Self {
            lexer: lexer.peekable(),
        }
    }

    fn parse_comma_sep_exprs(tokens: impl Iterator<Item = TokenTree>) -> Option<Vec<Ast>> {
        let mut exprs = Vec::new();
        let mut parser = Parser::new(tokens);
        while parser.lexer.peek().is_some() {
            exprs.push(parser.parse_expr()?);
            match parser.lexer.next() {
                Some(TokenTree::Punct(Punct::Comma)) => continue,
                Some(tok) => panic!("Unexpected token: {:?}", tok),
                None => break,
            }
        }
        Some(exprs)
    }

    // returns ((), u8) to be clear it's a prefix
    fn prefix_bp(op: &TokenTree) -> Option<((), u8)> {
        match op {
            TokenTree::Punct(Punct::Plus | Punct::Minus) => Some(((), 5)),
            _ => None,
        }
    }

    fn infix_bp(op: &TokenTree) -> Option<(u8, u8)> {
        match op {
            TokenTree::Punct(Punct::Plus | Punct::Minus) => Some((1, 2)),
            TokenTree::Punct(Punct::Star | Punct::Slash) => Some((3, 4)),
            _ => None,
        }
    }

    // returns (u8, ()) to be clear it's a postfix
    fn postfix_bp(op: &TokenTree) -> Option<(u8, ())> {
        match op {
            TokenTree::Punct(Punct::Bang)
            | TokenTree::Group {
                delim: GroupDelim::Paren | GroupDelim::Bracket,
                ..
            } => Some((7, ())),
            _ => None,
        }
    }

    fn parse_bp(&mut self, min_bp: u8) -> Option<Ast> {
        let mut lhs = match self.lexer.next()? {
            TokenTree::Group {
                delim: GroupDelim::Paren,
                tokens,
            } => {
                let mut p = Parser::new(tokens.iter().cloned());
                p.parse_expr()?
            }
            TokenTree::Group {
                delim: GroupDelim::Bracket,
                tokens,
            } => {
                let exprs = Self::parse_comma_sep_exprs(tokens.iter().cloned())?;
                Ast::ArrayLit(exprs)
            }
            tok @ (TokenTree::IntLit(_) | TokenTree::Ident(_)) => Ast::Atom(tok),
            tok if let Some(((), r_bp)) = Self::prefix_bp(&tok) => {
                let rhs = self.parse_bp(r_bp)?;
                Ast::PrefixOp {
                    op: match tok {
                        TokenTree::Punct(Punct::Minus) => PrefixOp::Neg,
                        _ => unreachable!(),
                    },
                    operand: Box::new(rhs),
                }
            }
            tok => panic!("Unexpected token: {:?}", tok),
        };

        loop {
            let op = match self.lexer.peek() {
                None => break,
                Some(TokenTree::Punct(Punct::Comma)) => {
                    break;
                }
                Some(tok @ TokenTree::Punct(p)) if p.is_op() => tok,
                Some(tok @ TokenTree::Group { .. }) => tok,
                tok => panic!("Unexpected token: {:?}", tok),
            };

            if let Some((l_bp, ())) = Self::postfix_bp(op) {
                if l_bp < min_bp {
                    break;
                }
                let op = self.lexer.next().expect("checked above");

                lhs = match op {
                    TokenTree::Group {
                        delim: GroupDelim::Paren,
                        tokens,
                    } => Ast::FunctionCall {
                        fun: Box::new(lhs),
                        args: Self::parse_comma_sep_exprs(tokens.iter().cloned())?,
                    },
                    TokenTree::Group {
                        delim: GroupDelim::Bracket,
                        tokens,
                    } => {
                        let mut parser = Parser::new(tokens.iter().cloned());
                        let idx = parser.parse_expr().unwrap();

                        Ast::Index {
                            arr: Box::new(lhs),
                            idx: Box::new(idx),
                        }
                    }
                    _ => Ast::PostfixOp {
                        op: match op {
                            TokenTree::Punct(Punct::Bang) => PostfixOp::Factorial,
                            _ => unreachable!(),
                        },
                        operand: Box::new(lhs),
                    },
                };
                continue;
            }

            if let Some((l_bp, r_bp)) = Self::infix_bp(op) {
                if l_bp < min_bp {
                    break;
                }

                let op = self.lexer.next().expect("checked above"); // take the peeked item
                let rhs = self.parse_bp(r_bp)?;
                lhs = Ast::BinOp {
                    op: match op {
                        TokenTree::Punct(Punct::Plus) => InfixOp::Add,
                        TokenTree::Punct(Punct::Minus) => InfixOp::Sub,
                        TokenTree::Punct(Punct::Star) => InfixOp::Mul,
                        TokenTree::Punct(Punct::Slash) => InfixOp::Div,
                        _ => unreachable!(),
                    },
                    operands: Box::new((lhs, rhs)),
                };
                continue;
            }

            unreachable!();
        }

        Some(lhs)
    }

    fn parse_expr(&mut self) -> Option<Ast> {
        self.parse_bp(0)
    }
}

fn main() {
    print!("> ");
    let mut variables = HashMap::<String, Value>::from_iter([
        //
        (
            "print".to_string(),
            Value::Function(|a| {
                for (i, a) in a.iter().enumerate() {
                    if i > 0 {
                        print!(" ");
                    }
                    print!("{}", a);
                }
                println!();
                Value::Unit
            }),
        ),
    ]);
    std::io::stdout().flush().unwrap();
    for l in std::io::stdin().lines() {
        let l = l.unwrap();
        if l.trim().is_empty() {
            print!("> ");
            std::io::stdout().flush().unwrap();
            continue;
        }
        let lex = Lexer::new(&l);
        println!("Token Trees:");
        for t in lex {
            println!("  {:?}", t);
        }

        let lex = Lexer::new(&l);
        let mut parser = Parser::new(lex);

        let e = parser.parse_expr().unwrap();
        println!("AST:");
        e.print(0);
        println!();
        match e.eval(&mut variables) {
            Ok(v) => println!("{}", v),
            Err(e) => println!("ERROR: {:?}", e),
        }
        print!("> ");
        std::io::stdout().flush().unwrap();
    }
}

#[cfg(test)]
mod test {
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
}
