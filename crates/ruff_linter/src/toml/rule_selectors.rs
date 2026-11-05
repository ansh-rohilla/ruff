use toml::Spanned;
use toml::de::{DeArray, DeTable, DeValue};

use ruff_python_ast::TomlSourceType;
use ruff_text_size::{TextRange, TextSize};

use crate::checkers::ast::LintContext;
use crate::codes::Rule;
use crate::preview::is_human_readable_names_enabled;
use crate::rules::ruff::rules::{redirected_rule_selector, rule_codes_in_selectors};

pub(super) fn check_rule_selectors(
    context: &LintContext,
    document: &DeTable<'_>,
    source_type: TomlSourceType,
) {
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
            check_selector_array(context, values, selector, in_lint_table);
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
                check_selector_array(context, values, selector, in_lint_table);
            }
        }
    }
}

fn check_selector_array(
    context: &LintContext,
    values: &DeArray<'_>,
    setting: &'static str,
    in_lint_table: bool,
) {
    let source = context.source_file().source_text();

    for value in values {
        let Some((selector, range)) = selector_string(value, source) else {
            continue;
        };
        if context.is_rule_enabled(Rule::RedirectedRuleSelector) {
            redirected_rule_selector(context, selector, range, setting, in_lint_table);
        }
        if context.is_rule_enabled(Rule::RuleCodesInSelectors)
            && is_human_readable_names_enabled(context.settings().preview)
        {
            rule_codes_in_selectors(context, selector, range, setting, in_lint_table);
        }
    }
}

/// Use the decoded selector for lookup, but replace the source text between the TOML quotes.
fn selector_string<'a>(
    value: &'a Spanned<DeValue<'_>>,
    source: &str,
) -> Option<(&'a str, TextRange)> {
    let selector = value.get_ref().as_str()?;
    let span = value.span();
    let raw = &source[span.clone()];
    let quotes = if raw.starts_with("\"\"\"") || raw.starts_with("'''") {
        3
    } else {
        1
    };
    let start = TextSize::try_from(span.start + quotes).ok()?;
    let end = TextSize::try_from(span.end - quotes).ok()?;
    Some((selector, TextRange::new(start, end)))
}
