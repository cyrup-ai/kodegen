//! Template compiler for optimizing template execution
//!
//! Provides template compilation with optimization passes.

use std::sync::Arc;

use super::parser::TemplateParser;
use crate::domain::chat::templates::core::{
    ChatTemplate, CompiledTemplate, TemplateAst, TemplateResult,
};

/// Template compiler configuration
#[derive(Debug, Clone)]
pub struct CompilerConfig {
    /// Enable optimization passes
    pub optimize: bool,
    /// Maximum compilation time in milliseconds
    pub max_compile_time_ms: u64,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            optimize: true,
            max_compile_time_ms: 5000, // 5 seconds
        }
    }
}

/// Template compiler for optimizing templates
#[derive(Debug, Clone)]
pub struct TemplateCompiler {
    config: CompilerConfig,
}

impl TemplateCompiler {
    /// Create a new template compiler
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: CompilerConfig::default(),
        }
    }

    /// Create compiler with configuration
    #[must_use]
    pub fn with_config(config: CompilerConfig) -> Self {
        Self { config }
    }

    /// Compile a template
    ///
    /// # Errors
    ///
    /// Returns `TemplateError` if template compilation fails
    pub fn compile(&self, template: &ChatTemplate) -> TemplateResult<CompiledTemplate> {
        // Create parser instance
        let parser = TemplateParser::new();

        // Parse template content into AST
        let ast = parser.parse(template.get_content())?;

        // Extract variables from parsed template (if needed)
        let variables = if template.variables.is_empty() {
            let extracted = parser.extract_variables(template.get_content())?;
            Arc::from(extracted)
        } else {
            template.variables.clone()
        };

        // Create compiled template
        let mut compiled = CompiledTemplate::new(template.metadata.clone(), ast, variables);

        // Apply optimizations if enabled
        if self.config.optimize {
            compiled = Self::optimize(compiled)?;
        }

        compiled.optimized = self.config.optimize;

        Ok(compiled)
    }

    /// Optimize a compiled template
    ///
    /// # Errors
    ///
    /// Returns `TemplateError` if optimization fails
    fn optimize(compiled: CompiledTemplate) -> TemplateResult<CompiledTemplate> {
        // Optimization passes:
        // 1. Adjacent text merging: Text("a") + Text("b") → Text("ab")
        // 2. Constant folding could be added in future
        // 3. Dead code elimination could be added in future

        let optimized_ast = Self::optimize_ast(&compiled.ast)?;

        Ok(CompiledTemplate {
            ast: optimized_ast,
            ..compiled
        })
    }

    /// Optimize an AST node
    ///
    /// # Errors
    ///
    /// Returns `TemplateError` if optimization fails
    fn optimize_ast(ast: &TemplateAst) -> TemplateResult<TemplateAst> {
        match ast {
            TemplateAst::Block(nodes) => {
                // Merge adjacent Text nodes
                let mut optimized = Vec::new();
                let mut pending_text = String::new();

                for node in nodes.iter() {
                    if let TemplateAst::Text(t) = node {
                        pending_text.push_str(t);
                    } else {
                        if !pending_text.is_empty() {
                            optimized.push(TemplateAst::Text(pending_text.clone()));
                            pending_text.clear();
                        }
                        optimized.push(Self::optimize_ast(node)?);
                    }
                }

                if !pending_text.is_empty() {
                    optimized.push(TemplateAst::Text(pending_text));
                }

                // If only one node after optimization, unwrap the block
                match optimized.len() {
                    0 => Ok(TemplateAst::Text(String::new())),
                    1 => {
                        if let Some(node) = optimized.into_iter().next() {
                            Ok(node)
                        } else {
                            Ok(TemplateAst::Text(String::new()))
                        }
                    }
                    _ => Ok(TemplateAst::Block(optimized.into())),
                }
            }
            TemplateAst::Conditional {
                condition,
                if_true,
                if_false,
            } => {
                let opt_condition = Self::optimize_ast(condition)?;
                let opt_if_true = Self::optimize_ast(if_true)?;
                let opt_if_false = if let Some(if_false_ast) = if_false {
                    Some(Arc::new(Self::optimize_ast(if_false_ast)?))
                } else {
                    None
                };

                Ok(TemplateAst::Conditional {
                    condition: Arc::new(opt_condition),
                    if_true: Arc::new(opt_if_true),
                    if_false: opt_if_false,
                })
            }
            TemplateAst::Loop {
                variable,
                iterable,
                body,
            } => {
                let opt_iterable = Self::optimize_ast(iterable)?;
                let opt_body = Self::optimize_ast(body)?;

                Ok(TemplateAst::Loop {
                    variable: variable.clone(),
                    iterable: Arc::new(opt_iterable),
                    body: Arc::new(opt_body),
                })
            }
            TemplateAst::Expression { operator, operands } => {
                let mut opt_operands = Vec::new();
                for operand in operands.iter() {
                    opt_operands.push(Self::optimize_ast(operand)?);
                }

                Ok(TemplateAst::Expression {
                    operator: operator.clone(),
                    operands: opt_operands.into(),
                })
            }
            TemplateAst::Function { name, args } => {
                let mut opt_args = Vec::new();
                for arg in args.iter() {
                    opt_args.push(Self::optimize_ast(arg)?);
                }

                Ok(TemplateAst::Function {
                    name: name.clone(),
                    args: opt_args.into(),
                })
            }
            _ => Ok(ast.clone()),
        }
    }

    /// Compile directly from AST (primarily for testing)
    ///
    /// # Errors
    ///
    /// Returns `TemplateError` if compilation fails
    pub fn compile_from_ast(&self, ast: TemplateAst) -> TemplateResult<CompiledTemplate> {
        use crate::domain::chat::templates::core::{
            TemplateCategory, TemplateMetadata, TemplatePermissions,
        };

        // Create minimal metadata for testing
        let metadata = TemplateMetadata {
            id: String::from("test"),
            name: String::from("test"),
            description: String::from("Test template"),
            author: String::from("test"),
            version: String::from("1.0.0"),
            category: TemplateCategory::default(),
            tags: Vec::new(),
            created_at: 0,
            modified_at: 0,
            usage_count: 0,
            rating: 0.0,
            permissions: TemplatePermissions::default(),
        };

        // No variables for simple test cases
        let variables = Arc::from([]);

        // Create compiled template
        let mut compiled = CompiledTemplate::new(metadata, ast, variables);

        // Apply optimizations if enabled
        if self.config.optimize {
            compiled = Self::optimize(compiled)?;
        }

        compiled.optimized = self.config.optimize;

        Ok(compiled)
    }

    /// Get compiler configuration
    #[must_use]
    pub fn config(&self) -> &CompilerConfig {
        &self.config
    }
}

impl Default for TemplateCompiler {
    fn default() -> Self {
        Self::new()
    }
}
