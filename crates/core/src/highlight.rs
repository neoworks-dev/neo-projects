//! Code highlighting for code blocks in Markdown. The code is split into runs, each with a kind
//! (keyword, string, comment…) rather than a colour, so the window colours them from its own
//! theme. Grammars are Sublime Text's, as `syntect` bundles them.

use std::sync::OnceLock;

use serde::Serialize;
use syntect::parsing::{ParseState, ScopeStack, ScopeStackOp, SyntaxSet};

/// A stretch of code and what it is.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Run {
    pub text: String,
    /// `keyword`, `string`, `comment`, `number`, `function`, `type`, `constant`, `variable`,
    /// `operator`, `punctuation`, `tag`, `attribute` or `plain`.
    pub kind: &'static str,
}

fn syntaxes() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

/// What a scope stack's innermost meaningful scope makes a run.
fn kind(stack: &ScopeStack) -> &'static str {
    const KINDS: [(&str, &str); 16] = [
        ("comment", "comment"),
        ("string", "string"),
        ("constant.numeric", "number"),
        ("constant.character.escape", "string"),
        ("constant", "constant"),
        ("keyword.operator", "operator"),
        ("keyword", "keyword"),
        ("storage.type", "keyword"),
        ("storage", "keyword"),
        ("entity.name.function", "function"),
        ("support.function", "function"),
        ("entity.name.tag", "tag"),
        ("entity.other.attribute-name", "attribute"),
        ("entity.name", "type"),
        ("support.type", "type"),
        ("punctuation", "punctuation"),
    ];
    for scope in stack.as_slice().iter().rev() {
        let name = scope.build_string();
        if let Some((_, kind)) = KINDS.iter().find(|(prefix, _)| name.starts_with(prefix)) {
            return kind;
        }
        if name.starts_with("variable") {
            return "variable";
        }
    }
    "plain"
}

/// `code` as runs, by the grammar `lang` names (a name or a file extension: `rust`, `rs`,
/// `ts`). Unknown languages, and code a grammar can't parse, come back as one plain run.
pub fn highlight(code: &str, lang: &str) -> Vec<Run> {
    let set = syntaxes();
    let lang = match lang.to_lowercase().as_str() {
        "ts" | "typescript" | "tsx" | "svelte" => "js".to_string(),
        "sh" | "shell" | "bash" | "zsh" | "fish" => "sh".to_string(),
        other => other.to_string(),
    };
    let syntax = set.find_syntax_by_token(&lang).or_else(|| set.find_syntax_by_extension(&lang));
    let plain = || vec![Run { text: code.to_string(), kind: "plain" }];
    let Some(syntax) = syntax else { return plain() };
    let mut state = ParseState::new(syntax);
    let mut stack = ScopeStack::new();
    let mut runs: Vec<Run> = Vec::new();
    for line in code.split_inclusive('\n') {
        let Ok(ops) = state.parse_line(line, set) else { return plain() };
        let mut at = 0;
        for (index, op) in ops {
            push(&mut runs, &line[at..index], kind(&stack));
            at = index;
            apply(&mut stack, &op);
        }
        push(&mut runs, &line[at..], kind(&stack));
    }
    runs
}

fn apply(stack: &mut ScopeStack, op: &ScopeStackOp) {
    // A broken grammar leaves the stack as it was rather than failing the block.
    let _ = stack.apply(op);
}

/// Adds text as a run, joining it to the last one when they are the same kind.
fn push(runs: &mut Vec<Run>, text: &str, kind: &'static str) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = runs.last_mut()
        && last.kind == kind
    {
        last.text.push_str(text);
        return;
    }
    runs.push(Run { text: text.to_string(), kind });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds<'a>(runs: &'a [Run], text: &str) -> Vec<&'a str> {
        runs.iter().filter(|r| r.text.trim() == text).map(|r| r.kind).collect()
    }

    #[test]
    fn rust_keywords_strings_and_comments() {
        let code = "// hi\nfn main() {\n    let s = \"x\";\n}\n";
        let runs = highlight(code, "rust");
        assert_eq!(runs.iter().map(|r| r.text.as_str()).collect::<String>(), code, "nothing lost");
        assert_eq!(kinds(&runs, "fn"), ["keyword"]);
        assert_eq!(kinds(&runs, "main"), ["function"]);
        assert!(runs.iter().any(|r| r.kind == "comment" && r.text.contains("hi")));
        assert!(runs.iter().any(|r| r.kind == "string" && r.text.contains('x')));
    }

    #[test]
    fn typescript_by_its_short_name() {
        let runs = highlight("const n = 42;", "ts");
        assert_eq!(kinds(&runs, "42"), ["number"]);
    }

    #[test]
    fn unknown_languages_are_plain() {
        assert_eq!(highlight("a b", "nope"), vec![Run { text: "a b".into(), kind: "plain" }]);
    }
}
