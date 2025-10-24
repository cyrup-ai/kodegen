//! Template abstract syntax tree

use std::sync::Arc;

/// Abstract syntax tree for templates
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateAst {
    /// Static text content
    Text(String),
    /// Variable reference
    Variable(String),
    /// Expression with operator and operands
    Expression {
        /// The operator for this expression
        operator: String,
        /// The operands for this expression
        operands: Arc<[TemplateAst]>,
    },
    /// Conditional (if/else) statement
    Conditional {
        /// The condition to evaluate
        condition: Arc<TemplateAst>,
        /// AST to execute if condition is true
        if_true: Arc<TemplateAst>,
        /// Optional AST to execute if condition is false
        if_false: Option<Arc<TemplateAst>>,
    },
    /// Loop statement
    Loop {
        /// Loop variable name
        variable: String,
        /// The iterable expression to loop over
        iterable: Arc<TemplateAst>,
        /// The body of the loop
        body: Arc<TemplateAst>,
    },
    /// Block of multiple AST nodes
    Block(Arc<[TemplateAst]>),
    /// Function call
    Function {
        /// Function name
        name: String,
        /// Function arguments
        args: Arc<[TemplateAst]>,
    },
}
