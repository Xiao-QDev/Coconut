use super::Token::Token;
use super::Token::TokenWithSpan;

pub struct Lexer {
    source: Vec<char>,
    position: usize,
    line: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            source: source.chars().collect(),
            position: 0,
            line: 1,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<TokenWithSpan>, String> {
        let mut tokens = Vec::new();
        while !self.is_at_end() {
            self.skip_whitespace();

            if self.is_at_end() {
                break;
            }
            let token = self.next_token()?;
            tokens.push(TokenWithSpan {
                token,
                line: self.line,
            });
        }
        tokens.push(TokenWithSpan {
            token: Token::EOF,
            line: self.line,
        });
        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Token, String> {
        let ch = self.advance();
        match ch {
            '(' => Ok(Token::LeftParen),
            ')' => Ok(Token::RightParen),
            '{' => Ok(Token::LeftBrace),
            '}' => Ok(Token::RightBrace),
            ',' => Ok(Token::Comma),
            ';' => Ok(Token::Semicolon),
            '.' => Ok(Token::Dot),
            '&' => Ok(Token::Ampersand),

            '+' => Ok(Token::Plus),
            '-' => Ok(Token::Minus),
            '*' => Ok(Token::Star),
            '/' => Ok(Token::Slash),

            '=' => {
                if !self.is_at_end() && self.current() == '=' {
                    self.advance();
                    Ok(Token::EqualEqual)
                } else {
                    Ok(Token::Equal)
                }
            }

            ':' => {
                if !self.is_at_end() && self.current() == '=' {
                    self.advance();
                    Ok(Token::ColonEqual)
                } else {
                    Err("Unexpected character: :".to_string())
                }
            }

            '!' => {
                if !self.is_at_end() && self.current() == '=' {
                    self.advance();
                    Ok(Token::NotEqual)
                } else {
                    Ok(Token::Bang)
                }
            }
            '<' => {
                if !self.is_at_end() && self.current() == '=' {
                    self.advance();
                    Ok(Token::LessEqual)
                } else {
                    Ok(Token::Less)
                }
            }
            '>' => {
                if !self.is_at_end() && self.current() == '=' {
                    self.advance();
                    Ok(Token::GreaterEqual)
                } else {
                    Ok(Token::Greater)
                }
            }
            '"' => self.read_string(),
            ch if ch.is_ascii_digit() => self.read_number(ch),
            ch if ch.is_alphabetic() || ch == '_' => self.read_identifier(ch),
            _ => Err(format!("Unexpected character: {}", ch)),
        }
    }

    fn read_string(&mut self) -> Result<Token, String> {
        let mut string = String::new();
        while !self.is_at_end() && self.current() != '"' {
            string.push(self.advance());
        }
        if self.is_at_end() {
            return Err("Unterminated string".to_string());
        }
        self.advance();
        Ok(Token::StringLiteral(string))
    }

    fn read_number(&mut self, first: char) -> Result<Token, String> {
        let mut num_str = String::from(first);
        let mut is_float = false;
        while !self.is_at_end() && (self.current().is_ascii_digit() || self.current() == '.') {
            if self.current() == '.' {
                is_float = true;
            }
            num_str.push(self.advance());
        }
        if is_float {
            let value = num_str
                .parse::<f64>()
                .map_err(|_| format!("Invalid float number: {}", num_str))?;
            Ok(Token::FloatLiteral(value))
        } else {
            let value = num_str
                .parse::<i64>()
                .map_err(|_| format!("Invalid number: {}", num_str))?;
            Ok(Token::IntLiteral(value))
        }
    }

    fn read_identifier(&mut self, first: char) -> Result<Token, String> {
        let mut ident = String::from(first);
        while !self.is_at_end() && (self.current().is_alphanumeric() || self.current() == '_') {
            ident.push(self.advance());
        }
        let token = match ident.as_str() {
            "package" => Token::Package,
            "import" => Token::Import,
            "fn" => Token::Fn,
            "var" => Token::Var,
            "const" => Token::Const,
            "if" => Token::If,
            "else" => Token::Else,
            "for" => Token::For,
            "return" => Token::Return,
            "break" => Token::Break,
            "continue" => Token::Continue,
            "extern" => Token::Extern,
            "int" => Token::TypeInt,
            "string" => Token::TypeString,
            "void" => Token::TypeVoid,
            _ => Token::Identifier(ident),
        };
        Ok(token)
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() && self.current().is_whitespace() {
            if self.current() == '\n' {
                self.line += 1;
            }
            self.advance();
        }
    }

    fn advance(&mut self) -> char {
        let ch = self.source[self.position];
        self.position += 1;
        ch
    }

    fn current(&self) -> char {
        self.source[self.position]
    }
    fn is_at_end(&self) -> bool {
        self.position >= self.source.len()
    }
}
