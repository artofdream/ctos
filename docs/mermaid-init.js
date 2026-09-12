/* Load a pinned mermaid.js and render ```mermaid fences.
 * Works with mdbook-mermaid preprocessor output (`pre.mermaid`)
 * and with raw highlight.js blocks (`code.language-mermaid`).
 * A missing network fetch leaves the source text visible — that is fine.
 */
(function () {
  var MERMAID_SRC =
    "https://cdn.jsdelivr.net/npm/mermaid@11.6.0/dist/mermaid.min.js";

  function isDark() {
    var cls = document.documentElement.className || "";
    return /(navy|coal|ayu)/.test(cls);
  }

  function unwrapHighlightBlocks() {
    document.querySelectorAll("code.language-mermaid").forEach(function (code) {
      var pre = code.parentElement;
      if (!pre || pre.tagName !== "PRE") {
        return;
      }
      var wrap = document.createElement("pre");
      wrap.className = "mermaid";
      wrap.textContent = code.textContent;
      pre.replaceWith(wrap);
    });
  }

  function boot() {
    if (!window.mermaid) {
      return;
    }
    unwrapHighlightBlocks();
    /* On phones, keep the natural diagram width and let .mermaid scroll
     * (see mermaid.css). useMaxWidth:true squashes labels on 320px. */
    var narrow = window.matchMedia("(max-width: 768px)").matches;
    window.mermaid.initialize({
      startOnLoad: false,
      theme: isDark() ? "dark" : "neutral",
      securityLevel: "strict",
      flowchart: { htmlLabels: false, useMaxWidth: !narrow },
    });
    window.mermaid.run({ querySelector: ".mermaid" }).catch(function () {
      /* Keep the source text if a diagram fails to parse. */
    });
  }

  var script = document.createElement("script");
  script.src = MERMAID_SRC;
  script.onload = boot;
  document.head.appendChild(script);
})();
