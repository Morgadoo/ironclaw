//! Gate 1: Unsafe block detection via AST parsing.
//!
//! Uses `syn` to parse the source and walk the AST, rejecting any
//! `unsafe` block, `unsafe fn`, or `unsafe impl`.

use syn::visit::Visit;

use project_e_core::types::GateName;

use super::{GateError, GateResult};

/// Check source code for any `unsafe` blocks using AST analysis.
pub fn check_no_unsafe(source: &str) -> GateResult {
    let syntax = match syn::parse_file(source) {
        Ok(file) => file,
        Err(e) => {
            return GateResult::Fail(
                GateError::new(GateName::UnsafeCheck, "Failed to parse source")
                    .with_details(e.to_string()),
            );
        }
    };

    let mut visitor = UnsafeVisitor { found: vec![] };
    visitor.visit_file(&syntax);

    if visitor.found.is_empty() {
        GateResult::Pass
    } else {
        GateResult::Fail(
            GateError::new(
                GateName::UnsafeCheck,
                format!("Found {} unsafe block(s)", visitor.found.len()),
            )
            .with_details(visitor.found.join("; ")),
        )
    }
}

struct UnsafeVisitor {
    found: Vec<String>,
}

impl<'ast> Visit<'ast> for UnsafeVisitor {
    fn visit_expr_unsafe(&mut self, node: &'ast syn::ExprUnsafe) {
        self.found.push("unsafe block detected".to_string());
        syn::visit::visit_expr_unsafe(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        if node.sig.unsafety.is_some() {
            self.found.push(format!("unsafe fn '{}'", node.sig.ident));
        }
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if node.unsafety.is_some() {
            self.found.push("unsafe impl block".to_string());
        }
        syn::visit::visit_item_impl(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_code_passes() {
        let code = r#"
            fn hello() -> String {
                "hello".to_string()
            }
        "#;
        assert!(check_no_unsafe(code).is_pass());
    }

    #[test]
    fn unsafe_block_fails() {
        let code = r#"
            fn risky() {
                unsafe {
                    std::ptr::null::<u8>().read();
                }
            }
        "#;
        assert!(!check_no_unsafe(code).is_pass());
    }

    #[test]
    fn unsafe_fn_fails() {
        let code = r#"
            unsafe fn dangerous() {}
        "#;
        assert!(!check_no_unsafe(code).is_pass());
    }

    #[test]
    fn invalid_source_fails() {
        let code = "this is not valid rust {{{";
        assert!(!check_no_unsafe(code).is_pass());
    }
}
