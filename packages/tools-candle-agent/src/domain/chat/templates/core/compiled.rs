//! Compiled template with rendering logic

use std::sync::Arc;

use super::ast::TemplateAst;
use super::context::TemplateContext;
use super::error::{TemplateError, TemplateResult};
use super::types::{TemplateMetadata, TemplateVariable};
use super::value::TemplateValue;

/// Compiled template for efficient rendering
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledTemplate {
    /// Template metadata and information
    pub metadata: TemplateMetadata,
    /// Compiled abstract syntax tree
    pub ast: TemplateAst,
    /// Template variables and their definitions
    pub variables: Arc<[TemplateVariable]>,
    /// Whether the template has been optimized for performance
    pub optimized: bool,
}

impl CompiledTemplate {
    /// Create a new compiled template
    #[must_use]
    pub fn new(
        metadata: TemplateMetadata,
        ast: TemplateAst,
        variables: Arc<[TemplateVariable]>,
    ) -> Self {
        Self {
            metadata,
            ast,
            variables,
            optimized: false,
        }
    }

    /// Render the compiled template with the given context
    ///
    /// # Errors
    ///
    /// Returns `TemplateError` if rendering fails
    pub fn render(&self, context: &TemplateContext) -> TemplateResult<String> {
        Self::render_ast(&self.ast, context)
    }

    fn render_ast(ast: &TemplateAst, context: &TemplateContext) -> TemplateResult<String> {
        match ast {
            TemplateAst::Text(text) => Ok(text.clone()),
            TemplateAst::Variable(name) => {
                // Get variable from context
                let value =
                    context
                        .get_variable(name)
                        .ok_or_else(|| TemplateError::VariableError {
                            message: format!("Variable '{name}' not found"),
                        })?;

                // Convert value to string
                match value {
                    TemplateValue::String(s) => Ok(s.clone()),
                    TemplateValue::Number(n) => Ok(n.to_string()),
                    TemplateValue::Boolean(b) => Ok(b.to_string()),
                    TemplateValue::Array(arr) => Ok(format!("[{} items]", arr.len())),
                    TemplateValue::Object(obj) => Ok(format!("{{{} keys}}", obj.len())),
                    TemplateValue::Null => Ok(String::new()),
                }
            }
            TemplateAst::Expression { operator, operands } => {
                Self::eval_expression(operator, operands, context)
            }
            TemplateAst::Conditional {
                condition,
                if_true,
                if_false,
            } => Self::render_conditional(condition, if_true, if_false.as_deref(), context),
            TemplateAst::Loop {
                variable,
                iterable,
                body,
            } => Self::render_loop(variable, iterable, body, context),
            TemplateAst::Block(nodes) => {
                let mut result = String::new();
                for node in nodes.iter() {
                    result.push_str(&Self::render_ast(node, context)?);
                }
                Ok(result)
            }
            TemplateAst::Function { name, args } => Self::call_function(name, args, context),
        }
    }

    fn render_conditional(
        condition: &TemplateAst,
        if_true: &TemplateAst,
        if_false: Option<&TemplateAst>,
        context: &TemplateContext,
    ) -> TemplateResult<String> {
        // Evaluate condition
        let condition_value = Self::render_ast(condition, context)?;

        // Check if condition is true (non-empty, not "false", not "0")
        let condition_met =
            !condition_value.is_empty() && condition_value != "false" && condition_value != "0";

        if condition_met {
            Self::render_ast(if_true, context)
        } else if let Some(else_branch) = if_false {
            Self::render_ast(else_branch, context)
        } else {
            Ok(String::new())
        }
    }

    fn eval_expression(
        operator: &str,
        operands: &[TemplateAst],
        context: &TemplateContext,
    ) -> TemplateResult<String> {
        if operands.len() != 2 {
            return Err(TemplateError::RenderError {
                message: format!("Binary operator '{operator}' requires exactly 2 operands"),
            });
        }

        let left = Self::render_ast(&operands[0], context)?;
        let right = Self::render_ast(&operands[1], context)?;

        match operator {
            "+" => Ok(Self::op_add(&left, &right)),
            "-" => Self::op_numeric(&left, &right, "subtraction", |l, r| l - r),
            "*" => Self::op_numeric(&left, &right, "multiplication", |l, r| l * r),
            "/" => Self::op_divide(&left, &right),
            "==" => Ok((left == right).to_string()),
            "!=" => Ok((left != right).to_string()),
            "<" => Ok(Self::op_compare(&left, &right, "<")),
            ">" => Ok(Self::op_compare(&left, &right, ">")),
            "<=" => Ok(Self::op_compare(&left, &right, "<=")),
            ">=" => Ok(Self::op_compare(&left, &right, ">=")),
            "&&" | "and" => Ok((Self::truthy(&left) && Self::truthy(&right)).to_string()),
            "||" | "or" => Ok((Self::truthy(&left) || Self::truthy(&right)).to_string()),
            _ => Err(TemplateError::RenderError {
                message: format!("Unknown operator: {operator}"),
            }),
        }
    }

    fn parse_number_for(op: &str, value: &str) -> Result<f64, TemplateError> {
        value
            .parse::<f64>()
            .map_err(|_| TemplateError::RenderError {
                message: format!("Cannot parse '{value}' as number for {op}"),
            })
    }

    fn op_add(left: &str, right: &str) -> String {
        if let (Ok(l), Ok(r)) = (left.parse::<f64>(), right.parse::<f64>()) {
            (l + r).to_string()
        } else {
            format!("{left}{right}")
        }
    }

    fn op_numeric(
        left: &str,
        right: &str,
        op_name: &str,
        f: impl Fn(f64, f64) -> f64,
    ) -> TemplateResult<String> {
        let l = Self::parse_number_for(op_name, left)?;
        let r = Self::parse_number_for(op_name, right)?;
        Ok(f(l, r).to_string())
    }

    fn op_divide(left: &str, right: &str) -> TemplateResult<String> {
        let l = Self::parse_number_for("division", left)?;
        let r = Self::parse_number_for("division", right)?;
        if r == 0.0 {
            return Err(TemplateError::RenderError {
                message: "Division by zero".to_string(),
            });
        }
        Ok((l / r).to_string())
    }

    fn try_parse_both(left: &str, right: &str) -> Option<(f64, f64)> {
        match (left.parse::<f64>(), right.parse::<f64>()) {
            (Ok(l), Ok(r)) => Some((l, r)),
            _ => None,
        }
    }

    fn op_compare(left: &str, right: &str, op: &str) -> String {
        if let Some((l, r)) = Self::try_parse_both(left, right) {
            let res = match op {
                "<" => l < r,
                ">" => l > r,
                "<=" => l <= r,
                ">=" => l >= r,
                _ => unreachable!(),
            };
            return res.to_string();
        }

        let res = match op {
            "<" => left < right,
            ">" => left > right,
            "<=" => left <= right,
            ">=" => left >= right,
            _ => unreachable!(),
        };
        res.to_string()
    }

    fn truthy(s: &str) -> bool {
        !s.is_empty() && s != "false" && s != "0"
    }

    fn render_loop(
        variable: &str,
        iterable: &TemplateAst,
        body: &TemplateAst,
        context: &TemplateContext,
    ) -> TemplateResult<String> {
        // Get the iterable value
        let iterable_value = Self::render_ast(iterable, context)?;

        // Get collection from context
        let collection =
            context
                .get_variable(&iterable_value)
                .ok_or_else(|| TemplateError::VariableError {
                    message: format!("Loop iterable '{iterable_value}' not found"),
                })?;

        let mut result = String::new();

        match collection {
            TemplateValue::Array(items) => {
                for item in items {
                    // Create new context with loop variable
                    let mut loop_context = context.clone();
                    loop_context.set_variable(variable, item.clone());

                    // Render body with loop context
                    let rendered = Self::render_ast(body, &loop_context)?;
                    result.push_str(&rendered);
                }
            }
            _ => {
                return Err(TemplateError::RenderError {
                    message: "Loop iterable must be an array".to_string(),
                });
            }
        }

        Ok(result)
    }

    fn call_function(
        name: &str,
        args: &[TemplateAst],
        context: &TemplateContext,
    ) -> TemplateResult<String> {
        // Get function from context
        let func = context
            .functions
            .get(name)
            .ok_or_else(|| TemplateError::RenderError {
                message: format!("Function '{name}' not found"),
            })?;

        // Evaluate arguments
        let mut arg_values = Vec::new();
        for arg in args {
            let rendered = Self::render_ast(arg, context)?;
            arg_values.push(TemplateValue::String(rendered));
        }

        // Call function
        let result = func(&arg_values)?;

        // Convert result to string
        match result {
            TemplateValue::String(s) => Ok(s),
            TemplateValue::Number(n) => Ok(n.to_string()),
            TemplateValue::Boolean(b) => Ok(b.to_string()),
            TemplateValue::Array(arr) => Ok(format!("[{} items]", arr.len())),
            TemplateValue::Object(obj) => Ok(format!("{{{} keys}}", obj.len())),
            TemplateValue::Null => Ok(String::new()),
        }
    }
}
