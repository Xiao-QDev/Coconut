use super::AST::*;
use crate::Lexer::token::Token;
use crate::Lexer::token::TokenWithSpan;

pub struct Parser {
    tokens: Vec<TokenWithSpan>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<TokenWithSpan>) -> Self {
        Parser { tokens, current: 0 }
    }

    pub fn parse(&mut self) -> Result<Program, String> {
        let package = self.parse_package()?;
        let mut imports = Vec::new();
        while self.check(Token::Import) {
            imports.push(self.parse_import()?);
        }
        let mut global_vars = Vec::new();
        let mut functions = Vec::new();
        let mut structs = Vec::new();
        while !self.is_at_end() {
            if self.check(Token::Var) || self.check(Token::Const) {
                if self.check(Token::Var) {
                    let stmt = self.parse_var_decl()?;
                    match stmt {
                        Statement::VarDecl(var_decl) => global_vars.push(var_decl),
                        Statement::MultiVarDecl(mvd) => {
                            for (name, value) in mvd.names.iter().zip(mvd.values.iter()) {
                                global_vars.push(VarDecl {
                                    name: name.clone(),
                                    var_type: mvd.var_type.clone(),
                                    value: value.clone(),
                                });
                            }
                        }
                        _ => return Err("Unexpected statement at global level".to_string()),
                    }
                } else {
                    self.advance();
                }
            } else if self.check(Token::Struct) {
                structs.push(self.parse_struct_def()?);
            } else {
                functions.push(self.parse_function()?);
            }
        }
        Ok(Program {
            package,
            imports,
            global_vars,
            functions,
            structs,
        })
    }

    fn parse_import(&mut self) -> Result<Import, String> {
        self.consume(Token::Import, "Expected 'import'")?;
        let path = if let Token::StringLiteral(s) = &self.peek().token {
            let s = s.clone();
            self.advance();
            s
        } else {
            return Err("Expected string literal for import path".to_string());
        };
        let alias = if matches!(&self.peek().token, Token::Identifier(_)) {
            Some(self.consume_identifier()?)
        } else {
            None
        };
        Ok(Import { path, alias })
    }

    fn parse_package(&mut self) -> Result<String, String> {
        self.consume(Token::Package, "Expected 'package'")?;
        let name = self.consume_identifier()?;
        Ok(name)
    }

    fn parse_function(&mut self) -> Result<Function, String> {
        self.consume(Token::Fn, "Expected 'fn'")?;
        let name = self.consume_identifier()?;
        self.consume(Token::LeftParen, "Expected '('")?;
        let params = self.parse_parameters()?;
        self.consume(Token::RightParen, "Expected ')'")?;
        let return_type = if !self.check(Token::LeftBrace) {
            Some(self.parse_type()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        Ok(Function {
            name,
            params,
            return_type,
            body,
        })
    }

    fn parse_parameters(&mut self) -> Result<Vec<Parameter>, String> {
        let mut params = Vec::new();
        if !self.check(Token::RightParen) {
            loop {
                let name = self.consume_identifier()?;
                let param_type = self.parse_type()?;
                params.push(Parameter { name, param_type });
                if !self.match_token(Token::Comma) {
                    break;
                }
            }
        }
        Ok(params)
    }

    fn parse_type(&mut self) -> Result<Type, String> {
        if self.check(Token::LeftBracket) {
            self.advance();
            let size = if let Token::IntLiteral(n) = &self.peek().token {
                *n as usize
            } else {
                return Err("Expected array size".to_string());
            };
            self.advance();
            self.consume(Token::RightBracket, "Expected ']'")?;
            let elem_type = self.parse_type()?;
            return Ok(Type::Array(Box::new(elem_type), size));
        }
        let token = self.advance();
        match &token.token {
            Token::TypeInt => Ok(Type::Int),
            Token::TypeFloat => Ok(Type::Float),
            Token::TypeString => Ok(Type::String),
            Token::TypeBool => Ok(Type::Bool),
            Token::TypeVoid => Ok(Type::Void),
            Token::Identifier(name) => Ok(Type::Struct(name.clone())),
            _ => Err(format!("Expected type, got {:?}", token.token)),
        }
    }

    fn parse_block(&mut self) -> Result<Block, String> {
        self.consume(Token::LeftBrace, "Expected '{'")?;
        let mut statements = Vec::new();
        while !self.check(Token::RightBrace) && !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }
        self.consume(Token::RightBrace, "Expected '}'")?;
        Ok(Block { statements })
    }

    fn parse_statement(&mut self) -> Result<Statement, String> {
        if self.check(Token::Return) {
            return self.parse_return();
        }
        if self.check(Token::Var) {
            return self.parse_var_decl();
        }
        if self.check(Token::If) {
            return self.parse_if_stmt().map(Statement::If);
        }
        if self.check(Token::For) {
            return self.parse_for_stmt();
        }
        if self.check(Token::While) {
            return self.parse_while_stmt().map(Statement::While);
        }
        if self.check(Token::Switch) {
            return self.parse_switch_stmt().map(Statement::Switch);
        }
        if self.check(Token::Break) {
            self.advance();
            return Ok(Statement::Break);
        }
        if self.check(Token::Continue) {
            self.advance();
            return Ok(Statement::Continue);
        }
        if self.peek_short_decl() {
            return self.parse_short_decl().map(Statement::ShortDecl);
        }
        if self.peek_multi_short_decl() {
            return self.parse_multi_short_decl().map(Statement::MultiShortDecl);
        }
        let expr = self.parse_expression()?;
        if self.check(Token::Equal) {
            return self.parse_assign_from(expr).map(Statement::Assign);
        }
        if self.check(Token::PlusPlus) {
            if !self.is_valid_lvalue(&expr) {
                return Err("Invalid increment target".to_string());
            }
            self.advance();
            return Ok(Statement::IncDec(expr, true));
        }
        if self.check(Token::MinusMinus) {
            if !self.is_valid_lvalue(&expr) {
                return Err("Invalid decrement target".to_string());
            }
            self.advance();
            return Ok(Statement::IncDec(expr, false));
        }
        Ok(Statement::Expression(expr))
    }

    fn is_valid_lvalue(&self, expr: &Expr) -> bool {
        matches!(
            expr,
            Expr::Identifier(_) | Expr::ArrayAccess(_, _) | Expr::FieldAccess(_, _)
        )
    }

    fn parse_assign_from(&mut self, target: Expr) -> Result<AssignStmt, String> {
        if !self.is_valid_lvalue(&target) {
            return Err(format!("Invalid assignment target: {:?}", target));
        }
        self.consume(Token::Equal, "Expected '='")?;
        let value = self.parse_expression()?;
        Ok(AssignStmt { target, value })
    }

    fn peek_assign(&self) -> bool {
        if let Token::Identifier(_) = &self.peek().token {
            if self.current + 1 < self.tokens.len() {
                return self.tokens[self.current + 1].token == Token::Equal;
            }
        }
        false
    }

    fn parse_assign(&mut self) -> Result<AssignStmt, String> {
        let target = Expr::Identifier(self.consume_identifier()?);
        self.consume(Token::Equal, "Expected '='")?;
        let value = self.parse_expression()?;
        Ok(AssignStmt { target, value })
    }

    fn parse_while_stmt(&mut self) -> Result<WhileStmt, String> {
        self.consume(Token::While, "Expected 'while'")?;
        let condition = self.parse_expression()?;
        let body = self.parse_block()?;
        Ok(WhileStmt { condition, body })
    }

    fn parse_for_stmt(&mut self) -> Result<Statement, String> {
        self.consume(Token::For, "Expected 'for'")?;
        if self.check(Token::LeftBrace) {
            let body = self.parse_loop_body()?;
            return Ok(Statement::For(ForStmt {
                init: None,
                condition: None,
                step: None,
                body,
            }));
        }
        if self.peek_short_decl() {
            let init = self.parse_short_decl().map(Statement::ShortDecl)?;
            self.consume(Token::Semicolon, "Expected ';' after for init")?;
            let condition = self.parse_expression()?;
            self.consume(Token::Semicolon, "Expected ';' after for condition")?;
            let step = self.parse_for_step()?;
            let body = self.parse_loop_body()?;
            return Ok(Statement::For(ForStmt {
                init: Some(Box::new(init)),
                condition: Some(condition),
                step: Some(Box::new(step)),
                body,
            }));
        }

        if self.peek_assign() {
            let init = self.parse_assign().map(Statement::Assign)?;
            self.consume(Token::Semicolon, "Expected ';' after for init")?;
            let condition = self.parse_expression()?;
            self.consume(Token::Semicolon, "Expected ';' after for condition")?;
            let step = self.parse_for_step()?;
            let body = self.parse_loop_body()?;
            return Ok(Statement::For(ForStmt {
                init: Some(Box::new(init)),
                condition: Some(condition),
                step: Some(Box::new(step)),
                body,
            }));
        }
        let condition = self.parse_expression()?;
        if self.check(Token::LeftBrace) {
            let body = self.parse_loop_body()?;
            return Ok(Statement::For(ForStmt {
                init: None,
                condition: Some(condition),
                step: None,
                body,
            }));
        }
        self.consume(Token::Semicolon, "Expected ';' after for condition")?;
        let cond2 = self.parse_expression()?;
        self.consume(Token::Semicolon, "Expected ';' after for condition")?;
        let step = self.parse_for_step()?;
        let body = self.parse_loop_body()?;
        Ok(Statement::For(ForStmt {
            init: Some(Box::new(Statement::Expression(condition))),
            condition: Some(cond2),
            step: Some(Box::new(step)),
            body,
        }))
    }

    fn parse_for_step(&mut self) -> Result<Statement, String> {
        if self.peek_short_decl() {
            return self.parse_short_decl().map(Statement::ShortDecl);
        }
        if self.peek_multi_short_decl() {
            return self.parse_multi_short_decl().map(Statement::MultiShortDecl);
        }
        let expr = self.parse_expression()?;
        if self.check(Token::Equal) {
            return if self.is_valid_lvalue(&expr) {
                self.advance();
                let value = self.parse_expression()?;
                Ok(Statement::Assign(AssignStmt {
                    target: expr,
                    value,
                }))
            } else {
                Err("Invalid assignment target in for step".to_string())
            };
        }
        if self.check(Token::PlusPlus) {
            if !self.is_valid_lvalue(&expr) {
                return Err("Invalid increment target".to_string());
            }
            self.advance();
            return Ok(Statement::IncDec(expr, true));
        }
        if self.check(Token::MinusMinus) {
            if !self.is_valid_lvalue(&expr) {
                return Err("Invalid decrement target".to_string());
            }
            self.advance();
            return Ok(Statement::IncDec(expr, false));
        }
        Ok(Statement::Expression(expr))
    }

    fn parse_loop_body(&mut self) -> Result<Block, String> {
        self.consume(Token::LeftBrace, "Expected '{'")?;
        let mut statements = Vec::new();
        while !self.check(Token::RightBrace) && !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }
        self.consume(Token::RightBrace, "Expected '}'")?;
        Ok(Block { statements })
    }

    fn peek_short_decl(&self) -> bool {
        if let Token::Identifier(_) = &self.peek().token {
            if self.current + 1 < self.tokens.len() {
                return self.tokens[self.current + 1].token == Token::ColonEqual;
            }
        }
        false
    }

    fn peek_multi_short_decl(&self) -> bool {
        if let Token::Identifier(_) = &self.peek().token {
            let mut i = self.current + 1;
            while i < self.tokens.len() {
                if self.tokens[i].token == Token::Comma {
                    i += 1;
                    if i < self.tokens.len() {
                        if let Token::Identifier(_) = &self.tokens[i].token {
                            i += 1;
                        } else {
                            return false;
                        }
                    } else {
                        return false;
                    }
                } else if self.tokens[i].token == Token::ColonEqual {
                    return true;
                } else {
                    return false;
                }
            }
        }
        false
    }

    fn parse_short_decl(&mut self) -> Result<ShortDecl, String> {
        let name = self.consume_identifier()?;
        self.consume(Token::ColonEqual, "Expected ':='")?;
        let value = self.parse_expression()?;
        Ok(ShortDecl { name, value })
    }

    fn parse_multi_short_decl(&mut self) -> Result<MultiShortDecl, String> {
        let mut names = vec![self.consume_identifier()?];
        while self.match_token(Token::Comma) {
            names.push(self.consume_identifier()?);
        }
        self.consume(Token::ColonEqual, "Expected ':='")?;
        let mut values = vec![self.parse_expression()?];
        while self.match_token(Token::Comma) {
            values.push(self.parse_expression()?);
        }
        if names.len() != values.len() {
            return Err(format!(
                "Variable count ({}) doesn't match value count ({})",
                names.len(),
                values.len()
            ));
        }
        Ok(MultiShortDecl { names, values })
    }

    fn parse_if_stmt(&mut self) -> Result<IfStmt, String> {
        self.consume(Token::If, "Expected 'if'")?;
        let condition = if self.check(Token::LeftParen) {
            self.advance();
            let cond = self.parse_expression()?;
            self.consume(Token::RightParen, "Expected ')'")?;
            cond
        } else {
            self.parse_expression()?
        };
        let then_block = self.parse_block()?;
        let else_block = if self.check(Token::Else) {
            self.advance();
            if self.check(Token::If) {
                let nested_if = self.parse_if_stmt()?;
                Some(Block {
                    statements: vec![Statement::If(nested_if)],
                })
            } else {
                Some(self.parse_block()?)
            }
        } else {
            None
        };
        Ok(IfStmt {
            condition,
            then_block,
            else_block,
        })
    }

    fn parse_var_decl(&mut self) -> Result<Statement, String> {
        self.consume(Token::Var, "Expected 'var'")?;
        let first_name = self.consume_identifier()?;
        if self.check(Token::Comma) {
            return self.parse_multi_var_decl(first_name);
        }
        if self.check(Token::ColonEqual) {
            self.advance();
            let value = self.parse_expression()?;
            return Ok(Statement::ShortDecl(ShortDecl {
                name: first_name,
                value,
            }));
        }
        let var_type = if !self.check(Token::Equal) {
            self.parse_type()?
        } else {
            Type::Int
        };
        let value = if self.check(Token::Equal) {
            self.advance();
            self.parse_expression()?
        } else {
            match var_type {
                Type::Int => Expr::Literal(Literal::Int(0)),
                Type::Float => Expr::Literal(Literal::Float(0.0)),
                Type::String => Expr::Literal(Literal::String("".to_string())),
                Type::Bool => Expr::Literal(Literal::Bool(false)),
                Type::Array(_, _) => Expr::Literal(Literal::Int(0)),
                Type::Struct(_) => Expr::Literal(Literal::Int(0)),
                Type::Void => return Err("void type not allowed for variables".to_string()),
            }
        };
        Ok(Statement::VarDecl(VarDecl {
            name: first_name,
            var_type,
            value,
        }))
    }

    fn parse_multi_var_decl(&mut self, first_name: String) -> Result<Statement, String> {
        let mut names = vec![first_name];
        while self.match_token(Token::Comma) {
            names.push(self.consume_identifier()?);
        }
        let var_type = self.parse_type()?;
        self.consume(Token::Equal, "Expected '=' in multi-variable declaration")?;
        let first_value = self.parse_expression()?;
        let mut values = vec![first_value];
        while self.match_token(Token::Comma) {
            values.push(self.parse_expression()?);
        }
        if names.len() != values.len() {
            return Err(format!(
                "Variable count ({}) doesn't match value count ({})",
                names.len(),
                values.len()
            ));
        }
        Ok(Statement::MultiVarDecl(MultiVarDecl {
            names,
            var_type,
            values,
        }))
    }

    fn parse_switch_stmt(&mut self) -> Result<SwitchStmt, String> {
        self.consume(Token::Switch, "Expected 'switch'")?;
        let expression = self.parse_expression()?;
        self.consume(Token::LeftBrace, "Expected '{'")?;
        let mut cases = Vec::new();
        let mut default_block = None;
        while !self.check(Token::RightBrace) && !self.is_at_end() {
            if self.check(Token::Case) {
                self.advance();
                let value = self.parse_expression()?;
                let block = self.parse_block()?;
                cases.push(SwitchCase {
                    condition: Some(value),
                    block,
                });
            } else if self.check(Token::Default) {
                self.advance();
                default_block = Some(self.parse_block()?);
            } else {
                return Err(format!(
                    "Expected 'case' or 'default', got {:?}",
                    self.peek().token
                ));
            }
        }
        self.consume(Token::RightBrace, "Expected '}'")?;
        Ok(SwitchStmt {
            expression,
            cases,
            default_block,
        })
    }

    fn parse_return(&mut self) -> Result<Statement, String> {
        self.consume(Token::Return, "Expected 'return'")?;
        let value = if !self.check_semicolon_or_brace() {
            Some(self.parse_expression()?)
        } else {
            None
        };
        Ok(Statement::Return(value))
    }

    fn parse_expression(&mut self) -> Result<Expr, String> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_and()?;
        while self.check(Token::PipePipe) {
            self.advance();
            let right = self.parse_and()?;
            expr = Expr::BinaryOp(Box::new(expr), Operator::LogicalOr, Box::new(right));
        }
        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_bitwise_or()?;
        while self.check(Token::AndAnd) {
            self.advance();
            let right = self.parse_bitwise_or()?;
            expr = Expr::BinaryOp(Box::new(expr), Operator::LogicalAnd, Box::new(right));
        }
        Ok(expr)
    }

    fn parse_bitwise_or(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_bitwise_xor()?;
        while self.check(Token::Pipe) {
            self.advance();
            let right = self.parse_bitwise_xor()?;
            expr = Expr::BinaryOp(Box::new(expr), Operator::BitwiseOr, Box::new(right));
        }
        Ok(expr)
    }

    fn parse_bitwise_xor(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_bitwise_and()?;
        while self.check(Token::Caret) {
            self.advance();
            let right = self.parse_bitwise_and()?;
            expr = Expr::BinaryOp(Box::new(expr), Operator::BitwiseXor, Box::new(right));
        }
        Ok(expr)
    }

    fn parse_bitwise_and(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_comparison()?;
        while self.check(Token::Ampersand) {
            self.advance();
            let right = self.parse_comparison()?;
            expr = Expr::BinaryOp(Box::new(expr), Operator::BitwiseAnd, Box::new(right));
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_addition()?;

        while self.check(Token::Greater)
            || self.check(Token::Less)
            || self.check(Token::GreaterEqual)
            || self.check(Token::LessEqual)
            || self.check(Token::EqualEqual)
            || self.check(Token::NotEqual)
        {
            let operator = if self.match_token(Token::Greater) {
                Operator::Greater
            } else if self.match_token(Token::Less) {
                Operator::Less
            } else if self.match_token(Token::GreaterEqual) {
                Operator::GreaterEqual
            } else if self.match_token(Token::LessEqual) {
                Operator::LessEqual
            } else if self.match_token(Token::EqualEqual) {
                Operator::Equal
            } else {
                Operator::NotEqual
            };
            let right = self.parse_addition()?;
            expr = Expr::BinaryOp(Box::new(expr), operator, Box::new(right));
        }
        Ok(expr)
    }

    fn parse_addition(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_multiplication()?;
        while self.check(Token::Plus) || self.check(Token::Minus) {
            let operator = if self.match_token(Token::Plus) {
                Operator::Add
            } else if self.match_token(Token::Minus) {
                Operator::Subtract
            } else {
                return Err("Expected + or -".to_string());
            };
            let right = self.parse_multiplication()?;
            expr = Expr::BinaryOp(Box::new(expr), operator, Box::new(right));
        }
        Ok(expr)
    }

    fn parse_multiplication(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_unary()?;
        while self.check(Token::Star) || self.check(Token::Slash) || self.check(Token::Percent) {
            let operator = if self.match_token(Token::Star) {
                Operator::Multiply
            } else if self.match_token(Token::Slash) {
                Operator::Divide
            } else if self.match_token(Token::Percent) {
                Operator::Modulo
            } else {
                return Err("Expected *, / or %".to_string());
            };
            let right = self.parse_unary()?;
            expr = Expr::BinaryOp(Box::new(expr), operator, Box::new(right));
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        if self.check(Token::Minus) {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::UnaryOp(UnaryOperator::Negate, Box::new(expr)));
        }
        if self.check(Token::Plus) {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::UnaryOp(UnaryOperator::Positive, Box::new(expr)));
        }
        if self.check(Token::Bang) {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::UnaryOp(UnaryOperator::Not, Box::new(expr)));
        }
        if self.check(Token::Tilde) {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::UnaryOp(UnaryOperator::BitwiseNot, Box::new(expr)));
        }
        self.parse_call()
    }

    fn parse_call(&mut self) -> Result<Expr, String> {
        let expr = self.parse_primary()?;
        if let Expr::Identifier(name) = &expr {
            if self.check(Token::LeftParen) {
                self.advance();
                let args = self.parse_arguments()?;
                self.consume(Token::RightParen, "Expected ')'")?;
                return Ok(Expr::Call(name.clone(), args));
            }
            if self.check(Token::Dot) {
                self.advance();
                if let Token::Identifier(method_name) = &self.peek().token {
                    let method_name = method_name.clone();
                    self.advance();
                    if self.check(Token::LeftParen) {
                        self.advance();
                        let args = self.parse_arguments()?;
                        self.consume(Token::RightParen, "Expected ')'")?;
                        return Ok(Expr::ModuleCall(name.clone(), method_name, args));
                    }
                    if self.check(Token::LeftBrace) {
                        let fields = self.parse_struct_literal_fields()?;
                        return Ok(Expr::StructLiteral(name.clone(), fields));
                    }
                    let field_name = method_name;
                    return Ok(Expr::FieldAccess(
                        Box::new(Expr::Identifier(name.clone())),
                        field_name,
                    ));
                }
            }
        }
        if self.check(Token::LeftBracket) {
            self.advance();
            let index = self.parse_expression()?;
            self.consume(Token::RightBracket, "Expected ']'")?;
            return Ok(Expr::ArrayAccess(Box::new(expr), Box::new(index)));
        }
        Ok(expr)
    }

    fn parse_arguments(&mut self) -> Result<Vec<Expr>, String> {
        let mut args = Vec::new();
        if !self.check(Token::RightParen) {
            loop {
                if self.check(Token::Ampersand) {
                    self.advance();
                }
                args.push(self.parse_expression()?);
                if !self.match_token(Token::Comma) {
                    break;
                }
            }
        }
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        let token = self.advance();
        match &token.token {
            Token::Identifier(name) => Ok(Expr::Identifier(name.clone())),
            Token::IntLiteral(value) => Ok(Expr::Literal(Literal::Int(*value))),
            Token::FloatLiteral(value) => Ok(Expr::Literal(Literal::Float(*value))),
            Token::StringLiteral(value) => Ok(Expr::Literal(Literal::String(value.clone()))),
            Token::True => Ok(Expr::Literal(Literal::Bool(true))),
            Token::False => Ok(Expr::Literal(Literal::Bool(false))),
            Token::LeftParen => {
                let expr = self.parse_expression()?;
                self.consume(Token::RightParen, "Expected ')'")?;
                Ok(expr)
            }
            _ => Err(format!("Unexpected token: {:?}", token.token)),
        }
    }

    fn parse_struct_def(&mut self) -> Result<StructDef, String> {
        self.consume(Token::Struct, "Expected 'struct'")?;
        let name = self.consume_identifier()?;
        self.consume(Token::LeftBrace, "Expected '{'")?;
        let mut fields = Vec::new();
        while !self.check(Token::RightBrace) && !self.is_at_end() {
            let field_name = self.consume_identifier()?;
            let field_type = self.parse_type()?;
            fields.push(StructField {
                name: field_name,
                field_type,
            });
            if !self.match_token(Token::Comma) {
                break;
            }
        }
        self.consume(Token::RightBrace, "Expected '}'")?;
        Ok(StructDef { name, fields })
    }

    fn parse_struct_literal_fields(&mut self) -> Result<Vec<(String, Expr)>, String> {
        self.consume(Token::LeftBrace, "Expected '{'")?;
        let mut fields = Vec::new();
        while !self.check(Token::RightBrace) && !self.is_at_end() {
            let name = self.consume_identifier()?;
            self.consume(Token::Colon, "Expected ':'")?;
            let value = self.parse_expression()?;
            fields.push((name, value));
            if !self.match_token(Token::Comma) {
                break;
            }
        }
        self.consume(Token::RightBrace, "Expected '}'")?;
        Ok(fields)
    }

    fn advance(&mut self) -> &TokenWithSpan {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn check(&self, token: Token) -> bool {
        if self.is_at_end() {
            return false;
        }
        self.peek().token == token
    }

    fn match_token(&mut self, token: Token) -> bool {
        if self.check(token) {
            self.advance();
            return true;
        }
        false
    }

    fn consume(&mut self, token: Token, message: &str) -> Result<&TokenWithSpan, String> {
        if self.check(token) {
            return Ok(self.advance());
        }
        Err(format!("{} at line {}", message, self.peek().line))
    }

    fn consume_identifier(&mut self) -> Result<String, String> {
        if let Token::Identifier(name) = &self.peek().token {
            let name = name.clone();
            self.advance();
            Ok(name)
        } else {
            Err(format!("Expected identifier at line {}", self.peek().line))
        }
    }

    fn peek(&self) -> &TokenWithSpan {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &TokenWithSpan {
        &self.tokens[self.current - 1]
    }

    fn is_at_end(&self) -> bool {
        self.peek().token == Token::EOF
    }

    fn check_semicolon_or_brace(&self) -> bool {
        self.check(Token::RightBrace) || self.is_at_end()
    }
}
