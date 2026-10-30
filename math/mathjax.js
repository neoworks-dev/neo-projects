// MathJax for the app's math worker (`crates/app/src/math.rs`), which runs this in a QuickJS of
// its own, apart from the UI. `bun build.mjs` bundles it to `dist/mathjax.js`.
import { mathjax } from "mathjax-full/js/mathjax.js";
import { TeX } from "mathjax-full/js/input/tex.js";
import { SVG } from "mathjax-full/js/output/svg.js";
import { liteAdaptor } from "mathjax-full/js/adaptors/liteAdaptor.js";
import { RegisterHTMLHandler } from "mathjax-full/js/handlers/html.js";
import { AllPackages } from "mathjax-full/js/input/tex/AllPackages.js";

const adaptor = liteAdaptor();
RegisterHTMLHandler(adaptor);

const document = mathjax.document("", {
  InputJax: new TeX({
    packages: AllPackages.filter((name) => name !== "bussproofs"),
    // A formula that doesn't parse throws, so the worker can tell it apart from one that does.
    formatError: (_jax, error) => {
      throw error;
    },
  }),
  // Every glyph as its own path, so the SVG stands alone.
  OutputJax: new SVG({ fontCache: "none" }),
});

/** The SVG of a TeX formula, display style or inline. Throws with TeX's message when it doesn't parse. */
globalThis.texToSvg = (tex, display) => {
  const node = document.convert(tex, { display });
  return adaptor.innerHTML(node);
};
