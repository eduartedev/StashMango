<script>
  import { EditorView, basicSetup } from "codemirror";
  import { EditorState } from "@codemirror/state";
  import { javascript } from "@codemirror/lang-javascript";
  import { rust } from "@codemirror/lang-rust";
  import { json } from "@codemirror/lang-json";
  import { markdown } from "@codemirror/lang-markdown";
  import { html } from "@codemirror/lang-html";
  import { css } from "@codemirror/lang-css";

  let { path = "", ours = "", theirs = "", result = $bindable(""), onsave } = $props();

  let oursEl;
  let resultEl;
  let theirsEl;
  let views = [];

  function language() {
    if (path.endsWith(".rs")) return rust();
    if (path.endsWith(".json")) return json();
    if (path.endsWith(".md")) return markdown();
    if (path.endsWith(".html")) return html();
    if (path.endsWith(".css")) return css();
    if (/\.(js|ts|svelte)$/.test(path)) return javascript({ typescript: path.endsWith(".ts") });
    return [];
  }

  function theme() {
    return EditorView.theme({
      "&": { background: "#14110c", color: "#f4ede0", height: "220px" },
      ".cm-scroller": { fontFamily: '"Fragment Mono", monospace' },
      ".cm-gutters": { background: "#221c14", color: "#b7a48a", border: "none" },
      ".cm-activeLine": { background: "#2c2418" },
    });
  }

  function mount(node, text, editable) {
    const view = new EditorView({
      state: EditorState.create({
        doc: text,
        extensions: [
          basicSetup,
          language(),
          theme(),
          EditorView.editable.of(editable),
          EditorState.readOnly.of(!editable),
          ...(editable
            ? [EditorView.updateListener.of((update) => {
                if (update.docChanged) result = update.state.doc.toString();
              })]
            : []),
        ],
      }),
      parent: node,
    });
    views.push(view);
    return view;
  }

  $effect(() => {
    views.forEach((view) => view.destroy());
    views = [];
    if (oursEl) mount(oursEl, ours, false);
    if (resultEl) mount(resultEl, result, true);
    if (theirsEl) mount(theirsEl, theirs, false);
    return () => views.forEach((view) => view.destroy());
  });
</script>

<div class="merge">
  <div>
    <p class="muted">Nuestro</p>
    <div class="cm" bind:this={oursEl}></div>
  </div>
  <div>
    <p class="muted">Resultado</p>
    <div class="cm" bind:this={resultEl}></div>
    <button class="primary" onclick={() => onsave(result)}>Guardar resultado</button>
  </div>
  <div>
    <p class="muted">Suyo</p>
    <div class="cm" bind:this={theirsEl}></div>
  </div>
</div>
