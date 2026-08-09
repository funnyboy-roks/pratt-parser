use std::{fmt::Display, io::Write, iter::Peekable};

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
enum TokenTree {
    Ident(String),
    IntLit(u32),
    Punct(Punct),
    ParenGroup { tokens: Vec<TokenTree> },
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
            '(' => TokenTree::ParenGroup {
                tokens: self.take_group(')')?,
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

#[derive(Debug, Clone, Copy)]
enum PrefixOp {
    Neg,
}

#[derive(Debug, Clone, Copy)]
enum PostfixOp {
    Factorial,
}

#[derive(Debug, Clone, Copy)]
enum InfixOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone)]
enum Ast {
    Atom(TokenTree),
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
                TokenTree::Ident(_) => todo!(),
                TokenTree::IntLit(n) => *n as _,
                _ => unreachable!(),
            },
            Ast::PrefixOp { op, operand } => match op {
                PrefixOp::Neg => -operand.eval(),
            },
            Ast::PostfixOp { op, operand } => match op {
                PostfixOp::Factorial => {
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
            },
            Ast::BinOp { op, operands } => match op {
                InfixOp::Add => operands.0.eval() + operands.1.eval(),
                InfixOp::Sub => operands.0.eval() - operands.1.eval(),
                InfixOp::Mul => operands.0.eval() * operands.1.eval(),
                InfixOp::Div => operands.0.eval() / operands.1.eval(),
            },
            Ast::FunctionCall { fun, args } => todo!(),
        };
        println!("  {} => {}", self, v);
        v
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
            TokenTree::Punct(Punct::Bang) => Some((7, ())),
            _ => None,
        }
    }

    fn parse_bp(&mut self, min_bp: u8) -> Option<Ast> {
        let mut lhs = match self.lexer.next()? {
            TokenTree::ParenGroup { tokens } => {
                let mut p = Parser::new(tokens.iter().cloned());
                p.parse_expr()?
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
                tok => panic!("Unexpected token: {:?}", tok),
            };

            if let Some((l_bp, ())) = Self::postfix_bp(op) {
                if l_bp < min_bp {
                    break;
                }
                let op = self.lexer.next().expect("checked above");

                match op {
                    TokenTree::ParenGroup { tokens } => {
                        let mut args = Vec::new();
                        let mut parser = Parser::new(tokens.iter().cloned());
                        while parser.lexer.peek().is_some() {
                            args.push(self.parse_expr()?);
                            match self.lexer.next() {
                                Some(TokenTree::Punct(Punct::Comma)) => continue,
                                Some(tok) => panic!("Unexpected token: {:?}", tok),
                                None => break,
                            }
                        }

                        lhs = Ast::FunctionCall {
                            fun: Box::new(lhs),
                            args,
                        };
                    }
                    _ => {
                        lhs = Ast::PostfixOp {
                            op: match op {
                                TokenTree::Punct(Punct::Bang) => PostfixOp::Factorial,
                                _ => unreachable!(),
                            },
                            operand: Box::new(lhs),
                        };
                    }
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
        println!("Token Trees:");
        for t in lex {
            println!("  {:?}", t);
        }

        let lex = Lexer::new(&l);
        let mut parser = Parser::new(lex);

        let e = parser.parse_expr().unwrap();
        println!("AST: {}", e);
        println!("{}", e.eval());
        print!("> ");
        std::io::stdout().flush().unwrap();
    }
}
