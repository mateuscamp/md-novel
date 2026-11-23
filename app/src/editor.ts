// O editor: CodeMirror 6 com as linhas marcadas para o CSS do livro valer
// direto nelas. A tipografia vem toda do CSS gerado pelo crate `estilo`.

import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { EditorState, type Extension, RangeSetBuilder, StateField } from "@codemirror/state";
import { Decoration, type DecorationSet, EditorView, keymap } from "@codemirror/view";
import { marcarLinhas } from "./blocos.ts";

export interface Editor {
  /** Troca o texto inteiro (outro capítulo): o desfazer começa do zero. */
  definirTexto(texto: string): void;
  texto(): string;
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

export function criarEditor(pai: HTMLElement, aoMudar: () => void, aoSalvar: () => void): Editor {
  const extensoes: Extension = [
    EditorView.lineWrapping,
    history(),
    keymap.of([
      { key: "Mod-s", preventDefault: true, run: () => (aoSalvar(), true) },
      ...defaultKeymap,
      ...historyKeymap,
    ]),
    marcasDasLinhas,
    // Altura fixa e rolagem própria: sem isto o CodeMirror desenha o
    // capítulo inteiro de uma vez (lição do spike, ADR 0003).
    EditorView.theme({ "&": { height: "100%" }, ".cm-scroller": { overflow: "auto" } }),
    EditorView.updateListener.of((atualizacao) => {
      if (atualizacao.docChanged) aoMudar();
    }),
    EditorView.contentAttributes.of({ spellcheck: "true", lang: "pt-BR" }),
  ];
  const view = new EditorView({ parent: pai, state: EditorState.create({ doc: "", extensions: extensoes }) });
  return {
    view,
    definirTexto: (texto) => view.setState(EditorState.create({ doc: texto, extensions: extensoes })),
    texto: () => view.state.doc.toString(),
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
