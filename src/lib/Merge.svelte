<script>
  import { untrack } from "svelte";
  import { EditorView, basicSetup } from "codemirror";
  import { EditorState, StateEffect, StateField } from "@codemirror/state";
  import { Decoration, EditorView as EV } from "@codemirror/view";
  import { javascript } from "@codemirror/lang-javascript";
  import { rust } from "@codemirror/lang-rust";
  import { json } from "@codemirror/lang-json";
  import { markdown } from "@codemirror/lang-markdown";
  import { html } from "@codemirror/lang-html";
  import { css } from "@codemirror/lang-css";

  let { path = "", ours = "", theirs = "", result = $bindable(""), onsave, onours, ontheirs } = $props();

  let oursEl;
  let resultEl;
  let theirsEl;
  let views = [];
  let oursScroll = $state(0);
  let theirsScroll = $state(0);
  let resultScroll = $state(0);

  function syncScroll(source, target) {
    const ratio = source / (source === oursScroll ? oursEl?.scrollHeight : source === theirsScroll ? theirsEl?.scrollHeight : resultEl?.scrollHeight || 1);
    if (target === oursScroll) oursEl?.scrollTo({ top: ratio * (oursEl.scrollHeight || 1) });
    if (target === theirsScroll) theirsEl?.scrollTo({ top: ratio * (theirsEl.scrollHeight || 1) });
    if (target === resultScroll) resultEl?.scrollTo({ top: ratio * (resultEl.scrollHeight || 1) });
  }

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
      "&": { background: "transparent", color: "#f4ede0", height: "100%" },
      ".cm-scroller": { fontFamily: '"Fragment Mono", monospace', overflow: "auto" },
      ".cm-gutters": { background: "#1a140e", color: "#b7a48a", border: "none" },
      ".cm-activeLine": { background: "#2c2418" },
      ".cm-content": { caretColor: "#f5b942" },
      ".cm-line.cm-add": { backgroundColor: "#1e3d20" },
      ".cm-line.cm-del": { backgroundColor: "#3d1d16" },
    });
  }

  const setDiff = StateEffect.define();

  const diffField = StateField.define({
    create() { return Decoration.none; },
    update(deco, tr) {
      deco = deco.map(tr.changes);
      for (const e of tr.effects) {
        if (e.is(setDiff)) {
          const decorations = [];
          for (const [line, kind] of e.value.lines) {
            decorations.push(
              Decoration.line({ class: kind === "add" ? "cm-add" : "cm-del" }).range(line)
            );
          }
          deco = Decoration.set(decorations, true);
        }
      }
      return deco;
    },
    provide: (f) => EV.decorations.from(f),
  });

  function computeDiffLines(resultText, baseText) {
    const resultLines = resultText.split("\n");
    const baseLines = baseText.split("\n");
    const lines = new Map();
    let i = 0, j = 0;
    
    while (i < resultLines.length || j < baseLines.length) {
      if (i < resultLines.length && j < baseLines.length && resultLines[i] === baseLines[j]) {
        i++; j++;
        continue;
      }
      
      if (j < baseLines.length && (i >= resultLines.length || baseLines[j] === resultLines[i + 1] || baseLines[j] === resultLines[i + 2])) {
        lines.set(i, "del");
        j++;
        continue;
      }
      
      if (i < resultLines.length && (j >= baseLines.length || resultLines[i] === baseLines[j + 1] || resultLines[i] === baseLines[j + 2])) {
        lines.set(i, "add");
        i++;
        continue;
      }
      
      if (i < resultLines.length) {
        lines.set(i, "add");
        i++;
      }
      if (j < baseLines.length) {
        j++;
      }
    }
    
    return lines;
  }

  function mount(node, text, editable) {
    const view = new EditorView({
      state: EditorState.create({
        doc: text,
        extensions: [
          basicSetup,
          language(),
          theme(),
          diffField,
          EditorView.editable.of(editable),
          EditorState.readOnly.of(!editable),
          EditorView.lineWrapping,
          ...(editable
            ? [EditorView.updateListener.of((update) => {
                if (update.docChanged) result = update.state.doc.toString();
              })]
            : []),
          EditorView.domEventHandlers({
            scroll: (event, view) => {
              const el = view.scrollDOM;
              if (el === oursEl) syncScroll(el.scrollTop, theirsScroll);
              if (el === theirsEl) syncScroll(el.scrollTop, oursScroll);
            },
          }),
        ],
      }),
      parent: node,
    });
    views.push(view);
    return view;
  }

  $effect(() => {
    path;
    const oursText = ours;
    const theirsText = theirs;
    const oursNode = oursEl;
    const resultNode = resultEl;
    const theirsNode = theirsEl;
    const start = untrack(() => result);
    views.forEach((view) => view.destroy());
    views = [];
    if (oursNode) {
      const view = mount(oursNode, oursText, false);
      const diffLines = computeDiffLines(oursText, start);
      view.dispatch({ effects: setDiff.of({ lines: diffLines }) });
    }
    if (resultNode) mount(resultNode, start, true);
    if (theirsNode) {
      const view = mount(theirsNode, theirsText, false);
      const diffLines = computeDiffLines(theirsText, start);
      view.dispatch({ effects: setDiff.of({ lines: diffLines }) });
    }
    return () => views.forEach((view) => view.destroy());
  });
</script>

<div class="merge">
  <div class="merge-top">
    <section class="pane ours">
      <header>
        <span>Nuestro</span>
        <button type="button" onclick={() => onours?.()}>Usar este</button>
      </header>
      <div class="cm" bind:this={oursEl}></div>
    </section>
    <section class="pane theirs">
      <header>
        <span>Suyo</span>
        <button type="button" onclick={() => ontheirs?.()}>Usar este</button>
      </header>
      <div class="cm" bind:this={theirsEl}></div>
    </section>
  </div>
  <section class="pane result">
    <header>
      <span>Resultado</span>
      <button type="button" class="primary" onclick={() => onsave?.(result)}>Guardar</button>
    </header>
    <div class="cm" bind:this={resultEl}></div>
  </section>
</div>

<style>
  .merge {
    display: grid;
    grid-template-rows: minmax(0, 1fr) minmax(0, 1.2fr);
    gap: 10px;
    min-height: 0;
    height: 100%;
  }
  .merge-top {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 10px;
    min-height: 0;
  }
  .pane {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: #100e0b;
    border: 1px solid #2a2218;
    border-radius: 14px;
    overflow: hidden;
    box-shadow: inset 0 1px 0 #f5b94222;
  }
  .pane.ours { box-shadow: inset 3px 0 0 #8ea34a, inset 0 1px 0 #f5b94222; }
  .pane.result { box-shadow: inset 3px 0 0 #f5b942, inset 0 1px 0 #f5b94222; }
  .pane.theirs { box-shadow: inset 3px 0 0 #e85d4c, inset 0 1px 0 #f5b94222; }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: none;
    padding: 8px 10px;
    border-bottom: 1px solid #2a2218;
    background: #16110c;
  }
  header span {
    color: #ffe7a3;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.14em;
    text-transform: uppercase;
  }
  header button {
    margin-left: auto;
    padding: 4px 10px;
    font-size: 11px;
  }
  .cm {
    flex: 1;
    min-height: 0;
    height: 100%;
  }
  .cm :global(.cm-editor) { height: 100%; }
  @media (max-width: 1100px) {
    .merge {
      grid-template-rows: minmax(180px, 1fr) minmax(220px, 1.2fr);
    }
    .merge-top {
      grid-template-columns: 1fr;
      grid-template-rows: minmax(180px, 1fr) minmax(180px, 1fr);
    }
  }
</style>
