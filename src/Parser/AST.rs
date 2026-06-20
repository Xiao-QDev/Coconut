// 程序 - 顶层结构
#[derive(Debug, Clone)]
pub struct Program {
    pub package: String,
    pub imports: Vec<Import>,
    pub global_vars: Vec<VarDecl>,
    pub functions: Vec<Function>,
    pub structs: Vec<StructDef>,
}
// 导入声明
#[derive(Debug, Clone)]
pub struct Import {
    pub path: String,
    pub alias: Option<String>,
}
// 结构体定义
#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<StructField>,
}
#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub field_type: Type,
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
    While(WhileStmt),
    Switch(SwitchStmt),
    MultiVarDecl(MultiVarDecl),
    MultiShortDecl(MultiShortDecl),
    IncDec(Expr, bool),
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
    pub target: Expr,
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
#[derive(Debug, Clone)]
pub struct ForStmt {
    pub init: Option<Box<Statement>>,
    pub condition: Option<Expr>,
    pub step: Option<Box<Statement>>,
    pub body: Block,
}
#[derive(Debug, Clone)]
pub struct WhileStmt {
    pub condition: Expr,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct SwitchStmt {
    pub expression: Expr,
    pub cases: Vec<SwitchCase>,
    pub default_block: Option<Block>,
}

#[derive(Debug, Clone)]
pub struct SwitchCase {
    pub condition: Option<Expr>,
    pub block: Block,
}

#[derive(Debug, Clone)]
pub struct MultiVarDecl {
    pub names: Vec<String>,
    pub var_type: Type,
    pub values: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub struct MultiShortDecl {
    pub names: Vec<String>,
    pub values: Vec<Expr>,
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
    ArrayAccess(Box<Expr>, Box<Expr>),
    FieldAccess(Box<Expr>, String),
    StructLiteral(String, Vec<(String, Expr)>),
}
// 一元运算符
#[derive(Debug, Clone)]
pub enum UnaryOperator {
    Negate,
    Not,
    Positive,
    BitwiseNot,
}
// 字面量
#[derive(Debug, Clone)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
}
// 运算符
#[derive(Debug, Clone)]
pub enum Operator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Greater,
    Less,
    GreaterEqual,
    LessEqual,
    Equal,
    NotEqual,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LogicalAnd,
    LogicalOr,
}
// 类型
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    String,
    Bool,
    Void,
    Array(Box<Type>, usize),
    Struct(String),
}
