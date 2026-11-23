import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { invoke } from "@tauri-apps/api/core";
import { capitulo } from "./capitulo";

const PALAVRAS = 10_000;
const INSERCOES = 200;
const PASSO_DE_ROLAGEM = 120;
const TECLAS_POR_RELATORIO = 50;
const FRASE_DIGITADA = "ela abriu a janela e esperou o barulho dos barcos ";

const painel = document.getElementById("medidas") as HTMLElement;
const linhas: string[] = [];

function relatar(linha: string): void {
  linhas.push(linha);
  painel.textContent = linhas.join("\n");
  // O terminal de `cargo tauri dev` recebe a mesma linha.
  void invoke("registrar_medida", { linha });
}

function quadro(): Promise<number> {
  return new Promise((resolve) => requestAnimationFrame(resolve));
}

function resumo(amostras: number[]): string {
  if (amostras.length === 0) return "sem amostras";
  const ordenadas = [...amostras].sort((a, b) => a - b);
  const em = (q: number) => ordenadas[Math.min(ordenadas.length - 1, Math.floor(q * ordenadas.length))];
  return `n=${ordenadas.length} p50=${em(0.5).toFixed(1)}ms p95=${em(0.95).toFixed(1)}ms max=${ordenadas[ordenadas.length - 1].toFixed(1)}ms`;
}

const texto = capitulo(PALAVRAS);
const view = new EditorView({
  parent: document.getElementById("folha") as HTMLElement,
  state: EditorState.create({
    doc: texto,
    extensions: [
      EditorView.lineWrapping,
      // O editor ocupa a janela e rola por dentro: sem isto ele cresce até o
      // tamanho do texto e o CodeMirror desenha todas as linhas de uma vez.
      EditorView.theme({ "&": { height: "100vh" }, ".cm-scroller": { overflow: "auto" } }),
    ],
  }),
});

/** Inserção no meio do texto, medida de três jeitos:
 *  - até o segundo quadro depois dela (o primeiro é o da medição de layout do
 *    CodeMirror, o segundo já foi pintado). O piso deste número é de dois
 *    períodos da tela: 33,3 ms a 60 Hz;
 *  - o trabalho síncrono da tecla: `dispatch` mais o layout forçado;
 *  - o intervalo entre quadros durante as inserções, que mostra quadro perdido. */
async function medirInsercoes(): Promise<string[]> {
  const ateOQuadro: number[] = [];
  const sincrono: number[] = [];
  const entreQuadros: number[] = [];
  const meio = Math.floor(view.state.doc.length / 2);
  view.dispatch({ selection: { anchor: meio }, scrollIntoView: true });
  await quadro();
  let anterior = await quadro();
  for (let i = 0; i < INSERCOES; i += 1) {
    const inicio = performance.now();
    const posicao = view.state.selection.main.head;
    // Letras e espaços de uma frase, como na digitação, e não 200 vezes a
    // mesma letra, que criaria uma palavra de 200 caracteres.
    const letra = FRASE_DIGITADA[i % FRASE_DIGITADA.length];
    view.dispatch({ changes: { from: posicao, insert: letra }, selection: { anchor: posicao + 1 } });
    // Ler a geometria obriga o motor a refazer o layout agora.
    void view.contentDOM.getBoundingClientRect().height;
    sincrono.push(performance.now() - inicio);
    const primeiro = await quadro();
    const segundo = await quadro();
    entreQuadros.push(primeiro - anterior, segundo - primeiro);
    anterior = segundo;
    ateOQuadro.push(performance.now() - inicio);
  }
  return [
    `inserções até o 2º quadro: ${resumo(ateOQuadro)}`,
    `inserções, trabalho síncrono (dispatch + layout): ${resumo(sincrono)}`,
    `inserções, intervalo entre quadros: ${resumo(entreQuadros)}`,
  ];
}

/** Intervalo entre quadros rolando do topo até o fim. */
async function medirRolagem(): Promise<string> {
  const rolagem = view.scrollDOM;
  rolagem.scrollTop = 0;
  await quadro();
  const intervalos: number[] = [];
  let anterior = await quadro();
  while (rolagem.scrollTop + rolagem.clientHeight < rolagem.scrollHeight - 1) {
    const antes = rolagem.scrollTop;
    rolagem.scrollTop += PASSO_DE_ROLAGEM;
    // Chegou ao fim mesmo que o arredondamento das alturas diga que falta
    // um pixel: sem isto o laço não terminaria.
    if (rolagem.scrollTop === antes) break;
    const agora = await quadro();
    intervalos.push(agora - anterior);
    anterior = agora;
  }
  return resumo(intervalos);
}

/** Tecla de verdade: do `keydown` que escreveu até o segundo quadro depois
 *  da mudança. Só o último `keydown` vale, e ele é consumido pelo `input`:
 *  tecla que não escreve (seta, Shift, Ctrl) não fica na fila para ser
 *  cobrada da próxima letra. */
function medirTeclas(): void {
  let ultimaTecla: number | undefined;
  const amostras: number[] = [];
  view.contentDOM.addEventListener("keydown", (evento) => (ultimaTecla = evento.timeStamp), true);
  view.dom.addEventListener("input", () => {
    const inicio = ultimaTecla;
    ultimaTecla = undefined;
    // Sem tecla antes (colar, composição de acento): não há o que medir.
    if (inicio === undefined) return;
    requestAnimationFrame(() =>
      requestAnimationFrame(() => {
        amostras.push(performance.now() - inicio);
        if (amostras.length % TECLAS_POR_RELATORIO === 0) {
          relatar(`teclas reais: ${resumo(amostras.slice(-TECLAS_POR_RELATORIO))}`);
        }
      }),
    );
  });
}

async function principal(): Promise<void> {
  await quadro();
  const linhasDoTexto = view.state.doc.lines;
  const fonte = getComputedStyle(view.contentDOM.querySelector(".cm-line") as Element);
  relatar(`texto: ${PALAVRAS} palavras, ${linhasDoTexto} parágrafos, ${view.state.doc.length} caracteres`);
  relatar(`linha: font-family=${fonte.fontFamily} align=${fonte.textAlign} indent=${fonte.textIndent} line-height=${fonte.lineHeight}`);
  relatar(`navegador: ${navigator.userAgent}`);
  for (const linha of await medirInsercoes()) relatar(linha);
  const rolagem = view.scrollDOM;
  relatar(`rolagem: área ${rolagem.clientHeight}px de ${rolagem.scrollHeight}px; ${await medirRolagem()}`);
  view.dispatch({ selection: { anchor: Math.floor(view.state.doc.length / 2) }, scrollIntoView: true });
  view.focus();
  medirTeclas();
  relatar(`pronto: digite no texto; a cada ${TECLAS_POR_RELATORIO} teclas sai uma medida`);
}

principal().catch((erro: unknown) => relatar(`ERRO no spike: ${String(erro)}`));
