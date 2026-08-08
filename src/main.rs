use std::{
    fmt::Display,
    io::{BufRead, Write},
    iter::Peekable,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Ident(String),
    IntLit(u32),
    Plus,
    Minus,
    Star,
    Slash,
    Bang,
    LParen,
    RParen,
    Comma,
}

impl Token {
    fn is_op(&self) -> bool {
        match self {
            Token::Ident(_) | Token::IntLit(_) => false,
            Token::Plus
            | Token::Minus
            | Token::Star
            | Token::Slash
            | Token::Bang
            | Token::LParen
            | Token::RParen
            | Token::Comma => true,
        }
    }
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

    fn peek_char(&self) -> Option<char> {
        self.content[self.position..].chars().next()
    }

    fn take_char(&mut self) -> Option<char> {
        let c = self.content[self.position..].chars().next()?;
        self.position += c.len_utf8();
        Some(c)
    }

    fn untake_char(&mut self, c: char) {
        self.position -= c.len_utf8();
        assert_eq!(self.peek_char(), Some(c));
    }

    fn take_number(&mut self) -> Token {
        let rest = &self.content[self.position..];
        let idx = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        self.position += idx;
        let num = &rest[..idx];
        let num = num.parse::<u32>().expect("TODO");
        Token::IntLit(num)
    }

    fn take_ident(&mut self) -> &'a str {
        let rest = &self.content[self.position..];
        let idx = rest
            .find(|c: char| !matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_'))
            .unwrap_or(rest.len());
        self.position += idx;
        &rest[..idx]
    }

    fn next_token(&mut self) -> Option<Token> {
        self.skip_whitespace();

        Some(match self.take_char()? {
            c @ ('a'..='z' | 'A'..='Z') => {
                self.untake_char(c);
                Token::Ident(self.take_ident().into())
            }
            '+' => Token::Plus,
            '-' => Token::Minus,
            '*' => Token::Star,
            '/' => Token::Slash,
            '!' => Token::Bang,
            '(' => Token::LParen,
            ')' => Token::RParen,
            ',' => Token::Comma,
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
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}

#[derive(Debug, Clone)]
enum Ast {
    Atom(Token),
    PrefixOp {
        op: Token,
        operand: Box<Ast>,
    },
    PostfixOp {
        op: Token,
        operand: Box<Ast>,
    },
    BinOp {
        op: Token,
        operands: Box<(Ast, Ast)>,
    },
    FunctionCall {
        fun: Box<Ast>,
        args: Vec<Ast>,
    },
}

impl Display for Ast {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ast::Atom(token) => write!(f, "{:?}", token),
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
        }
    }
}

impl Ast {
    fn eval(&self) -> i32 {
        let v = match self {
            Ast::Atom(token) => match token {
                Token::Ident(_) => todo!(),
                Token::IntLit(n) => *n as _,
                _ => unreachable!(),
            },
            Ast::PrefixOp { op, operand } => match op {
                Token::Plus => operand.eval(),
                Token::Minus => -operand.eval(),
                _ => unreachable!(),
            },
            Ast::PostfixOp { op, operand } => match op {
                Token::Bang => {
                    let n = operand.eval();
                    if n <= 0 {
                        return 1;
                    }
                    let mut out = 1;
                    for i in 1..=n {
                        out *= i;
                    }
                    out
                }
                _ => unreachable!(),
            },
            Ast::BinOp { op, operands } => match op {
                Token::Plus => operands.0.eval() + operands.1.eval(),
                Token::Minus => operands.0.eval() - operands.1.eval(),
                Token::Star => operands.0.eval() * operands.1.eval(),
                Token::Slash => operands.0.eval() / operands.1.eval(),
                _ => unreachable!(),
            },
            Ast::FunctionCall { fun, args } => todo!(),
        };
        println!("  {} => {}", self, v);
        v
    }
}

#[derive(Debug, Clone)]
struct Parser<'a> {
    lexer: Peekable<Lexer<'a>>,
}

impl<'a> Parser<'a> {
    fn new(lexer: Lexer<'a>) -> Self {
        Self {
            lexer: lexer.peekable(),
        }
    }

    // returns ((), u8) to be clear it's a prefix
    fn prefix_bp(op: &Token) -> Option<((), u8)> {
        match op {
            Token::Plus | Token::Minus => Some(((), 5)),
            _ => None,
        }
    }

    fn infix_bp(op: &Token) -> Option<(u8, u8)> {
        match op {
            Token::Plus | Token::Minus => Some((1, 2)),
            Token::Star | Token::Slash => Some((3, 4)),
            _ => None,
        }
    }

    // returns (u8, ()) to be clear it's a postfix
    fn postfix_bp(op: &Token) -> Option<(u8, ())> {
        match op {
            Token::Bang | Token::LParen => Some((7, ())),
            _ => None,
        }
    }

    fn parse_bp(&mut self, min_bp: u8) -> Option<Ast> {
        let mut lhs = match self.lexer.next()? {
            Token::LParen => {
                let lhs = self.parse_expr()?;
                assert_eq!(self.lexer.next(), Some(Token::RParen));
                lhs
            }
            tok @ (Token::IntLit(_) | Token::Ident(_)) => Ast::Atom(tok),
            tok @ (Token::Plus | Token::Minus | Token::Star | Token::Slash)
                if let Some(((), r_bp)) = Self::prefix_bp(&tok) =>
            {
                let rhs = self.parse_bp(r_bp)?;
                Ast::PrefixOp {
                    op: tok,
                    operand: Box::new(rhs),
                }
            }
            tok => panic!("Unexpected token: {:?}", tok),
        };

        loop {
            let op = match self.lexer.peek() {
                None => break,
                Some(Token::RParen | Token::Comma) => {
                    break;
                }
                Some(tok) if tok.is_op() => tok,
                tok => panic!("Unexpected token: {:?}", tok),
            };

            if let Some((l_bp, ())) = Self::postfix_bp(op) {
                if l_bp < min_bp {
                    break;
                }
                let op = self.lexer.next().expect("checked above");

                if op == Token::LParen {
                    let mut args = Vec::new();
                    while let Some(peek) = self.lexer.peek()
                        && *peek != Token::RParen
                    {
                        args.push(self.parse_expr()?);
                        match self.lexer.next() {
                            Some(Token::Comma) => continue,
                            Some(Token::RParen) => break,
                            Some(tok) => panic!("Unexpected token: {:?}", tok),
                            None => panic!("Unexpected EOF"),
                        }
                    }

                    lhs = Ast::FunctionCall {
                        fun: Box::new(lhs),
                        args,
                    };
                } else {
                    lhs = Ast::PostfixOp {
                        op,
                        operand: Box::new(lhs),
                    };
                }
                continue;
            }

            if let Some((l_bp, r_bp)) = Self::infix_bp(op) {
                if l_bp < min_bp {
                    break;
                }

                let op = self.lexer.next().expect("checked above"); // take the peeked item
                let rhs = self.parse_bp(r_bp)?;
                lhs = Ast::BinOp {
                    op,
                    operands: Box::new((lhs, rhs)),
                };
                continue;
            }

            unreachable!()
        }

        Some(lhs)
    }

    fn parse_expr(&mut self) -> Option<Ast> {
        self.parse_bp(0)
    }
}

fn main() {
    print!("> ");
    std::io::stdout().flush().unwrap();
    for l in std::io::stdin().lines() {
        let l = l.unwrap();
        let lex = Lexer::new(&l);
        let mut parser = Parser::new(lex);

        let e = parser.parse_expr().unwrap();
        println!("{}", e);
        println!("{}", e.eval());
        print!("> ");
        std::io::stdout().flush().unwrap();
    }
}
