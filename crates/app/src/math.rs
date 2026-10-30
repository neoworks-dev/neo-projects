//! LaTeX math as SVG. MathJax typesets it in a QuickJS of its own on a thread of its own, so the
//! UI never waits for it. MathJax is `dist/mathjax.js`, which `build.mjs` bundles beside the UI;
//! it is loaded when the first formula asks for it, and each formula is typeset once.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{Sender, channel};

use rquickjs::{Context, Function, Runtime};
use serde::Serialize;

/// A typeset formula. Sizes are in ems of the text around it, which MathJax's SVG gives in
/// thousandths in its `viewBox`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Formula {
    pub svg: String,
    pub width: f32,
    pub height: f32,
    /// How far it reaches below the baseline.
    pub depth: f32,
}

type Answer = Result<Formula, String>;

struct Request {
    tex: String,
    display: bool,
    reply: Sender<Answer>,
}

pub struct Math {
    requests: Sender<Request>,
}

impl Math {
    /// Starts the worker, which loads `script` (MathJax) on its first formula.
    pub fn start(script: PathBuf) -> Math {
        let (requests, incoming) = channel::<Request>();
        // MathJax recurses deeply through its parser, more than a thread's default stack holds.
        std::thread::Builder::new()
            .name("math".into())
            .stack_size(32 << 20)
            .spawn(move || {
                let mut typesetter: Option<Result<Typesetter, String>> = None;
                let mut done: HashMap<(String, bool), Answer> = HashMap::new();
                for request in incoming {
                    let key = (request.tex, request.display);
                    if let Some(answer) = done.get(&key) {
                        let _ = request.reply.send(answer.clone());
                        continue;
                    }
                    let typesetter = typesetter.get_or_insert_with(|| Typesetter::load(&script));
                    let answer = match typesetter {
                        Ok(typesetter) => typesetter.typeset(&key.0, key.1),
                        Err(e) => Err(e.clone()),
                    };
                    let _ = request.reply.send(answer.clone());
                    done.insert(key, answer);
                }
            })
            .expect("start the math thread");
        Math { requests }
    }

    /// The formula typeset, or TeX's message when it doesn't parse. Blocks until it is done.
    pub fn typeset(&self, tex: &str, display: bool) -> Answer {
        let (reply, answer) = channel();
        let request = Request { tex: tex.to_string(), display, reply };
        if self.requests.send(request).is_err() {
            return Err("The math worker stopped".into());
        }
        answer.recv().unwrap_or_else(|_| Err("The math worker stopped".into()))
    }
}

struct Typesetter {
    // The context holds the runtime alive, but the runtime is kept to make the order plain.
    _runtime: Runtime,
    context: Context,
}

impl Typesetter {
    fn load(script: &PathBuf) -> Result<Typesetter, String> {
        let source = std::fs::read(script).map_err(|e| format!("can't read {}: {e}", script.display()))?;
        let runtime = Runtime::new().map_err(|e| e.to_string())?;
        runtime.set_max_stack_size(24 << 20);
        let context = Context::full(&runtime).map_err(|e| e.to_string())?;
        context.with(|ctx| {
            ctx.eval::<(), _>(source).map_err(|_| format!("MathJax didn't load: {}", thrown(&ctx)))
        })?;
        Ok(Typesetter { _runtime: runtime, context })
    }

    fn typeset(&self, tex: &str, display: bool) -> Answer {
        let svg = self.context.with(|ctx| {
            let to_svg: Function = ctx.globals().get("texToSvg").map_err(|e| e.to_string())?;
            to_svg.call::<_, String>((tex, display)).map_err(|_| thrown(&ctx))
        })?;
        let Some((width, height, depth)) = measure(&svg) else {
            return Err("MathJax gave an SVG without a viewBox".into());
        };
        Ok(Formula { svg, width, height, depth })
    }
}

/// The message of what the script just threw. TeX's errors aren't `Error`s, but have a message.
fn thrown(ctx: &rquickjs::Ctx<'_>) -> String {
    let value = ctx.catch();
    let message = value.as_object().and_then(|object| object.get::<_, String>("message").ok());
    message.unwrap_or_else(|| "MathJax failed".into())
}

/// Width, height and depth in ems, from the `viewBox` of MathJax's SVG: its y runs from
/// `min_y` (the top, above the baseline at 0) down to `min_y + height`, in thousandths of an em.
pub fn measure(svg: &str) -> Option<(f32, f32, f32)> {
    let start = svg.find("viewBox=\"")? + "viewBox=\"".len();
    let end = start + svg[start..].find('"')?;
    let numbers: Vec<f32> = svg[start..end].split_whitespace().filter_map(|n| n.parse().ok()).collect();
    let [_, min_y, width, height] = numbers[..] else {
        return None;
    };
    Some((width / 1000.0, height / 1000.0, (min_y + height).max(0.0) / 1000.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_come_from_the_view_box_in_ems() {
        let svg = r#"<svg style="vertical-align: -0.566ex" viewBox="0 -750 4046.6 1000" width="9ex">"#;
        assert_eq!(measure(svg), Some((4.0466, 1.0, 0.25)));
        assert_eq!(measure("<svg>"), None);
    }

    /// Runs MathJax itself, which `bun build.mjs` bundles first.
    #[test]
    fn typesets_a_formula_and_reports_one_that_does_not_parse() {
        let script = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../dist/mathjax.js"));
        if !script.exists() {
            eprintln!("skipped: run `bun build.mjs` to bundle MathJax");
            return;
        }
        let math = Math::start(script);
        let formula = math.typeset(r"\frac{a}{b} + x^2", false).unwrap();
        assert!(formula.svg.starts_with("<svg"));
        assert!(formula.height > formula.depth && formula.depth > 0.0);
        let error = math.typeset(r"\frac{a}{b", false).unwrap_err();
        assert!(error.contains("Missing close brace"), "{error}");
        assert_eq!(math.typeset(r"\frac{a}{b} + x^2", false).unwrap(), formula);
    }
}
