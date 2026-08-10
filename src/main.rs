use std::{
    cell::{Ref, RefCell, RefMut},
    cmp::Ordering,
    collections::HashMap,
    fmt::{Debug, Display},
    io::Write,
    iter::Peekable,
    ops::Deref,
    rc::Rc,
};

#[cfg(test)]
mod test;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Keyword {
    Let,
    True,
    False,
    If,
    Else,
    While,
    For,
    In,
}

impl Keyword {
    pub fn from_ident(ident: &str) -> Option<Self> {
        match ident {
            "let" => Some(Self::Let),
            "true" => Some(Self::True),
            "false" => Some(Self::False),
            "if" => Some(Self::If),
            "else" => Some(Self::Else),
            "while" => Some(Self::While),
            "for" => Some(Self::For),
            "in" => Some(Self::In),
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
    /// `=`
    Eq,
    /// `==`
    EqEq,
    /// `<`
    Lt,
    /// `<=`
    Lte,
    /// `>`
    Gt,
    /// `>=`
    Gte,
    /// `=>`
    FatArrow,
    Semicolon,
    /// `.`
    Dot,
    /// `..`
    DotDot,
    /// `..=`
    DotDotEq,
}

impl Punct {
    fn is_op(&self) -> bool {
        match self {
            Self::Plus
            | Self::Minus
            | Self::Star
            | Self::Slash
            | Self::Bang
            | Self::Eq
            | Self::EqEq
            | Self::Lt
            | Self::Lte
            | Self::Gt
            | Self::Gte
            | Self::Dot
            | Self::DotDot
            | Self::DotDotEq => true,
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
    Ident(Rc<str>),
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
                return Some(out);
            }
            self.untake_char(next);
            let Some(c) = self.next_token() else {
                break;
            };
            out.push(c);
        }

        panic!("Unclosed block");
    }

    fn next_token(&mut self) -> Option<TokenTree> {
        self.skip_whitespace();

        let tt = match self.take_char()? {
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
            '/' if self.peek_char() == Some('/') => {
                let rest = &self.content[self.position..];
                self.position += rest.find('\n').unwrap_or(rest.len());
                self.next_token()?
            }
            '/' => TokenTree::Punct(Punct::Slash),
            '!' => TokenTree::Punct(Punct::Bang),
            ',' => TokenTree::Punct(Punct::Comma),
            '<' => match self.peek_char() {
                Some('=') => {
                    self.take_char();
                    TokenTree::Punct(Punct::Lte)
                }
                _ => TokenTree::Punct(Punct::Lt),
            },
            '>' => match self.peek_char() {
                Some('=') => {
                    self.take_char();
                    TokenTree::Punct(Punct::Gte)
                }
                _ => TokenTree::Punct(Punct::Gt),
            },
            '=' => match self.take_char() {
                Some('=') => TokenTree::Punct(Punct::EqEq),
                Some('>') => TokenTree::Punct(Punct::FatArrow),
                Some(c) => {
                    self.untake_char(c);
                    TokenTree::Punct(Punct::Eq)
                }
                None => TokenTree::Punct(Punct::Eq),
            },
            ';' => TokenTree::Punct(Punct::Semicolon),
            '.' if self.peek_char() == Some('.') => {
                self.take_char();
                if self.peek_char() == Some('=') {
                    self.take_char();
                    TokenTree::Punct(Punct::DotDotEq)
                } else {
                    TokenTree::Punct(Punct::DotDot)
                }
            }
            '.' => TokenTree::Punct(Punct::Dot),
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
        };
        Some(tt)
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
enum Cmp {
    Less,
    LessOrEq,
    GreaterOrEq,
    Greater,
}

impl Cmp {
    fn has_equal(self) -> bool {
        match self {
            Cmp::Less => false,
            Cmp::LessOrEq => true,
            Cmp::GreaterOrEq => true,
            Cmp::Greater => false,
        }
    }

    fn matches(self, ord: Ordering) -> bool {
        match self {
            Cmp::Less => matches!(ord, Ordering::Less),
            Cmp::LessOrEq => matches!(ord, Ordering::Less | Ordering::Equal),
            Cmp::GreaterOrEq => matches!(ord, Ordering::Greater | Ordering::Equal),
            Cmp::Greater => matches!(ord, Ordering::Greater),
        }
    }
}

#[derive(Debug)]
struct Scope {
    parent: Option<Rc<Scope>>,
    vars: RefCell<HashMap<Rc<str>, ValueRef>>,
}

impl Scope {
    fn new() -> Rc<Scope> {
        Rc::new(Scope {
            parent: None,
            vars: Default::default(),
        })
    }

    fn get_var(self: &Rc<Self>, var: &str) -> Option<ValueRef> {
        if let Some(val) = self.vars.borrow().get(var) {
            Some(val.clone())
        } else if let Some(parent) = &self.parent {
            parent.get_var(var).clone()
        } else {
            None
        }
    }

    /// Returns false if the variable was already declared
    fn declare_var(self: &Rc<Self>, var: impl Into<Rc<str>>, value: impl Into<ValueRef>) -> bool {
        self.vars
            .borrow_mut()
            .insert(var.into(), value.into())
            .is_none()
    }

    /// returns Err if the variable was not defined
    fn set_var(self: &Rc<Self>, var: &str, value: impl Into<ValueRef>) -> Result<(), ()> {
        if let Some(val) = self.vars.borrow_mut().get_mut(var) {
            *val = value.into();
            Ok(())
        } else if let Some(parent) = &self.parent {
            parent.set_var(var, value)
        } else {
            Err(())
        }
    }

    fn new_child(self: &Rc<Self>) -> Rc<Scope> {
        Rc::new(Scope {
            parent: Some(Rc::clone(self)),
            vars: RefCell::new(Default::default()),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InfixOp {
    Add,
    Sub,
    Mul,
    Div,
    Assign,
    Equality,
    Cmp(Cmp),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Block {
    exprs: Vec<Ast>,
    /// whether the last expression should be treated as return value
    ret: bool,
}

impl Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        for (i, a) in self.exprs.iter().enumerate() {
            write!(f, "{}", a)?;
            if i < self.exprs.len() - 1 || self.ret {
                write!(f, "; ")?;
            }
        }
        write!(f, "}}")
    }
}

impl Block {
    fn eval(&self, scope: Rc<Scope>) -> Result<ValueRef, EvalError> {
        let mut last = Value::Unit.into();
        for e in &self.exprs {
            last = e.eval(scope.clone())?;
        }
        if self.ret {
            Ok(last)
        } else {
            Ok(Value::Unit.into())
        }
    }
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
        var: Rc<str>,
        val: Option<Box<Ast>>,
    },
    Block(Block),
    LambdaLit {
        args: Vec<Rc<str>>,
        block: Block,
    },
    If {
        cond: Box<Ast>,
        body: Block,
        elze: Option<Block>,
    },
    While {
        cond: Box<Ast>,
        body: Block,
    },
    For {
        var: Rc<str>,
        index: Option<Rc<str>>,
        array: Box<Ast>,
        body: Block,
    },
    Range {
        range: Box<(Ast, Ast)>,
        close_end: bool,
    },
    FieldAccess {
        value: Box<Ast>,
        field: Rc<str>,
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
            Ast::If { cond, body, elze } => {
                write!(f, "(if {} {}", cond, body)?;
                if let Some(elze) = elze {
                    write!(f, " {}", elze)?;
                }
                write!(f, ")")
            }
            Ast::While { cond, body } => {
                write!(f, "(while {} {})", cond, body)
            }
            Ast::For {
                var,
                index,
                array,
                body,
            } => {
                write!(f, "(for {:?}, {:?} in {} {})", var, index, array, body)
            }
            Ast::Range { range, close_end } => {
                write!(
                    f,
                    "({}{}{})",
                    range.0,
                    if *close_end { "..=" } else { ".." },
                    range.1
                )
            }
            Ast::FieldAccess {
                value: object,
                field,
            } => {
                write!(f, "({}.{})", object, field)
            }
        }
    }
}

type NativeFn = Rc<dyn Fn(&[ValueRef]) -> ValueRef>;

#[allow(unpredictable_function_pointer_comparisons)] // not really important here
#[derive(Clone, Default)]
enum Value {
    #[default]
    Unit,
    Bool(bool),
    Integer(i32),
    NativeFn(NativeFn),
    LambdaFn {
        args: Vec<Rc<str>>,
        body: Block,
    },
    Array(Vec<ValueRef>),
    Range {
        range: (ValueRef, ValueRef),
        close_end: bool,
    },
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
            (Self::Integer(l0), Self::Integer(r0)) => l0 == r0,
            (Self::NativeFn(_), Self::NativeFn(_)) => false,
            (
                Self::LambdaFn {
                    args: l_args,
                    body: l_body,
                },
                Self::LambdaFn {
                    args: r_args,
                    body: r_body,
                },
            ) => l_args == r_args && l_body == r_body,
            (Self::Array(l0), Self::Array(r0)) => *l0 == *r0,
            (
                Self::Range {
                    range: l_range,
                    close_end: l_close_end,
                },
                Self::Range {
                    range: r_range,
                    close_end: r_close_end,
                },
            ) => l_range == r_range && l_close_end == r_close_end,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unit => write!(f, "Unit"),
            Self::Bool(arg0) => f.debug_tuple("Bool").field(arg0).finish(),
            Self::Integer(arg0) => f.debug_tuple("Integer").field(arg0).finish(),
            Self::NativeFn(_) => f.debug_tuple("NativeFn").finish_non_exhaustive(),
            Self::LambdaFn { args, body } => f
                .debug_struct("LambdaFn")
                .field("args", args)
                .field("body", body)
                .finish(),
            Self::Array(arg0) => f.debug_tuple("Array").field(arg0).finish(),
            Self::Range { range, close_end } => f
                .debug_struct("Range")
                .field("range", range)
                .field("close_end", close_end)
                .finish(),
        }
    }
}

impl PartialEq<bool> for Value {
    fn eq(&self, other: &bool) -> bool {
        match self {
            Self::Bool(b) => *b == *other,
            _ => false,
        }
    }
}

impl From<()> for Value {
    fn from((): ()) -> Self {
        Value::Unit
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Bool(value)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value::Integer(value)
    }
}

impl From<Vec<ValueRef>> for Value {
    fn from(value: Vec<ValueRef>) -> Self {
        Value::Array(value)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
struct ValueRef(Rc<RefCell<Value>>);

impl Display for ValueRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0.borrow(), f)
    }
}

impl<T> From<T> for ValueRef
where
    Value: From<T>,
{
    fn from(value: T) -> Self {
        Self(Rc::new(RefCell::new(value.into())))
    }
}

impl ValueRef {
    fn borrow(&self) -> Ref<'_, Value> {
        self.0.borrow()
    }

    fn borrow_mut(&self) -> RefMut<'_, Value> {
        self.0.borrow_mut()
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Unit => write!(f, "()"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Integer(n) => write!(f, "{}", n),
            Value::NativeFn(_) => write!(f, "<native function>"),
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
            Value::Range { range, close_end } => {
                write!(
                    f,
                    "{}{}{}",
                    range.0,
                    if *close_end { "..=" } else { ".." },
                    range.1
                )
            }
        }
    }
}

#[expect(unused, reason = "debug printing")]
#[derive(Debug)]
enum EvalError {
    ExpectedVar,
    ExpectedBool(Value),
    ExpectedInt(Value),
    ExpectedArray(Value),
    ExpectedFunction(Value),
    OutOfBounds { max: usize, idx: i32 },
    UndefinedVariable(Rc<str>),
    UnknownField(Value, Rc<str>),
}

impl Ast {
    fn eval(&self, scope: Rc<Scope>) -> Result<ValueRef, EvalError> {
        match self {
            Ast::Atom(token) => match token {
                TokenTree::Ident(ident) => scope
                    .get_var(ident)
                    .ok_or_else(|| EvalError::UndefinedVariable(ident.clone())),
                TokenTree::IntLit(n) => Ok((*n as i32).into()),
                TokenTree::Keyword(Keyword::True) => Ok(true.into()),
                TokenTree::Keyword(Keyword::False) => Ok(false.into()),
                _ => unreachable!(),
            },
            Ast::ArrayLit(items) => {
                let mut items2 = Vec::with_capacity(items.len());
                for i in items {
                    items2.push(i.eval(scope.clone())?);
                }
                Ok(items2.into())
            }
            Ast::PrefixOp { op, operand } => match op {
                PrefixOp::Neg => match &*operand.eval(scope)?.borrow() {
                    Value::Integer(n) => Ok((-n).into()),
                    v => Err(EvalError::ExpectedInt(v.clone())),
                },
            },
            Ast::PostfixOp { op, operand } => match op {
                PostfixOp::Factorial => {
                    let n = match &*operand.eval(scope)?.borrow() {
                        Value::Integer(n) => *n,
                        v => return Err(EvalError::ExpectedInt(v.clone())),
                    };

                    let mut out = 1;
                    for i in 1..=n {
                        out *= i;
                    }
                    Ok(out.into())
                }
            },
            Ast::BinOp { op, operands } => {
                let int = |ast: &Ast| -> Result<i32, EvalError> {
                    match &*ast.eval(scope.clone())?.borrow() {
                        Value::Integer(n) => Ok(*n),
                        v => Err(EvalError::ExpectedInt(v.clone())),
                    }
                };
                match op {
                    InfixOp::Add => Ok((int(&operands.0)? + int(&operands.1)?).into()),
                    InfixOp::Sub => Ok((int(&operands.0)? - int(&operands.1)?).into()),
                    InfixOp::Mul => Ok((int(&operands.0)? * int(&operands.1)?).into()),
                    InfixOp::Div => Ok((int(&operands.0)? / int(&operands.1)?).into()),
                    InfixOp::Assign => match &operands.0 {
                        Ast::Atom(TokenTree::Ident(ident)) => {
                            let value = operands.1.eval(scope.clone())?;
                            if scope.set_var(ident, value).is_ok() {
                                Ok(().into())
                            } else {
                                Err(EvalError::UndefinedVariable(ident.clone()))
                            }
                        }
                        Ast::Index { .. } => Err(EvalError::ExpectedVar),
                        _ => Err(EvalError::ExpectedVar),
                    },
                    InfixOp::Equality => {
                        let lhs = operands.0.eval(scope.clone())?;
                        let rhs = operands.1.eval(scope.clone())?;

                        Ok((lhs == rhs).into())
                    }
                    InfixOp::Cmp(cmp) => {
                        let lhs = operands.0.eval(scope.clone())?;
                        let rhs = operands.1.eval(scope.clone())?;

                        let result = match (&*lhs.borrow(), &*rhs.borrow()) {
                            (Value::Unit, Value::Unit) => cmp.has_equal(),
                            (Value::Bool(l), Value::Bool(r)) => cmp.matches(l.cmp(r)),
                            (Value::Integer(l), Value::Integer(r)) => cmp.matches(l.cmp(r)),
                            (Value::NativeFn(_), Value::NativeFn(_)) => false,
                            (Value::LambdaFn { .. }, Value::LambdaFn { .. }) => false,
                            (Value::Array(_), Value::Array(_)) => {
                                false // TODO
                            }
                            _ => false,
                        };

                        Ok(result.into())
                    }
                }
            }
            Ast::FunctionCall {
                fun,
                args: call_args,
            } => match &*fun.eval(scope.clone())?.borrow() {
                Value::NativeFn(fun) => {
                    let mut args = Vec::with_capacity(call_args.len());
                    for i in call_args {
                        args.push(i.eval(scope.clone())?);
                    }

                    Ok(fun(&args))
                }
                Value::LambdaFn { args, body } => {
                    let new_scope = scope.new_child();
                    for (i, a) in args.iter().enumerate() {
                        let val = if let Some(a) = call_args.get(i) {
                            a.eval(scope.clone())?
                        } else {
                            Value::Unit.into()
                        };

                        new_scope.declare_var(a.clone(), val);
                    }

                    let mut last = Value::Unit.into();
                    for e in &body.exprs {
                        last = e.eval(new_scope.clone())?;
                    }

                    if body.ret {
                        Ok(last)
                    } else {
                        Ok(Value::Unit.into())
                    }
                }
                v => Err(EvalError::ExpectedFunction(v.clone())),
            },
            Ast::Index { arr, idx } => {
                let arr = arr.eval(scope.clone())?;
                let borrow = arr.borrow();
                let arr = match &*borrow {
                    Value::Array(arr) => arr,
                    v => return Err(EvalError::ExpectedArray(v.clone())),
                };
                let idx = match &*idx.eval(scope.clone())?.borrow() {
                    Value::Integer(n) => *n,
                    v => return Err(EvalError::ExpectedInt(v.clone())),
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
                    val.eval(scope.clone())?
                } else {
                    Value::Unit.into()
                };
                scope.declare_var(var.clone(), val);
                Ok(Value::Unit.into())
            }
            Ast::Block(b) => b.eval(scope.clone()),
            Ast::LambdaLit { args, block } => Ok(Value::LambdaFn {
                args: args.clone(),
                body: block.clone(),
            }
            .into()),
            Ast::If { cond, body, elze } => {
                let cond = match &*cond.eval(scope.clone())?.borrow() {
                    Value::Bool(b) => *b,
                    v => return Err(EvalError::ExpectedBool(v.clone())),
                };

                if cond {
                    body.eval(scope.clone())
                } else if let Some(elze) = elze {
                    elze.eval(scope.clone())
                } else {
                    Ok(Value::Unit.into())
                }
            }
            Ast::While { cond, body } => {
                let mut last = Value::Unit.into();
                loop {
                    match &*cond.eval(scope.clone())?.borrow() {
                        Value::Bool(false) => break,
                        Value::Bool(true) => {}
                        v => return Err(EvalError::ExpectedBool(v.clone())),
                    }

                    last = body.eval(scope.clone())?;
                }
                Ok(last)
            }
            Ast::For {
                var,
                index,
                array,
                body,
            } => {
                let a = array.eval(scope.clone())?;
                let bor = a.borrow();
                let array: &mut dyn Iterator<Item = ValueRef> = match bor.deref() {
                    Value::Array(a) => &mut a.iter().cloned(),
                    Value::Range { range, close_end } => {
                        let &start = match &*range.0.borrow() {
                            Value::Integer(n) => n,
                            v => return Err(EvalError::ExpectedInt(v.clone())),
                        };
                        let &end = match &*range.1.borrow() {
                            Value::Integer(n) => n,
                            v => return Err(EvalError::ExpectedInt(v.clone())),
                        };

                        if *close_end {
                            &mut (start..end).map(Into::into)
                        } else {
                            &mut (start..=end).map(Into::into)
                        }
                    }
                    v => return Err(EvalError::ExpectedArray(v.clone())),
                };
                let scope = scope.new_child();

                scope.declare_var(var.clone(), ());
                if let Some(index) = index {
                    scope.declare_var(index.clone(), ());
                }

                let mut last = Value::Unit.into();
                for (i, item) in array.into_iter().enumerate() {
                    scope.set_var(var, item).unwrap();
                    if let Some(index) = index {
                        scope.set_var(index, Value::Integer(i as _)).unwrap();
                    }

                    last = body.eval(scope.clone())?;
                }
                Ok(last)
            }
            Ast::Range { range, close_end } => Ok(Value::Range {
                range: (range.0.eval(scope.clone())?, range.1.eval(scope.clone())?),
                close_end: *close_end,
            }
            .into()),
            Ast::FieldAccess { value, field } => {
                let value = value.eval(scope.clone())?;
                let value_cloned = value.clone();
                match &*value.borrow() {
                    Value::Array(values) if &**field == "len" => Ok((values.len() as i32).into()),
                    Value::Array(values) if &**field == "push" => {
                        Ok(Value::NativeFn(Rc::new(move |a| {
                            match &mut *value_cloned.borrow_mut() {
                                Value::Array(v) => {
                                    v.extend(a.iter().cloned());
                                }
                                _ => unreachable!(),
                            }
                            ().into()
                        }))
                        .into())
                    }
                    v => Err(EvalError::UnknownField(v.clone(), field.clone())),
                }
            }
        }
    }
}

#[expect(unused)]
#[derive(Debug, Clone)]
enum ParseError {
    UnexpectedToken { expected: String, actual: TokenTree },
    UnexpectedEof { expected: String },
}

impl ParseError {
    fn unexpected(expected: impl Into<String>, actual: Option<TokenTree>) -> Self {
        if let Some(actual) = actual {
            ParseError::UnexpectedToken {
                expected: expected.into(),
                actual,
            }
        } else {
            ParseError::UnexpectedEof {
                expected: expected.into(),
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

    fn parse_comma_sep_exprs(
        tokens: impl Iterator<Item = TokenTree>,
    ) -> Result<Vec<Ast>, ParseError> {
        let mut exprs = Vec::new();
        let mut parser = Parser::new(tokens);
        while parser.lexer.peek().is_some() {
            exprs.push(parser.parse_expr()?);
            match parser.lexer.next() {
                Some(TokenTree::Punct(Punct::Comma)) => continue,
                Some(tok) => {
                    return Err(ParseError::unexpected("Comma", Some(tok)));
                }
                None => break,
            }
        }
        Ok(exprs)
    }

    fn parse_block(tokens: impl Iterator<Item = TokenTree>) -> Result<Block, ParseError> {
        let mut exprs = Vec::new();
        let mut parser = Parser::new(tokens);
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
                Some(tok) => {
                    return Err(ParseError::UnexpectedToken {
                        expected: "Comma".into(),
                        actual: tok,
                    });
                }
                None => break,
            }
        }
        Ok(Block { exprs, ret })
    }

    fn take_token(&mut self, tt: &TokenTree) -> Result<(), ParseError> {
        match self.lexer.next() {
            Some(tok) if tok == *tt => Ok(()),
            tok => Err(ParseError::unexpected(format!("{:?}", tt), tok)),
        }
    }

    fn take_ident(&mut self) -> Result<Rc<str>, ParseError> {
        match self.lexer.next() {
            Some(TokenTree::Ident(d)) => Ok(d.clone()),
            tok => Err(ParseError::unexpected("identifier", tok)),
        }
    }

    // TODO: literal numbers for bp is hard

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
            TokenTree::Punct(Punct::EqEq | Punct::Lt | Punct::Lte | Punct::Gte | Punct::Gt) => {
                Some((2, 3))
            }
            TokenTree::Punct(Punct::DotDot | Punct::DotDotEq) => Some((4, 5)),
            TokenTree::Punct(Punct::Plus | Punct::Minus) => Some((6, 7)),
            TokenTree::Punct(Punct::Star | Punct::Slash) => Some((8, 9)),
            _ => None,
        }
    }

    // returns (u8, ()) to be clear it's a postfix
    fn postfix_bp(op: &TokenTree) -> Option<(u8, ())> {
        match op {
            TokenTree::Punct(Punct::Bang | Punct::Dot)
            | TokenTree::Group {
                delim: GroupDelim::Paren | GroupDelim::Bracket,
                ..
            } => Some((7, ())),
            _ => None,
        }
    }

    fn parse_bp(&mut self, min_bp: u8) -> Result<Ast, ParseError> {
        let mut lhs = match self
            .lexer
            .next()
            .ok_or_else(|| ParseError::unexpected("token", None))?
        {
            TokenTree::Group {
                delim: GroupDelim::Paren,
                tokens,
            } => match self.lexer.peek() {
                Some(TokenTree::Punct(Punct::FatArrow)) => {
                    self.lexer.next().expect("peeked above");
                    let mut args = Vec::new();
                    let mut parser = Parser::new(tokens.into_iter());
                    while parser.lexer.peek().is_some() {
                        let ident = parser.take_ident()?;
                        args.push(ident);
                        match parser.lexer.next() {
                            Some(TokenTree::Punct(Punct::Comma)) => continue,
                            Some(tok) => return Err(ParseError::unexpected("Comma", Some(tok))),
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
                    let mut p = Parser::new(tokens.into_iter());
                    p.parse_expr()?
                }
            },
            TokenTree::Group {
                delim: GroupDelim::Bracket,
                tokens,
            } => {
                let exprs = Self::parse_comma_sep_exprs(tokens.into_iter())?;
                Ast::ArrayLit(exprs)
            }
            TokenTree::Group {
                delim: GroupDelim::Brace,
                tokens,
            } => Ast::Block(Self::parse_block(tokens.into_iter())?),
            TokenTree::Keyword(Keyword::Let) => {
                let v = self.take_ident()?;

                return match self.lexer.peek() {
                    Some(TokenTree::Punct(Punct::Semicolon)) => {
                        Ok(Ast::Declare { var: v, val: None })
                    }
                    Some(TokenTree::Punct(Punct::Eq)) => {
                        self.lexer.next().unwrap(); // munch eq
                        let expr = self.parse_expr()?;
                        Ok(Ast::Declare {
                            var: v,
                            val: Some(Box::new(expr)),
                        })
                    }
                    tok => Err(ParseError::unexpected("semicolon or eq", tok.cloned())),
                };
            }
            TokenTree::Keyword(Keyword::If) => {
                let cond = self.parse_expr().unwrap();

                let body = match self.lexer.next() {
                    Some(TokenTree::Group {
                        delim: GroupDelim::Brace,
                        tokens,
                    }) => Self::parse_block(tokens.into_iter())?,
                    tok => return Err(ParseError::unexpected("Block", tok)),
                };

                let elze = if self
                    .lexer
                    .next_if_eq(&TokenTree::Keyword(Keyword::Else))
                    .is_some()
                {
                    let body = match self.lexer.next() {
                        Some(TokenTree::Group {
                            delim: GroupDelim::Brace,
                            tokens,
                        }) => Self::parse_block(tokens.into_iter())?,
                        tok => return Err(ParseError::unexpected("Block", tok)),
                    };
                    Some(body)
                } else {
                    None
                };

                Ast::If {
                    cond: Box::new(cond),
                    body,
                    elze,
                }
            }
            TokenTree::Keyword(Keyword::While) => {
                let cond = self.parse_expr().unwrap();

                let body = match self.lexer.next() {
                    Some(TokenTree::Group {
                        delim: GroupDelim::Brace,
                        tokens,
                    }) => Self::parse_block(tokens.into_iter())?,
                    tok => return Err(ParseError::unexpected("Block", tok)),
                };

                Ast::While {
                    cond: Box::new(cond),
                    body,
                }
            }
            TokenTree::Keyword(Keyword::For) => {
                let var = self.take_ident()?;

                let index = match self.lexer.next() {
                    Some(TokenTree::Keyword(Keyword::In)) => None,
                    Some(TokenTree::Punct(Punct::Comma)) => {
                        let ident = self.take_ident()?;
                        self.take_token(&TokenTree::Keyword(Keyword::In))?;
                        Some(ident)
                    }
                    tok => return Err(ParseError::unexpected("'in' or Comma", tok)),
                };

                let array = self.parse_expr()?;

                let body = match self.lexer.next() {
                    Some(TokenTree::Group {
                        delim: GroupDelim::Brace,
                        tokens,
                    }) => Self::parse_block(tokens.into_iter())?,
                    tok => return Err(ParseError::unexpected("Block", tok)),
                };

                Ast::For {
                    var,
                    index,
                    array: Box::new(array),
                    body,
                }
            }
            tok @ (TokenTree::IntLit(_)
            | TokenTree::Ident(_)
            | TokenTree::Keyword(Keyword::True)
            | TokenTree::Keyword(Keyword::False)) => Ast::Atom(tok),
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
            tok => return Err(ParseError::unexpected("expression", Some(tok))),
        };

        loop {
            let op = match self.lexer.peek() {
                None => break,
                Some(TokenTree::Punct(Punct::Comma | Punct::Semicolon)) => {
                    break;
                }
                Some(tok @ TokenTree::Punct(p)) if p.is_op() => tok,
                Some(tok @ TokenTree::Group { .. }) => tok,
                tok => return Err(ParseError::unexpected("operator", tok.cloned())),
            };

            if let Some((l_bp, ())) = Self::postfix_bp(op) {
                if l_bp < min_bp {
                    break;
                }
                let op = self.lexer.next().expect("checked above");

                lhs = match op {
                    TokenTree::Punct(Punct::Dot) => {
                        let ident = self.take_ident()?;
                        Ast::FieldAccess {
                            value: Box::new(lhs),
                            field: ident,
                        }
                    }
                    TokenTree::Group {
                        delim: GroupDelim::Paren,
                        tokens,
                    } => Ast::FunctionCall {
                        fun: Box::new(lhs),
                        args: Self::parse_comma_sep_exprs(tokens.into_iter())?,
                    },
                    TokenTree::Group {
                        delim: GroupDelim::Bracket,
                        tokens,
                    } => {
                        let mut parser = Parser::new(tokens.into_iter());
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
                lhs = match op {
                    TokenTree::Punct(Punct::DotDot) => Ast::Range {
                        range: Box::new((lhs, rhs)),
                        close_end: false,
                    },
                    TokenTree::Punct(Punct::DotDotEq) => Ast::Range {
                        range: Box::new((lhs, rhs)),
                        close_end: true,
                    },
                    _ => Ast::BinOp {
                        op: match op {
                            TokenTree::Punct(Punct::Plus) => InfixOp::Add,
                            TokenTree::Punct(Punct::Minus) => InfixOp::Sub,
                            TokenTree::Punct(Punct::Star) => InfixOp::Mul,
                            TokenTree::Punct(Punct::Slash) => InfixOp::Div,
                            TokenTree::Punct(Punct::Eq) => InfixOp::Assign,
                            TokenTree::Punct(Punct::EqEq) => InfixOp::Equality,
                            TokenTree::Punct(Punct::Lt) => InfixOp::Cmp(Cmp::Less),
                            TokenTree::Punct(Punct::Lte) => InfixOp::Cmp(Cmp::LessOrEq),
                            TokenTree::Punct(Punct::Gte) => InfixOp::Cmp(Cmp::GreaterOrEq),
                            TokenTree::Punct(Punct::Gt) => InfixOp::Cmp(Cmp::Greater),
                            _ => unreachable!(),
                        },
                        operands: Box::new((lhs, rhs)),
                    },
                };
                continue;
            }

            break;
        }

        Ok(lhs)
    }

    fn parse_expr(&mut self) -> Result<Ast, ParseError> {
        self.parse_bp(0)
    }
}

fn main() {
    let scope = Scope::new();

    scope.declare_var(
        "print",
        Value::NativeFn(Rc::new(|a| {
            for (i, a) in a.iter().enumerate() {
                if i > 0 {
                    print!(" ");
                }
                print!("{}", a);
            }
            println!();
            Value::Unit.into()
        })),
    );
    scope.declare_var("debug", Value::Bool(true));

    print!("> ");
    std::io::stdout().flush().unwrap();
    for l in std::io::stdin().lines() {
        let l = l.unwrap();
        if l.trim().is_empty() {
            print!("> ");
            std::io::stdout().flush().unwrap();
            continue;
        }

        let debug = scope.get_var("debug").is_some_and(|v| *v.borrow() == true);

        if debug {
            let lex = Lexer::new(&l);
            println!("Token Trees:");
            for t in lex {
                println!("    {:?}", t);
            }
        }

        let lex = Lexer::new(&l);
        let mut parser = Parser::new(lex);

        while parser.lexer.peek().is_some() {
            let e = parser.parse_expr().unwrap();
            if debug {
                print!("AST: {:?}", e);
                println!("\n");
            }
            match e.eval(scope.clone()) {
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
