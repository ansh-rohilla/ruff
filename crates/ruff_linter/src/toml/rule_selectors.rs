use toml::de::{DeTable, DeValue};

use ruff_python_ast::TomlSourceType;

use crate::checkers::ast::LintContext;
use crate::preview::is_human_readable_names_enabled;
use crate::rules::ruff::rules::rule_codes_in_selectors;

pub(super) fn check_rule_selectors(
    context: &LintContext,
    document: &DeTable<'_>,
    source_type: TomlSourceType,
) {
    if !is_human_readable_names_enabled(context.settings().preview) {
        return;
    }

    let ruff = match source_type {
        TomlSourceType::Pyproject => document
            .get("tool")
            .and_then(|tool| tool.get_ref().get("ruff"))
            .and_then(|ruff| ruff.get_ref().as_table()),
        TomlSourceType::Ruff => Some(document),
        _ => None,
    };

    let Some(ruff) = ruff else {
        return;
    };

    check_selectors(context, ruff, false);

    if let Some(lint) = ruff.get("lint").and_then(|lint| lint.get_ref().as_table()) {
        check_selectors(context, lint, true);
    }
}

/// Selectors that are themselves arrays.
///
/// For example:
///
/// ```toml
/// select = ["F401"]
/// ```
const ARRAY_SELECTORS: &[&str] = &[
    "select",
    "extend-select",
    "fixable",
    "extend-fixable",
    "ignore",
    "extend-ignore",
    "unfixable",
    "extend-unfixable",
    "extend-safe-fixes",
    "extend-unsafe-fixes",
];

/// Selectors that are tables containing arrays.
///
/// For example:
///
/// ```toml
/// per-file-ignores = { "*.py" = ["F401"] }
/// ```
const TABLE_SELECTORS: &[&str] = &["per-file-ignores", "extend-per-file-ignores"];

fn check_selectors(context: &LintContext, table: &DeTable<'_>, in_lint_table: bool) {
    for &selector in ARRAY_SELECTORS {
        let Some(value) = table.get(selector) else {
            continue;
        };

        if let DeValue::Array(values) = value.get_ref() {
            rule_codes_in_selectors(context, values, selector, in_lint_table);
        }
    }

    for &selector in TABLE_SELECTORS {
        let Some(value) = table.get(selector) else {
            continue;
        };

        if let DeValue::Table(per_file) = value.get_ref() {
            for value in per_file.values() {
                let Some(values) = value.get_ref().as_array() else {
                    continue;
                };
                rule_codes_in_selectors(context, values, selector, in_lint_table);
            }
        }
    }
}
