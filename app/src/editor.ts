// O editor: CodeMirror 6 com as linhas marcadas para o CSS do livro valer
// direto nelas. A tipografia vem toda do CSS gerado pelo crate `estilo`.

import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { defineLanguageFacet, Language, syntaxHighlighting } from "@codemirror/language";
import { EditorState, type Extension, RangeSetBuilder, StateField } from "@codemirror/state";
import { Decoration, type DecorationSet, EditorView, keymap } from "@codemirror/view";
import { tagHighlighter, tags } from "@lezer/highlight";
import { parser, Strikethrough } from "@lezer/markdown";
import { marcarLinhas } from "./blocos.ts";

export interface Editor {
  /** Troca o texto inteiro (outro capítulo): o desfazer começa do zero. */
  definirTexto(texto: string): void;
  texto(): string;
  /** O texto selecionado; vazio se não há seleção. */
  selecao(): string;
  focar(): void;
  /** Digita no fim do texto, como o autor faria. Usado pelo teste funcional. */
  digitarNoFim(texto: string): void;
  readonly view: EditorView;
}

/** Uma decoração por linha que precisa de marca: estilo nomeado, título,
 *  marcador `:::` ou linha vazia. Refeita a cada mudança do texto. */
function decorar(estado: EditorState): DecorationSet {
  const marcas = marcarLinhas(estado.doc.toString().split("\n"));
  const decoracoes = new RangeSetBuilder<Decoration>();
  marcas.forEach((marca, indice) => {
    const linha = estado.doc.line(indice + 1);
    const classes: string[] = [];
    if (marca.titulo) classes.push("md-novel-titulo");
    if (marca.marcador) classes.push("md-novel-marcador");
    if (linha.length === 0) classes.push("md-novel-vazia");
    if (classes.length === 0 && marca.estilo === null) return;
    decoracoes.add(
      linha.from,
      linha.from,
      Decoration.line({
        class: classes.join(" ") || undefined,
        // O atributo que o CSS do livro procura (ADR 0002).
        attributes: marca.estilo === null ? undefined : { "data-md-novel-estilo": marca.estilo },
      }),
    );
  });
  return decoracoes.finish();
}

const marcasDasLinhas = StateField.define<DecorationSet>({
  create: decorar,
  update: (atual, transacao) => (transacao.docChanged ? decorar(transacao.state) : atual),
  provide: (campo) => EditorView.decorations.from(campo),
});

// O Markdown à vista: o analisador do Lezer acha a ênfase, os títulos e os
// sinais, e cada um ganha uma classe que o `app.css` pinta. Só o analisador
// (`@lezer/markdown`), sem o pacote `lang-markdown`, que traria junto HTML,
// CSS e JavaScript. O `:::` do Pandoc ele lê como texto comum; quem cuida
// dele é o `marcarLinhas`.
const markdown = new Language(defineLanguageFacet(), parser.configure(Strikethrough), [], "markdown");
const realce = tagHighlighter([
  { tag: tags.emphasis, class: "md-novel-enfase" },
  { tag: tags.strong, class: "md-novel-forte" },
  { tag: tags.strikethrough, class: "md-novel-riscado" },
  { tag: tags.heading1, class: "md-novel-t1" },
  { tag: tags.heading2, class: "md-novel-t2" },
  { tag: tags.heading3, class: "md-novel-t3" },
  { tag: [tags.link, tags.url], class: "md-novel-link" },
  { tag: tags.monospace, class: "md-novel-codigo" },
  { tag: tags.quote, class: "md-novel-citacao" },
  { tag: tags.comment, class: "md-novel-comentario" },
  // Os sinais digitados: `*`, `#`, `>`, o traço da lista.
  { tag: tags.processingInstruction, class: "md-novel-sinal" },
]);

export interface AoEditar {
  /** O texto mudou. */
  aoMudar(): void;
  /** O autor pediu para salvar (Ctrl+S). */
  aoSalvar(): void;
  /** A seleção mudou, com ou sem mudança no texto. */
  aoSelecionar(): void;
}

export function criarEditor(pai: HTMLElement, { aoMudar, aoSalvar, aoSelecionar }: AoEditar): Editor {
  const extensoes: Extension = [
    EditorView.lineWrapping,
    history(),
    keymap.of([
      { key: "Mod-s", preventDefault: true, run: () => (aoSalvar(), true) },
      ...defaultKeymap,
      ...historyKeymap,
    ]),
    marcasDasLinhas,
    markdown.extension,
    syntaxHighlighting(realce),
    // Altura fixa e rolagem própria: sem isto o CodeMirror desenha o
    // capítulo inteiro de uma vez (lição do spike, ADR 0003).
    EditorView.theme({ "&": { height: "100%" }, ".cm-scroller": { overflow: "auto" } }),
    EditorView.updateListener.of((atualizacao) => {
      if (atualizacao.docChanged) aoMudar();
      if (atualizacao.selectionSet) aoSelecionar();
    }),
    EditorView.contentAttributes.of({ spellcheck: "true", lang: "pt-BR" }),
  ];
  const view = new EditorView({ parent: pai, state: EditorState.create({ doc: "", extensions: extensoes }) });
  return {
    view,
    definirTexto: (texto) => view.setState(EditorState.create({ doc: texto, extensions: extensoes })),
    texto: () => view.state.doc.toString(),
    selecao: () =>
      view.state.selection.ranges
        .filter((trecho) => !trecho.empty)
        .map((trecho) => view.state.sliceDoc(trecho.from, trecho.to))
        .join("\n"),
    focar: () => view.focus(),
    digitarNoFim: (texto) => {
      const fim = view.state.doc.length;
      view.dispatch({
        changes: { from: fim, insert: texto },
        selection: { anchor: fim + texto.length },
        userEvent: "input.type",
        scrollIntoView: true,
      });
    },
  };
}
