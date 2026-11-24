// O editor: CodeMirror 6 com as linhas marcadas para o CSS do livro valer
// direto nelas. A tipografia vem toda do CSS gerado pelo crate `estilo`.

import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { defineLanguageFacet, Language, syntaxHighlighting, syntaxTree } from "@codemirror/language";
import { findNext, openSearchPanel, SearchQuery, search, searchKeymap, setSearchQuery } from "@codemirror/search";
import { EditorState, type Extension, type RangeSet, RangeSetBuilder, RangeValue, StateField } from "@codemirror/state";
import { Decoration, type DecorationSet, EditorView, keymap } from "@codemirror/view";
import { tagHighlighter, tags } from "@lezer/highlight";
import { parser, Strikethrough } from "@lezer/markdown";
import { marcarLinhas } from "./blocos.ts";
import { FRASES_DA_BUSCA } from "./busca.ts";
import { enterCriaParagrafo } from "./paragrafo.ts";
import { converter, substituicao } from "./tipografia.ts";

export interface Editor {
  /** Troca o texto inteiro (outro capítulo): o desfazer começa do zero. */
  definirTexto(texto: string): void;
  texto(): string;
  /** O texto selecionado; vazio se não há seleção. */
  selecao(): string;
  focar(): void;
  /** Leva a linha do cursor para o meio da tela. */
  centrar(): void;
  /** A posição do cursor no texto. */
  posicao(): number;
  /** Leva o cursor para `posicao` (ou para o fim, se o texto é mais curto) e mostra a linha. */
  irPara(posicao: number): void;
  /** Troca no texto inteiro os hifens duplos por travessão e as aspas retas
   *  por curvas, e diz quantas trocas fez. */
  converterSinais(): number;
  /** Põe `texto` na busca e seleciona a próxima ocorrência. Usado pelo teste funcional. */
  buscarProximo(texto: string): void;
  /** Digita no fim do texto, como o autor faria. Usado pelo teste funcional. */
  digitarNoFim(texto: string): void;
  readonly view: EditorView;
}

/** O trecho que o cursor atravessa de uma vez: a linha em branco escondida
 *  entre dois parágrafos, com as duas quebras de linha em volta dela. */
class Separador extends RangeValue {}
const SEPARADOR = new Separador();

interface Marcas {
  decoracoes: DecorationSet;
  separadores: RangeSet<Separador>;
}

/** Uma decoração por linha que precisa de marca: estilo nomeado, título,
 *  marcador `:::`, linha vazia ou separadora de parágrafos. Refeita a cada
 *  mudança do texto. */
function decorar(estado: EditorState): Marcas {
  const marcas = marcarLinhas(estado.doc.toString().split("\n"));
  const decoracoes = new RangeSetBuilder<Decoration>();
  const separadores = new RangeSetBuilder<Separador>();
  marcas.forEach((marca, indice) => {
    const linha = estado.doc.line(indice + 1);
    const classes: string[] = [];
    if (marca.titulo) classes.push("md-novel-titulo");
    if (marca.marcador) classes.push("md-novel-marcador");
    if (linha.length === 0) classes.push("md-novel-vazia");
    if (marca.separador) {
      // Parágrafos como no Word (ADR 0003): a linha some da tela, e do fim
      // do parágrafo de cima ao começo do de baixo é um passo só do cursor.
      // Backspace no começo do parágrafo apaga o trecho inteiro e junta os dois.
      classes.push("md-novel-separador");
      separadores.add(linha.from - 1, linha.to + 1, SEPARADOR);
    }
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
  return { decoracoes: decoracoes.finish(), separadores: separadores.finish() };
}

const marcasDasLinhas = StateField.define<Marcas>({
  create: decorar,
  update: (atual, transacao) => (transacao.docChanged ? decorar(transacao.state) : atual),
  provide: (campo) => [
    EditorView.decorations.from(campo, (marcas) => marcas.decoracoes),
    EditorView.atomicRanges.of((view) => view.state.field(campo).separadores),
  ],
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
  /** A máquina de escrever está ligada: a linha do cursor vai para o meio. */
  maquinaDeEscrever(): boolean;
  /** Aspas curvas e travessão ao digitar estão ligados. */
  tipografia(): boolean;
}

/** Onde o texto é código, endereço ou HTML: ali o sinal digitado fica reto. */
const SEM_TIPOGRAFIA = new Set([
  "FencedCode",
  "CodeBlock",
  "InlineCode",
  "CodeText",
  "CodeInfo",
  "URL",
  "HTMLTag",
  "HTMLBlock",
  "Comment",
  "CommentBlock",
]);

function dentroDeCodigo(estado: EditorState, posicao: number): boolean {
  for (let no: ReturnType<typeof syntaxTree>["topNode"] | null = syntaxTree(estado).resolveInner(posicao, -1); no; no = no.parent) {
    if (SEM_TIPOGRAFIA.has(no.name)) return true;
  }
  return false;
}

export function criarEditor(
  pai: HTMLElement,
  { aoMudar, aoSalvar, aoSelecionar, maquinaDeEscrever, tipografia }: AoEditar,
): Editor {
  // Um pedido de centro por quadro: digitar depressa não empilha rolagens.
  let centroPedido = false;
  const centrar = (view: EditorView): void => {
    if (centroPedido) return;
    centroPedido = true;
    requestAnimationFrame(() => {
      centroPedido = false;
      // Só o efeito de rolagem: não muda texto nem seleção, então não volta
      // a pedir centro.
      view.dispatch({ effects: EditorView.scrollIntoView(view.state.selection.main.head, { y: "center" }) });
    });
  };
  const extensoes: Extension = [
    EditorView.lineWrapping,
    history(),
    keymap.of([
      { key: "Mod-s", preventDefault: true, run: () => (aoSalvar(), true) },
      // Enter cria um parágrafo: o app grava a linha em branco que o
      // Markdown pede (ADR 0003). Com Shift, ou onde a linha em branco
      // estragaria o Markdown, vale o Enter de sempre, mais abaixo.
      {
        key: "Enter",
        run: (view) => {
          const { ranges, main } = view.state.selection;
          if (ranges.length !== 1 || dentroDeCodigo(view.state, main.from)) return false;
          const linha = view.state.doc.lineAt(main.from);
          const deCimaVazia = linha.number > 1 && view.state.doc.line(linha.number - 1).length === 0;
          if (!enterCriaParagrafo(linha.text, { noComeco: main.from === linha.from, deCimaVazia })) return false;
          view.dispatch({
            changes: { from: main.from, to: main.to, insert: "\n\n" },
            selection: { anchor: main.from + 2 },
            userEvent: "input",
            scrollIntoView: true,
          });
          return true;
        },
      },
      // Buscar e substituir são o mesmo painel: Ctrl+H abre o de Ctrl+F.
      { key: "Mod-h", preventDefault: true, run: openSearchPanel },
      ...searchKeymap,
      ...defaultKeymap,
      ...historyKeymap,
    ]),
    search({ top: true }),
    EditorState.phrases.of(FRASES_DA_BUSCA),
    // Aspas curvas e travessão: uma tecla por vez, sem seleção, fora de código.
    EditorView.inputHandler.of((view, de, ate, texto) => {
      if (!tipografia() || de !== ate || dentroDeCodigo(view.state, de)) return false;
      const linha = view.state.doc.lineAt(de);
      const troca = substituicao(linha.text.slice(0, de - linha.from), texto);
      if (troca === null) return false;
      const inicio = de - troca.apagar;
      view.dispatch({
        changes: { from: inicio, to: ate, insert: troca.inserir },
        selection: { anchor: inicio + troca.inserir.length },
        userEvent: "input.type",
        scrollIntoView: true,
      });
      return true;
    }),
    marcasDasLinhas,
    markdown.extension,
    syntaxHighlighting(realce),
    // Altura fixa e rolagem própria: sem isto o CodeMirror desenha o
    // capítulo inteiro de uma vez (lição do spike, ADR 0003).
    EditorView.theme({ "&": { height: "100%" }, ".cm-scroller": { overflow: "auto" } }),
    EditorView.updateListener.of((atualizacao) => {
      if (atualizacao.docChanged) aoMudar();
      if (atualizacao.selectionSet) aoSelecionar();
      // Máquina de escrever: digitar ou andar com o teclado leva a linha
      // para o meio. Clicar e arrastar com o mouse não: a tela fugiria de
      // debaixo do ponteiro.
      const peloMouse = atualizacao.transactions.some((transacao) => transacao.isUserEvent("select.pointer"));
      if (maquinaDeEscrever() && (atualizacao.docChanged || atualizacao.selectionSet) && !peloMouse) {
        centrar(atualizacao.view);
      }
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
    centrar: () => centrar(view),
    posicao: () => view.state.selection.main.head,
    irPara: (posicao) => {
      const destino = Math.min(Math.max(0, posicao), view.state.doc.length);
      view.dispatch({
        selection: { anchor: destino },
        effects: EditorView.scrollIntoView(destino, { y: "center" }),
      });
    },
    converterSinais: () => {
      const { mudancas } = converter(view.state.doc.toString());
      if (mudancas.length === 0) return 0;
      // Uma transação só: um Ctrl+Z desfaz a conversão inteira.
      view.dispatch({
        changes: mudancas.map(({ de, ate, inserir }) => ({ from: de, to: ate, insert: inserir })),
        userEvent: "input.converter",
      });
      return mudancas.length;
    },
    buscarProximo: (texto) => {
      view.dispatch({ effects: setSearchQuery.of(new SearchQuery({ search: texto })) });
      findNext(view);
    },
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
