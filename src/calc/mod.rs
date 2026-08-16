//! Terminal Math Calculator & Expression Evaluator for Pico OS
//! Supports standard arithmetic, operator precedence, parentheses, variables, and math functions.

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    LParen,
    RParen,
    Comma,
    Equals,
}

pub struct Lexer<'a> {
    chars: core::str::Chars<'a>,
    peeked: Option<char>,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lex = Lexer {
            chars: input.chars(),
            peeked: None,
        };
        lex.peeked = lex.chars.next();
        lex
    }

    fn advance(&mut self) -> Option<char> {
        let curr = self.peeked;
        self.peeked = self.chars.next();
        curr
    }

    fn peek(&self) -> Option<char> {
        self.peeked
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut raw_tokens = Vec::new();

        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
                continue;
            }

            match c {
                '+' => { self.advance(); raw_tokens.push(Token::Plus); }
                '-' => { self.advance(); raw_tokens.push(Token::Minus); }
                '*' => { self.advance(); raw_tokens.push(Token::Star); }
                '/' => { self.advance(); raw_tokens.push(Token::Slash); }
                '%' => { self.advance(); raw_tokens.push(Token::Percent); }
                '^' => { self.advance(); raw_tokens.push(Token::Caret); }
                '(' => { self.advance(); raw_tokens.push(Token::LParen); }
                ')' => { self.advance(); raw_tokens.push(Token::RParen); }
                ',' => { self.advance(); raw_tokens.push(Token::Comma); }
                '=' => { self.advance(); raw_tokens.push(Token::Equals); }
                '0'..='9' | '.' => {
                    let num = self.read_number()?;
                    raw_tokens.push(Token::Number(num));
                }
                'x' | 'X' => {
                    // Check if 'x' is used as a multiplication operator:
                    // e.g. after a number or ')', or followed by a digit like 50x4
                    let prev_is_term = match raw_tokens.last() {
                        Some(Token::Number(_)) | Some(Token::RParen) => true,
                        _ => false,
                    };

                    self.advance(); // consume 'x'
                    let next_char = self.peek();
                    let next_is_term = match next_char {
                        Some('0'..='9') | Some('.') | Some('(') | Some(' ') | None => true,
                        _ => false,
                    };

                    if prev_is_term && next_is_term {
                        raw_tokens.push(Token::Star);
                    } else {
                        // Otherwise it's a variable or identifier starting with x
                        let mut ident = String::from("x");
                        while let Some(ch) = self.peek() {
                            if ch.is_ascii_alphanumeric() || ch == '_' {
                                ident.push(self.advance().unwrap());
                            } else {
                                break;
                            }
                        }
                        if ident == "x" && prev_is_term {
                            raw_tokens.push(Token::Star);
                        } else {
                            raw_tokens.push(Token::Ident(ident));
                        }
                    }
                }
                'a'..='w' | 'y'..='z' | 'A'..='W' | 'Y'..='Z' | '_' => {
                    let ident = self.read_ident();
                    raw_tokens.push(Token::Ident(ident));
                }
                _ => return Err(format!("Unexpected character: '{}'", c)),
            }
        }

        // Post-processing: Insert implicit multiplication (e.g. 2(3+4) -> 2*(3+4), (2+3)(4+5) -> (2+3)*(4+5))
        let mut tokens = Vec::new();
        for i in 0..raw_tokens.len() {
            let curr = &raw_tokens[i];
            tokens.push(curr.clone());

            if i + 1 < raw_tokens.len() {
                let next = &raw_tokens[i + 1];
                let need_mult = match (curr, next) {
                    (Token::Number(_), Token::LParen) => true,
                    (Token::Number(_), Token::Ident(_)) => true,
                    (Token::RParen, Token::LParen) => true,
                    (Token::RParen, Token::Number(_)) => true,
                    (Token::RParen, Token::Ident(_)) => true,
                    _ => false,
                };
                if need_mult {
                    tokens.push(Token::Star);
                }
            }
        }

        Ok(tokens)
    }

    fn read_number(&mut self) -> Result<f64, String> {
        let mut s = String::new();
        let mut has_dot = false;

        // Check for hex literal 0x... or binary 0b...
        if self.peek() == Some('0') {
            s.push(self.advance().unwrap());
            if let Some(p) = self.peek() {
                if p == 'x' || p == 'X' {
                    s.push(self.advance().unwrap());
                    while let Some(h) = self.peek() {
                        if h.is_ascii_hexdigit() {
                            s.push(self.advance().unwrap());
                        } else {
                            break;
                        }
                    }
                    if s.len() <= 2 {
                        return Err(String::from("Invalid hex number"));
                    }
                    let val = u64::from_str_radix(&s[2..], 16)
                        .map_err(|_| String::from("Hex parse error"))?;
                    return Ok(val as f64);
                } else if p == 'b' || p == 'B' {
                    s.push(self.advance().unwrap());
                    while let Some(b) = self.peek() {
                        if b == '0' || b == '1' {
                            s.push(self.advance().unwrap());
                        } else {
                            break;
                        }
                    }
                    if s.len() <= 2 {
                        return Err(String::from("Invalid binary number"));
                    }
                    let val = u64::from_str_radix(&s[2..], 2)
                        .map_err(|_| String::from("Binary parse error"))?;
                    return Ok(val as f64);
                }
            }
        }

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                s.push(self.advance().unwrap());
            } else if c == '.' {
                if has_dot {
                    break;
                }
                has_dot = true;
                s.push(self.advance().unwrap());
            } else {
                break;
            }
        }

        parse_f64(&s).ok_or_else(|| format!("Invalid number: {}", s))
    }

    fn read_ident(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' {
                s.push(self.advance().unwrap());
            } else {
                break;
            }
        }
        s
    }
}

pub struct CalcContext {
    pub vars: Vec<(String, f64)>,
    pub ans: f64,
}

impl CalcContext {
    pub fn new() -> Self {
        let mut ctx = CalcContext {
            vars: Vec::new(),
            ans: 0.0,
        };
        ctx.set_var("pi", 3.141592653589793);
        ctx.set_var("e", 2.718281828459045);
        ctx
    }

    pub fn get_var(&self, name: &str) -> Option<f64> {
        if name == "ans" {
            return Some(self.ans);
        }
        for (k, v) in &self.vars {
            if k == name {
                return Some(*v);
            }
        }
        None
    }

    pub fn set_var(&mut self, name: &str, val: f64) {
        for (k, v) in &mut self.vars {
            if k == name {
                *v = val;
                return;
            }
        }
        self.vars.push((name.to_string(), val));
    }

    pub fn eval(&mut self, input: &str) -> Result<CalcOutput, String> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Ok(CalcOutput::Empty);
        }
        if trimmed == "help" {
            return Ok(CalcOutput::Help);
        }
        if trimmed == "exit" || trimmed == "quit" || trimmed == "q" {
            return Ok(CalcOutput::Exit);
        }
        if trimmed == "vars" {
            let mut list = Vec::new();
            list.push(format!("ans = {}", format_num(self.ans)));
            for (k, v) in &self.vars {
                list.push(format!("{} = {}", k, format_num(*v)));
            }
            return Ok(CalcOutput::VarList(list));
        }

        let mut lexer = Lexer::new(trimmed);
        let tokens = lexer.tokenize()?;
        if tokens.is_empty() {
            return Ok(CalcOutput::Empty);
        }

        let mut parser = Parser::new(tokens, self);
        parser.parse()
    }
}

#[derive(Debug)]
pub enum CalcOutput {
    Empty,
    Exit,
    Help,
    VarList(Vec<String>),
    Value(f64),
    Assignment(String, f64),
}

pub struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    ctx: &'a mut CalcContext,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Token>, ctx: &'a mut CalcContext) -> Self {
        Parser { tokens, pos: 0, ctx }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let t = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(t)
        } else {
            None
        }
    }

    pub fn parse(&mut self) -> Result<CalcOutput, String> {
        // Check for variable assignment: `ident = expr`
        if self.tokens.len() >= 2 {
            if let (Some(Token::Ident(name)), Some(Token::Equals)) = (self.tokens.get(0), self.tokens.get(1)) {
                let var_name = name.clone();
                self.pos = 2; // skip ident and =
                let val = self.parse_expr()?;
                self.ctx.set_var(&var_name, val);
                self.ctx.ans = val;
                return Ok(CalcOutput::Assignment(var_name, val));
            }
        }

        let val = self.parse_expr()?;
        if self.pos < self.tokens.len() {
            return Err(format!("Unexpected trailing token: {:?}", self.tokens[self.pos]));
        }
        self.ctx.ans = val;
        Ok(CalcOutput::Value(val))
    }

    fn parse_expr(&mut self) -> Result<f64, String> {
        self.parse_additive()
    }

    fn parse_additive(&mut self) -> Result<f64, String> {
        let mut left = self.parse_multiplicative()?;

        while let Some(tok) = self.peek() {
            match tok {
                Token::Plus => {
                    self.advance();
                    let right = self.parse_multiplicative()?;
                    left += right;
                }
                Token::Minus => {
                    self.advance();
                    let right = self.parse_multiplicative()?;
                    left -= right;
                }
                _ => break,
            }
        }

        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<f64, String> {
        let mut left = self.parse_power()?;

        while let Some(tok) = self.peek() {
            match tok {
                Token::Star => {
                    self.advance();
                    let right = self.parse_power()?;
                    left *= right;
                }
                Token::Slash => {
                    self.advance();
                    let right = self.parse_power()?;
                    if right == 0.0 {
                        return Err(String::from("Division by zero"));
                    }
                    left /= right;
                }
                Token::Percent => {
                    self.advance();
                    let right = self.parse_power()?;
                    if right == 0.0 {
                        return Err(String::from("Division by zero in modulo"));
                    }
                    left %= right;
                }
                _ => break,
            }
        }

        Ok(left)
    }

    fn parse_power(&mut self) -> Result<f64, String> {
        let left = self.parse_unary()?;

        if let Some(Token::Caret) = self.peek() {
            self.advance();
            let right = self.parse_power()?; // Right associative
            Ok(pow_f64(left, right))
        } else {
            Ok(left)
        }
    }

    fn parse_unary(&mut self) -> Result<f64, String> {
        if let Some(Token::Minus) = self.peek() {
            self.advance();
            let val = self.parse_unary()?;
            Ok(-val)
        } else if let Some(Token::Plus) = self.peek() {
            self.advance();
            self.parse_unary()
        } else {
            self.parse_primary()
        }
    }

    fn parse_primary(&mut self) -> Result<f64, String> {
        match self.advance() {
            Some(Token::Number(n)) => Ok(n),
            Some(Token::LParen) => {
                let val = self.parse_expr()?;
                match self.advance() {
                    Some(Token::RParen) => Ok(val),
                    _ => Err(String::from("Expected closing parenthesis ')'")),
                }
            }
            Some(Token::Ident(name)) => {
                if let Some(Token::LParen) = self.peek() {
                    self.advance(); // consume '('
                    self.parse_function_call(&name)
                } else if let Some(val) = self.ctx.get_var(&name) {
                    Ok(val)
                } else {
                    Err(format!("Unknown variable or function: '{}'", name))
                }
            }
            Some(tok) => Err(format!("Unexpected token: {:?}", tok)),
            None => Err(String::from("Unexpected end of expression")),
        }
    }

    fn parse_function_call(&mut self, name: &str) -> Result<f64, String> {
        let mut args = Vec::new();
        if let Some(Token::RParen) = self.peek() {
            self.advance();
        } else {
            loop {
                let arg = self.parse_expr()?;
                args.push(arg);
                match self.peek() {
                    Some(Token::Comma) => {
                        self.advance();
                    }
                    Some(Token::RParen) => {
                        self.advance();
                        break;
                    }
                    _ => return Err(String::from("Expected ',' or ')' in function arguments")),
                }
            }
        }

        match name.to_lowercase().as_str() {
            "sqrt" => {
                if args.len() != 1 {
                    return Err(String::from("sqrt() takes exactly 1 argument"));
                }
                if args[0] < 0.0 {
                    return Err(String::from("sqrt() of negative number"));
                }
                Ok(sqrt_f64(args[0]))
            }
            "abs" => {
                if args.len() != 1 {
                    return Err(String::from("abs() takes exactly 1 argument"));
                }
                Ok(if args[0] < 0.0 { -args[0] } else { args[0] })
            }
            "pow" => {
                if args.len() != 2 {
                    return Err(String::from("pow(base, exp) takes exactly 2 arguments"));
                }
                Ok(pow_f64(args[0], args[1]))
            }
            "min" => {
                if args.len() != 2 {
                    return Err(String::from("min(a, b) takes 2 arguments"));
                }
                Ok(if args[0] < args[1] { args[0] } else { args[1] })
            }
            "max" => {
                if args.len() != 2 {
                    return Err(String::from("max(a, b) takes 2 arguments"));
                }
                Ok(if args[0] > args[1] { args[0] } else { args[1] })
            }
            "round" => {
                if args.len() != 1 {
                    return Err(String::from("round() takes 1 argument"));
                }
                Ok(round_f64(args[0]))
            }
            "floor" => {
                if args.len() != 1 {
                    return Err(String::from("floor() takes 1 argument"));
                }
                Ok(floor_f64(args[0]))
            }
            "ceil" => {
                if args.len() != 1 {
                    return Err(String::from("ceil() takes 1 argument"));
                }
                Ok(ceil_f64(args[0]))
            }
            _ => Err(format!("Unknown function: '{}(...)'", name)),
        }
    }
}

/// Newton-Raphson approximation for square root in pure no_std f64
pub fn sqrt_f64(val: f64) -> f64 {
    if val == 0.0 || val == 1.0 {
        return val;
    }
    let mut x = val / 2.0;
    for _ in 0..20 {
        let next_x = 0.5 * (x + val / x);
        if abs_diff(next_x, x) < 1e-12 {
            break;
        }
        x = next_x;
    }
    x
}

fn abs_diff(a: f64, b: f64) -> f64 {
    if a > b { a - b } else { b - a }
}

/// Exponentiation in pure f64
pub fn pow_f64(base: f64, exp: f64) -> f64 {
    if exp == 0.0 {
        return 1.0;
    }
    if base == 0.0 {
        return 0.0;
    }

    // Check if integer power
    let int_exp = exp as i32;
    if abs_diff(int_exp as f64, exp) < 1e-9 {
        let mut res = 1.0;
        let mut b = base;
        let mut e = int_exp.unsigned_abs();
        while e > 0 {
            if e % 2 == 1 {
                res *= b;
            }
            b *= b;
            e /= 2;
        }
        if int_exp < 0 {
            return 1.0 / res;
        } else {
            return res;
        }
    }

    // Fallback for square root/half power
    if abs_diff(exp, 0.5) < 1e-9 {
        return sqrt_f64(base);
    }

    // Basic polynomial approximation for arbitrary fractional powers
    let int_part = exp as i32;
    let frac = exp - int_part as f64;
    let base_int = pow_f64(base, int_part as f64);
    
    // sqrt approximation for fraction
    base_int * (1.0 + frac * (base - 1.0))
}

pub fn floor_f64(x: f64) -> f64 {
    let i = x as i64;
    if x < 0.0 && (x - i as f64) != 0.0 {
        (i - 1) as f64
    } else {
        i as f64
    }
}

pub fn ceil_f64(x: f64) -> f64 {
    let i = x as i64;
    if x > 0.0 && (x - i as f64) != 0.0 {
        (i + 1) as f64
    } else {
        i as f64
    }
}

pub fn round_f64(x: f64) -> f64 {
    if x >= 0.0 {
        floor_f64(x + 0.5)
    } else {
        ceil_f64(x - 0.5)
    }
}

pub fn format_num(val: f64) -> String {
    let int_val = val as i64;
    if abs_diff(int_val as f64, val) < 1e-6 {
        format!("{}", int_val)
    } else {
        // Format with up to 4 decimals, trim trailing zeroes
        let s = format!("{:.4}", val);
        let trimmed = s.trim_end_matches('0').trim_end_matches('.');
        String::from(trimmed)
    }
}

fn parse_f64(s: &str) -> Option<f64> {
    let mut parts = s.split('.');
    let int_part_str = parts.next()?;
    let int_part: i64 = int_part_str.parse().ok()?;

    if let Some(frac_str) = parts.next() {
        if parts.next().is_some() {
            return None; // multiple dots
        }
        let mut frac_val: f64 = 0.0;
        let mut div: f64 = 10.0;
        for c in frac_str.chars() {
            let digit = c.to_digit(10)? as f64;
            frac_val += digit / div;
            div *= 10.0;
        }
        if int_part < 0 || int_part_str.starts_with('-') {
            Some(int_part as f64 - frac_val)
        } else {
            Some(int_part as f64 + frac_val)
        }
    } else {
        Some(int_part as f64)
    }
}
