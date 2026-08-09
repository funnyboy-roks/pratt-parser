use std::{collections::HashMap, fmt::Display, io::Write, iter::Peekable};

#[cfg(test)]
mod test;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Keyword {
    Let,
}

impl Keyword {
    pub fn from_ident(ident: &str) -> Option<Self> {
        match ident {
            "let" => Some(Self::Let),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Punct {
    Plus,
    Minus,
    Star,
    Slash,
    Bang,
    Comma,
    Eq,
    FatArrow,
    Semicolon,
}

impl Punct {
    fn is_op(&self) -> bool {
        match self {
            Self::Plus | Self::Minus | Self::Star | Self::Slash | Self::Bang | Self::Eq => true,
            Self::Comma | Self::Semicolon | Self::FatArrow => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GroupDelim {
    /// `()`
    Paren,
    /// `[]`
    Bracket,
    /// `{}`
    Brace,
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
    Keyword(Keyword),
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
            self.skip_whitespace();
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
            c @ ('a'..='z' | 'A'..='Z' | '_') => {
                self.untake_char(c);
                let ident = self.take_ident();
                if let Some(kw) = Keyword::from_ident(ident) {
                    TokenTree::Keyword(kw)
                } else {
                    TokenTree::Ident(ident.into())
                }
            }
            '+' => TokenTree::Punct(Punct::Plus),
            '-' => TokenTree::Punct(Punct::Minus),
            '*' => TokenTree::Punct(Punct::Star),
            '/' => TokenTree::Punct(Punct::Slash),
            '!' => TokenTree::Punct(Punct::Bang),
            ',' => TokenTree::Punct(Punct::Comma),
            '=' => match self.take_char() {
                Some('>') => TokenTree::Punct(Punct::FatArrow),
                Some(c) => {
                    self.untake_char(c);
                    TokenTree::Punct(Punct::Eq)
                }
                None => TokenTree::Punct(Punct::Eq),
            },
            ';' => TokenTree::Punct(Punct::Semicolon),
            '(' => TokenTree::Group {
                delim: GroupDelim::Paren,
                tokens: self.take_group(')')?,
            },
            '[' => TokenTree::Group {
                delim: GroupDelim::Bracket,
                tokens: self.take_group(']')?,
            },
            '{' => TokenTree::Group {
                delim: GroupDelim::Brace,
                tokens: self.take_group('}')?,
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
    Assign,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Block {
    exprs: Vec<Ast>,
    /// whether the last expression should be treated as return value
    ret: bool,
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
    Declare {
        var: String,
        val: Option<Box<Ast>>,
    },
    Block(Block),
    LambdaLit {
        args: Vec<String>,
        block: Block,
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
            Ast::Declare { var, val } => {
                if let Some(val) = val {
                    write!(f, "(<decl>{} {})", var, val)
                } else {
                    write!(f, "(<decl>{})", var)
                }
            }
            Ast::Block(Block { exprs, ret }) => {
                write!(f, "(<block ret={}>", ret)?;
                for a in exprs {
                    write!(f, " {}", a)?;
                }
                write!(f, ")")
            }
            Ast::LambdaLit { args, block } => {
                write!(f, "(<lambda args={:?}>", args)?;
                for a in &block.exprs {
                    write!(f, " {}", a)?;
                }
                write!(f, ")")
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
    NativeFn(fn(&[Value]) -> Value),
    LambdaFn {
        args: Vec<String>,
        body: Block,
    },
    Array(Vec<Value>),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Unit => write!(f, "()"),
            Value::Integer(n) => write!(f, "{}", n),
            Value::NativeFn(fun) => write!(f, "<function {:?}>", fun),
            Value::LambdaFn { .. } => write!(f, "<lambda>"),
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
    ExpectedVar,
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
                let mut int = |ast: &Ast| -> Result<i32, EvalError> {
                    match ast.eval(variables)? {
                        Value::Integer(n) => Ok(n),
                        v => Err(EvalError::ExpectedInt(v)),
                    }
                };
                match op {
                    InfixOp::Add => Ok(Value::Integer(int(&operands.0)? + int(&operands.1)?)),
                    InfixOp::Sub => Ok(Value::Integer(int(&operands.0)? - int(&operands.1)?)),
                    InfixOp::Mul => Ok(Value::Integer(int(&operands.0)? * int(&operands.1)?)),
                    InfixOp::Div => Ok(Value::Integer(int(&operands.0)? / int(&operands.1)?)),
                    InfixOp::Assign => match &operands.0 {
                        Ast::Atom(TokenTree::Ident(ident)) => {
                            let value = operands.1.eval(variables)?;
                            if let Some(var) = variables.get_mut(ident) {
                                *var = value;
                                Ok(Value::Unit)
                            } else {
                                Err(EvalError::UndefinedVariable(ident.clone()))
                            }
                        }
                        Ast::Index { .. } => Err(EvalError::ExpectedVar),
                        _ => Err(EvalError::ExpectedVar),
                    },
                }
            }
            Ast::FunctionCall {
                fun,
                args: call_args,
            } => match fun.eval(variables)? {
                Value::NativeFn(fun) => {
                    let mut args = Vec::with_capacity(call_args.len());
                    for i in call_args {
                        args.push(i.eval(variables)?);
                    }

                    Ok(fun(&args))
                }
                Value::LambdaFn { args, body } => {
                    let mut new_vars = variables.clone();
                    for (i, a) in args.iter().enumerate() {
                        let val = if let Some(a) = call_args.get(i) {
                            a.eval(variables)?
                        } else {
                            Value::Unit
                        };

                        new_vars.insert(a.clone(), val);
                    }

                    let mut last = Value::Unit;
                    for e in &body.exprs {
                        last = e.eval(&mut new_vars)?;
                    }

                    if body.ret { Ok(last) } else { Ok(Value::Unit) }
                }
                v => Err(EvalError::ExpectedFunction(v)),
            },
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
            Ast::Declare { var, val } => {
                let val = if let Some(val) = val {
                    val.eval(variables)?
                } else {
                    Value::Unit
                };
                variables.insert(var.clone(), val);
                Ok(Value::Unit)
            }
            Ast::Block(Block { exprs, ret }) => {
                let mut last = Value::Unit;
                for e in exprs {
                    last = e.eval(variables)?;
                }
                if *ret { Ok(last) } else { Ok(Value::Unit) }
            }
            Ast::LambdaLit { args, block } => Ok(Value::LambdaFn {
                args: args.clone(),
                body: block.clone(),
            }),
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
            Ast::Declare { var, val } => {
                println!("Declare {{");
                print_indent(indent + 1);
                println!("var: {},", var);
                print_indent(indent + 1);
                print!("val: ");
                if let Some(val) = val {
                    val.print(indent + 1);
                } else {
                    print!("None");
                }
                println!(",");
                print_indent(indent);
                print!("}}");
            }
            Ast::Block(Block { exprs, ret }) => {
                println!("Block {{");
                print_indent(indent + 1);
                println!("ret: {},", ret);
                print_indent(indent + 1);
                println!("exprs: [");
                for a in exprs {
                    print_indent(indent + 2);
                    a.print(indent + 2);
                    println!(",");
                }
                print_indent(indent + 1);
                println!("],");
                print_indent(indent);
                print!("}}");
            }
            Ast::LambdaLit { args, block } => {
                println!("LambdaLit {{");
                print_indent(indent + 1);
                println!("args: {:?},", args);
                print_indent(indent + 1);
                println!("ret: {},", block.ret);
                print_indent(indent + 1);
                println!("body: [");
                for a in &block.exprs {
                    print_indent(indent + 2);
                    a.print(indent + 2);
                    println!(",");
                }
                print_indent(indent + 1);
                println!("],");
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
            TokenTree::Punct(Punct::Eq) => Some((0, 1)),
            TokenTree::Punct(Punct::Plus | Punct::Minus) => Some((2, 3)),
            TokenTree::Punct(Punct::Star | Punct::Slash) => Some((4, 5)),
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
            } => match self.lexer.peek() {
                Some(TokenTree::Punct(Punct::FatArrow)) => {
                    self.lexer.next().expect("peeked above");
                    let mut args = Vec::new();
                    let mut parser = Parser::new(tokens.iter().cloned());
                    while let Some(tt) = parser.lexer.next() {
                        let ident = match tt {
                            TokenTree::Ident(ident) => ident,
                            tok => panic!("Unexpected token: {:?}", tok),
                        };
                        args.push(ident);
                        match parser.lexer.next() {
                            Some(TokenTree::Punct(Punct::Comma)) => continue,
                            Some(tok) => panic!("Unexpected token: {:?}", tok),
                            None => break,
                        }
                    }
                    Ast::LambdaLit {
                        args,
                        block: match self.parse_expr()? {
                            Ast::Block(block) => block,
                            e => Block {
                                exprs: vec![e],
                                ret: true,
                            },
                        },
                    }
                }
                _ => {
                    let mut p = Parser::new(tokens.iter().cloned());
                    p.parse_expr()?
                }
            },
            TokenTree::Group {
                delim: GroupDelim::Bracket,
                tokens,
            } => {
                let exprs = Self::parse_comma_sep_exprs(tokens.iter().cloned())?;
                Ast::ArrayLit(exprs)
            }
            TokenTree::Group {
                delim: GroupDelim::Brace,
                tokens,
            } => {
                let mut exprs = Vec::new();
                let mut parser = Parser::new(tokens.iter().cloned());
                let mut ret = true;
                while parser.lexer.peek().is_some() {
                    exprs.push(parser.parse_expr()?);
                    match parser.lexer.next() {
                        Some(TokenTree::Punct(Punct::Semicolon)) => {
                            // if we get a semicolon as the last token, then we don't return a value
                            if parser.lexer.peek().is_none() {
                                ret = false;
                                break;
                            }
                        }
                        Some(tok) => panic!("Unexpected token: {:?}", tok),
                        None => break,
                    }
                }
                Ast::Block(Block { exprs, ret })
            }
            TokenTree::Keyword(Keyword::Let) => {
                let v = match self.lexer.next() {
                    Some(TokenTree::Ident(ident)) => ident,
                    Some(tok) => panic!("Unexpected token: {:?}", tok),
                    None => panic!("Unexpected EOF"),
                };

                match self.lexer.peek() {
                    Some(TokenTree::Punct(Punct::Semicolon)) => {
                        return Some(Ast::Declare { var: v, val: None });
                    }
                    Some(TokenTree::Punct(Punct::Eq)) => {
                        self.lexer.next().unwrap(); // munch eq
                        let expr = self.parse_expr()?;
                        return Some(Ast::Declare {
                            var: v,
                            val: Some(Box::new(expr)),
                        });
                    }
                    Some(tok) => panic!("Unexpected token: {:?}", tok),
                    None => panic!("Unexpected EOF"),
                };
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
                Some(TokenTree::Punct(Punct::Comma | Punct::Semicolon)) => {
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
                        TokenTree::Punct(Punct::Eq) => InfixOp::Assign,
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
    let mut variables: HashMap<String, Value> = HashMap::from_iter([
        //
        (
            "print".to_string(),
            Value::NativeFn(|a| {
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
            println!("    {:?}", t);
        }

        let lex = Lexer::new(&l);
        let mut parser = Parser::new(lex);

        while parser.lexer.peek().is_some() {
            let e = parser.parse_expr().unwrap();
            print!("AST: ");
            e.print(0);
            println!("\n");
            match e.eval(&mut variables) {
                Ok(v) => println!("=> {}", v),
                Err(e) => {
                    println!("ERROR: {:?}", e);
                    break;
                }
            }
            while parser
                .lexer
                .next_if_eq(&TokenTree::Punct(Punct::Semicolon))
                .is_some()
            {}
        }
        print!("> ");
        std::io::stdout().flush().unwrap();
    }
}
