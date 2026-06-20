use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(f64),
    Variable(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LParen,
    RParen,
    Comma,
    Func(String),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Number(f64),
    Variable(String),
    BinaryOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    UnaryMinus(Box<Expr>),
    FuncCall {
        name: String,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
}

const ALLOWED_FUNCS: &[&str] = &[
    "sin", "cos", "tan", "exp", "log", "sqrt", "abs", "asin", "acos", "atan",
    "ceil", "floor", "log2", "log10", "pow",
];

fn is_allowed_func(name: &str) -> bool {
    ALLOWED_FUNCS.contains(&name)
}

pub struct Tokenizer {
    chars: Vec<char>,
    pos: usize,
}

impl Tokenizer {
    pub fn new(input: &str) -> Self {
        Tokenizer {
            chars: input.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied();
        self.pos += 1;
        ch
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        while let Some(ch) = self.peek() {
            match ch {
                ' ' | '\t' | '\n' | '\r' => {
                    self.advance();
                }
                '0'..='9' | '.' => {
                    let mut num_str = String::new();
                    while let Some(c) = self.peek() {
                        if c.is_ascii_digit() || c == '.' {
                            num_str.push(c);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    let val: f64 = num_str.parse().map_err(|_| format!("Invalid number: {}", num_str))?;
                    tokens.push(Token::Number(val));
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    let mut name = String::new();
                    while let Some(c) = self.peek() {
                        if c.is_ascii_alphanumeric() || c == '_' {
                            name.push(c);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    if is_allowed_func(&name) {
                        tokens.push(Token::Func(name));
                    } else {
                        tokens.push(Token::Variable(name));
                    }
                }
                '+' => { self.advance(); tokens.push(Token::Plus); }
                '-' => { self.advance(); tokens.push(Token::Minus); }
                '*' => { self.advance(); tokens.push(Token::Star); }
                '/' => { self.advance(); tokens.push(Token::Slash); }
                '^' => { self.advance(); tokens.push(Token::Caret); }
                '(' => { self.advance(); tokens.push(Token::LParen); }
                ')' => { self.advance(); tokens.push(Token::RParen); }
                ',' => { self.advance(); tokens.push(Token::Comma); }
                _ => return Err(format!("Unexpected character: '{}'", ch)),
            }
        }
        Ok(tokens)
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        let tok = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        tok
    }

    pub fn parse(&mut self) -> Result<Expr, String> {
        let expr = self.parse_addition()?;
        if self.pos < self.tokens.len() {
            return Err("Unexpected token after expression".to_string());
        }
        Ok(expr)
    }

    fn parse_addition(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplication()?;
        loop {
            match self.peek() {
                Some(Token::Plus) => {
                    self.advance();
                    let right = self.parse_multiplication()?;
                    left = Expr::BinaryOp { op: BinOp::Add, left: Box::new(left), right: Box::new(right) };
                }
                Some(Token::Minus) => {
                    self.advance();
                    let right = self.parse_multiplication()?;
                    left = Expr::BinaryOp { op: BinOp::Sub, left: Box::new(left), right: Box::new(right) };
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_multiplication(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_power()?;
        loop {
            match self.peek() {
                Some(Token::Star) => {
                    self.advance();
                    let right = self.parse_power()?;
                    left = Expr::BinaryOp { op: BinOp::Mul, left: Box::new(left), right: Box::new(right) };
                }
                Some(Token::Slash) => {
                    self.advance();
                    let right = self.parse_power()?;
                    left = Expr::BinaryOp { op: BinOp::Div, left: Box::new(left), right: Box::new(right) };
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_power(&mut self) -> Result<Expr, String> {
        let base = self.parse_unary()?;
        if let Some(Token::Caret) = self.peek() {
            self.advance();
            let exp = self.parse_unary()?;
            Ok(Expr::BinaryOp { op: BinOp::Pow, left: Box::new(base), right: Box::new(exp) })
        } else {
            Ok(base)
        }
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some(Token::Minus) => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::UnaryMinus(Box::new(expr)))
            }
            Some(Token::Plus) => {
                self.advance();
                self.parse_unary()
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.peek().cloned() {
            Some(Token::Number(n)) => {
                self.advance();
                Ok(Expr::Number(n))
            }
            Some(Token::Variable(name)) => {
                self.advance();
                Ok(Expr::Variable(name))
            }
            Some(Token::Func(name)) => {
                self.advance();
                match self.peek() {
                    Some(Token::LParen) => {
                        self.advance();
                        let mut args = Vec::new();
                        if !matches!(self.peek(), Some(Token::RParen)) {
                            args.push(self.parse_addition()?);
                            while let Some(Token::Comma) = self.peek() {
                                self.advance();
                                args.push(self.parse_addition()?);
                            }
                        }
                        match self.advance() {
                            Some(Token::RParen) => {}
                            _ => return Err(format!("Expected ')' after function arguments for {}", name)),
                        }
                        Ok(Expr::FuncCall { name, args })
                    }
                    _ => Err(format!("Expected '(' after function name {}", name)),
                }
            }
            Some(Token::LParen) => {
                self.advance();
                let expr = self.parse_addition()?;
                match self.advance() {
                    Some(Token::RParen) => Ok(expr),
                    _ => Err("Expected ')'".to_string()),
                }
            }
            _ => Err("Unexpected token in expression".to_string()),
        }
    }
}

pub fn compile_expr(input: &str) -> Result<Expr, String> {
    let mut tokenizer = Tokenizer::new(input);
    let tokens = tokenizer.tokenize()?;
    let mut parser = Parser::new(tokens);
    parser.parse()
}

pub fn eval_expr(expr: &Expr, vars: &HashMap<String, f64>) -> Result<f64, String> {
    match expr {
        Expr::Number(n) => Ok(*n),
        Expr::Variable(name) => {
            vars.get(name).copied().ok_or_else(|| format!("Undefined variable: {}", name))
        }
        Expr::BinaryOp { op, left, right } => {
            let l = eval_expr(left, vars)?;
            let r = eval_expr(right, vars)?;
            match op {
                BinOp::Add => Ok(l + r),
                BinOp::Sub => Ok(l - r),
                BinOp::Mul => Ok(l * r),
                BinOp::Div => {
                    if r == 0.0 { Ok(f64::INFINITY) } else { Ok(l / r) }
                }
                BinOp::Pow => Ok(l.powf(r)),
            }
        }
        Expr::UnaryMinus(inner) => {
            let v = eval_expr(inner, vars)?;
            Ok(-v)
        }
        Expr::FuncCall { name, args } => {
            match name.as_str() {
                "pow" => {
                    if args.len() != 2 {
                        return Err(format!("pow() requires exactly 2 arguments, got {}", args.len()));
                    }
                    let base = eval_expr(&args[0], vars)?;
                    let exp = eval_expr(&args[1], vars)?;
                    Ok(base.powf(exp))
                }
                _ => {
                    if args.len() != 1 {
                        return Err(format!("{}() requires exactly 1 argument, got {}", name, args.len()));
                    }
                    let v = eval_expr(&args[0], vars)?;
                    match name.as_str() {
                        "sin" => Ok(v.sin()),
                        "cos" => Ok(v.cos()),
                        "tan" => Ok(v.tan()),
                        "exp" => Ok(v.exp()),
                        "log" => Ok(v.ln()),
                        "sqrt" => { if v < 0.0 { Ok(f64::NAN) } else { Ok(v.sqrt()) } }
                        "abs" => Ok(v.abs()),
                        "asin" => Ok(v.asin()),
                        "acos" => Ok(v.acos()),
                        "atan" => Ok(v.atan()),
                        "ceil" => Ok(v.ceil()),
                        "floor" => Ok(v.floor()),
                        "log2" => Ok(v.log2()),
                        "log10" => Ok(v.log10()),
                        _ => Err(format!("Unknown function: {}", name)),
                    }
                }
            }
        }
    }
}

pub struct CompiledExpr {
    ast: Expr,
    var_indices: Vec<(String, usize)>,
}

impl CompiledExpr {
    pub fn new(input: &str, allowed_vars: &[String]) -> Result<Self, String> {
        let ast = compile_expr(input)?;
        let mut var_names = Vec::new();
        collect_vars(&ast, &mut var_names);
        let mut var_indices = Vec::with_capacity(var_names.len());
        for v in &var_names {
            match allowed_vars.iter().position(|x| x == v) {
                Some(idx) => var_indices.push((v.clone(), idx)),
                None => return Err(format!("Unknown variable '{}' in expression. Allowed: {:?}", v, allowed_vars)),
            }
        }
        Ok(CompiledExpr { ast, var_indices })
    }

    pub fn eval(&self, var_values: &[f64]) -> Result<f64, String> {
        let mut vars = HashMap::new();
        for (name, idx) in &self.var_indices {
            if *idx < var_values.len() {
                vars.insert(name.clone(), var_values[*idx]);
            } else {
                return Err(format!("Variable '{}' index {} out of bounds (values len: {})", name, idx, var_values.len()));
            }
        }
        eval_expr(&self.ast, &vars)
    }
}

fn collect_vars(expr: &Expr, vars: &mut Vec<String>) {
    match expr {
        Expr::Variable(name) => {
            if !vars.contains(name) {
                vars.push(name.clone());
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            collect_vars(left, vars);
            collect_vars(right, vars);
        }
        Expr::UnaryMinus(inner) => collect_vars(inner, vars),
        Expr::FuncCall { args, .. } => {
            for a in args {
                collect_vars(a, vars);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_arithmetic() {
        let vars = HashMap::new();
        let expr = compile_expr("2 + 3 * 4").unwrap();
        assert_eq!(eval_expr(&expr, &vars).unwrap(), 14.0);
    }

    #[test]
    fn test_variable() {
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), 5.0);
        let expr = compile_expr("x * 2 + 1").unwrap();
        assert_eq!(eval_expr(&expr, &vars).unwrap(), 11.0);
    }

    #[test]
    fn test_function() {
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), 0.0);
        let expr = compile_expr("sin(x) + cos(x)").unwrap();
        let result = eval_expr(&expr, &vars).unwrap();
        assert!((result - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_power() {
        let vars = HashMap::new();
        let expr = compile_expr("2 ^ 10").unwrap();
        assert_eq!(eval_expr(&expr, &vars).unwrap(), 1024.0);
    }

    #[test]
    fn test_unary_minus() {
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), 3.0);
        let expr = compile_expr("-x + 5").unwrap();
        assert_eq!(eval_expr(&expr, &vars).unwrap(), 2.0);
    }

    #[test]
    fn test_parentheses() {
        let vars = HashMap::new();
        let expr = compile_expr("(2 + 3) * 4").unwrap();
        assert_eq!(eval_expr(&expr, &vars).unwrap(), 20.0);
    }

    #[test]
    fn test_reject_unknown_func() {
        let mut tokenizer = Tokenizer::new("system('rm -rf /')");
        let result = tokenizer.tokenize();
        assert!(result.is_err());
    }

    #[test]
    fn test_variable_order_independence() {
        let allowed = vec!["x".to_string(), "y".to_string()];
        let expr = CompiledExpr::new("y*y + x*x", &allowed).unwrap();
        let values = vec![3.0, 4.0];
        let result = expr.eval(&values).unwrap();
        assert!((result - 25.0).abs() < 1e-10);
    }

    #[test]
    fn test_pow_function() {
        let allowed = vec!["x".to_string(), "y".to_string()];
        let expr = CompiledExpr::new("pow(x, 3) + pow(y, 2)", &allowed).unwrap();
        let values = vec![2.0, 3.0];
        let result = expr.eval(&values).unwrap();
        assert!((result - (8.0 + 9.0)).abs() < 1e-10);
        assert!((result - 17.0).abs() < 1e-10);
    }
}

