// 程序 - 顶层结构
#[derive(Debug, Clone)]
pub struct Program {
    pub package: String,
    pub imports: Vec<Import>,
    pub global_vars: Vec<VarDecl>,
    pub functions: Vec<Function>,
}
// 导入声明
#[derive(Debug, Clone)]
pub struct Import {
    pub path: String,
    pub alias: Option<String>,
}
// 函数
#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub body: Block,
}
// 参数
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub param_type: Type,
}
// 语句块
#[derive(Debug, Clone)]
pub struct Block {
    pub statements: Vec<Statement>,
}
// 语句
#[derive(Debug, Clone)]
pub enum Statement {
    Return(Option<Expr>),
    Expression(Expr),
    VarDecl(VarDecl),
    ShortDecl(ShortDecl),
    If(IfStmt),
    For(ForStmt),
    Break,
    Continue,
    Assign(AssignStmt),
}
// 变量声明 如: var a int = 10
#[derive(Debug, Clone)]
pub struct VarDecl {
    pub name: String,
    pub var_type: Type,
    pub value: Expr,
}
// 简短声明 如: a := 10
#[derive(Debug, Clone)]
pub struct ShortDecl {
    pub name: String,
    pub value: Expr,
}
// 赋值语句 如: a = 10
#[derive(Debug, Clone)]
pub struct AssignStmt {
    pub name: String,
    pub value: Expr,
}
// If 语句
#[derive(Debug, Clone)]
pub struct IfStmt {
    pub condition: Expr,
    pub then_block: Block,
    pub else_block: Option<Block>,
}
// For 循环语句
// 三段式: for init; cond; step { body }
// 条件式: for cond { body }
// 无限式: for { body }
#[derive(Debug, Clone)]
pub struct ForStmt {
    pub init: Option<Box<Statement>>,
    pub condition: Option<Expr>,
    pub step: Option<Box<Statement>>,
    pub body: Block,
}
// 表达式
#[derive(Debug, Clone)]
pub enum Expr {
    Identifier(String),
    Literal(Literal),
    Call(String, Vec<Expr>),
    ModuleCall(String, String, Vec<Expr>),
    BinaryOp(Box<Expr>, Operator, Box<Expr>),
    UnaryOp(UnaryOperator, Box<Expr>),
}
// 一元运算符
#[derive(Debug, Clone)]
pub enum UnaryOperator {
    Negate,
    Not,
    Positive,
}
// 字面量
#[derive(Debug, Clone)]
pub enum Literal {
    Int(i64),
    String(String),
}
// 运算符
#[derive(Debug, Clone)]
pub enum Operator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Greater,
    Less,
    GreaterEqual,
    LessEqual,
    Equal,
    NotEqual,
}
// 类型
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    String,
    Void,
}