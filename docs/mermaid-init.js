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
    /* Always useMaxWidth so diagrams fit the content column on phones
     * and desktop. CSS (.mermaid svg { max-width:100% }) reinforces the
     * clamp; overflow-x:auto on .mermaid is fallback only. Slightly larger
     * theme font keeps labels readable after scale-down. */
    window.mermaid.initialize({
      startOnLoad: false,
      theme: isDark() ? "dark" : "neutral",
      securityLevel: "strict",
      themeVariables: {
        fontSize: "16px",
      },
      flowchart: { htmlLabels: false, useMaxWidth: true },
      sequence: { useMaxWidth: true },
      gantt: { useMaxWidth: true },
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
