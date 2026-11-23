// A janela: lista de capítulos, editor e avisos. Quem lê e grava é o Rust;
// aqui só se pede e se mostra.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { criarEditor } from "./editor.ts";
import { fonteInstalada, linhaSimples } from "./fonte.ts";
import { type Estado, Gravador } from "./gravador.ts";

interface LivroAberto {
  pasta: string;
  nome: string;
  capitulos: string[];
  css: string | null;
  fontes: string[];
  avisos: string[];
}

interface ExportacaoFeita {
  destino: string;
  avisos: string[];
}

interface ErroDeComando {
  codigo: "erro" | "mudou-por-fora";
  mensagem: string;
}

function elemento<T extends HTMLElement>(id: string): T {
  const achado = document.getElementById(id);
  if (!achado) throw new Error(`falta o elemento #${id} no index.html`);
  return achado as T;
}

const nomeDoLivro = elemento("nome-do-livro");
const listaDeCapitulos = elemento<HTMLUListElement>("capitulos");
const nomeDoCapitulo = elemento("nome-do-capitulo");
const estado = elemento("estado");
const botaoSalvar = elemento<HTMLButtonElement>("salvar");
const botaoExportar = elemento<HTMLButtonElement>("exportar");
const avisos = elemento("avisos");
const vazio = elemento("vazio");
const folha = elemento("folha");

// O CSS do livro entra numa folha de estilo construída, que troca inteira
// quando o livro muda.
const cssDoLivro = new CSSStyleSheet();
document.adoptedStyleSheets = [...document.adoptedStyleSheets, cssDoLivro];

let capituloAtual: string | null = null;
/** O aviso do problema de gravação em curso: um só, trocado a cada mudança. */
let avisoDeGravacao: HTMLElement | null = null;
/** Os avisos da última exportação, trocados a cada nova. */
let avisosDaExportacao: HTMLElement[] = [];

const gravador = new Gravador({
  gravar: (nome, texto) =>
    invoke<boolean>("salvar_capitulo", { nome, texto }).then(() => registrar(`salvo: ${nome}`)),
  texto: () => editor.texto(),
  agendar: (tarefa, ms) => window.setTimeout(tarefa, ms),
  cancelar: (agendamento) => window.clearTimeout(agendamento as number),
  aoMudarEstado: mostrarEstadoDaGravacao,
});
const editor = criarEditor(
  folha,
  () => gravador.mudou(),
  () => void gravador.salvarAgora(),
);

function registrar(linha: string): void {
  void invoke("registrar", { linha });
}

function comoErro(motivo: unknown): ErroDeComando {
  if (typeof motivo === "object" && motivo !== null && "mensagem" in motivo) return motivo as ErroDeComando;
  return { codigo: "erro", mensagem: String(motivo) };
}

function avisar(
  texto: string,
  classe: "" | "erro" | "feito" = "",
  acao?: { rotulo: string; fazer: () => void },
  dispensavel = true,
): HTMLElement {
  const paragrafo = document.createElement("p");
  const mensagem = document.createElement("span");
  mensagem.textContent = texto;
  paragrafo.append(mensagem);
  if (classe) paragrafo.classList.add(classe);
  if (acao) {
    const botao = document.createElement("button");
    botao.type = "button";
    botao.textContent = acao.rotulo;
    botao.addEventListener("click", acao.fazer);
    paragrafo.append(botao);
  }
  // O aviso comum pode ser dispensado, para nenhum ficar na tela para sempre.
  if (dispensavel) {
    const fechar = document.createElement("button");
    fechar.type = "button";
    fechar.className = "fechar";
    fechar.textContent = "×";
    fechar.title = "Dispensar este aviso";
    fechar.setAttribute("aria-label", "Dispensar este aviso");
    fechar.addEventListener("click", () => paragrafo.remove());
    paragrafo.append(fechar);
  }
  avisos.append(paragrafo);
  return paragrafo;
}

/** Aviso de texto que não está no disco. Não pode ser dispensado: ele some
 *  sozinho quando o problema acaba, e pode carregar o único botão que
 *  resolve um conflito. */
function avisarDaGravacao(texto: string, acao?: { rotulo: string; fazer: () => void }): HTMLElement {
  return avisar(texto, "erro", acao, false);
}

function mostrarEstadoDaGravacao(novo: Estado): void {
  avisoDeGravacao?.remove();
  avisoDeGravacao = null;
  estado.classList.toggle("erro", novo.tipo === "conflito" || novo.tipo === "erro");
  botaoSalvar.disabled = novo.tipo === "limpo" || novo.tipo === "salvo";
  switch (novo.tipo) {
    case "limpo":
      estado.textContent = "";
      break;
    case "editando":
      estado.textContent = "Editando…";
      break;
    case "salvo":
      estado.textContent = `Salvo às ${new Date().toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" })}`;
      break;
    case "conflito": {
      estado.textContent = "Não salvo: o arquivo mudou por fora";
      const nome = capituloAtual;
      avisoDeGravacao = avisarDaGravacao(novo.mensagem, {
        rotulo: "Recarregar do disco (descarta o que foi digitado aqui)",
        fazer: () => (nome === null ? undefined : void abrirCapitulo(nome, true)),
      });
      registrar(`não salvo: mudou por fora`);
      break;
    }
    case "erro":
      estado.textContent = "Não salvo";
      avisoDeGravacao = avisarDaGravacao(novo.mensagem);
      registrar(`não salvo: erro`);
      break;
  }
}

/** O que dizer quando sobrou texto fora do disco e o autor quer sair dele. */
const TEXTO_NAO_SALVO =
  "Este capítulo tem texto que não foi salvo. Resolva o aviso acima antes de sair dele, para não perder o que digitou.";

async function abrirCapitulo(nome: string, descartarMudancas = false): Promise<void> {
  // Trocar de capítulo espera a gravação em curso e salva o que falta. Se
  // sobrou texto fora do disco, fica onde está, a menos que o autor tenha
  // pedido para descartar.
  const salvo = await gravador.salvarAgora();
  if (!salvo && !descartarMudancas) {
    if (capituloAtual !== nome) avisoDeGravacao ??= avisarDaGravacao(TEXTO_NAO_SALVO);
    return;
  }
  try {
    const texto = await invoke<string>("abrir_capitulo", { nome });
    capituloAtual = nome;
    editor.definirTexto(texto);
    gravador.abrir(nome);
    nomeDoCapitulo.textContent = nome;
    for (const botao of listaDeCapitulos.querySelectorAll("button")) {
      botao.setAttribute("aria-current", String(botao.textContent === nome));
    }
    vazio.hidden = true;
    folha.hidden = false;
    editor.focar();
    registrar(`capítulo: ${nome}, ${editor.view.state.doc.lines} linhas`);
  } catch (motivo) {
    avisar(comoErro(motivo).mensagem, "erro");
  }
}

/** Exporta o livro inteiro para .docx. O Rust lê os capítulos do disco,
 *  então o que está na tela é salvo antes; o destino é escolhido no seletor
 *  do sistema, que o Rust abre. */
async function exportar(): Promise<void> {
  if (!(await gravador.salvarAgora())) {
    avisoDeGravacao ??= avisarDaGravacao(TEXTO_NAO_SALVO);
    return;
  }
  avisosDaExportacao.forEach((aviso) => aviso.remove());
  avisosDaExportacao = [avisar("Exportando o livro para .docx…")];
  botaoExportar.disabled = true;
  try {
    const feita = await invoke<ExportacaoFeita | null>("exportar_livro");
    avisosDaExportacao.forEach((aviso) => aviso.remove());
    avisosDaExportacao = [];
    // `null`: o autor cancelou no seletor.
    if (feita) {
      avisosDaExportacao = [
        avisar(`Livro exportado para ${feita.destino}`, "feito"),
        ...feita.avisos.map((aviso) => avisar(`Aviso da exportação: ${aviso}`)),
      ];
      registrar(`exportado: ${feita.avisos.length} avisos`);
    }
  } catch (motivo) {
    avisosDaExportacao.forEach((aviso) => aviso.remove());
    avisosDaExportacao = [avisar(comoErro(motivo).mensagem, "erro")];
    registrar("não exportado");
  } finally {
    botaoExportar.disabled = false;
  }
}

function aplicarEstilo(livro: LivroAberto): void {
  cssDoLivro.replaceSync(livro.css ?? "");
  const [corpo] = livro.fontes;
  if (corpo === undefined) {
    document.documentElement.style.removeProperty("--md-novel-linha-simples");
    return;
  }
  document.documentElement.style.setProperty("--md-novel-linha-simples", String(linhaSimples(corpo)));
  for (const fonte of livro.fontes) {
    if (!fonteInstalada(fonte)) {
      avisar(
        `A fonte "${fonte}" do estilo.yaml não está instalada neste computador: a tela está usando outra, e o Word vai escolher a dele. Instale a fonte para ver o livro como ele vai sair.`,
      );
    }
  }
}

async function mostrarLivro(livro: LivroAberto): Promise<void> {
  capituloAtual = null;
  nomeDoLivro.textContent = livro.nome;
  nomeDoLivro.title = livro.pasta;
  avisos.replaceChildren();
  avisoDeGravacao = null;
  avisosDaExportacao = [];
  botaoExportar.disabled = false;
  livro.avisos.forEach((aviso) => avisar(aviso));
  aplicarEstilo(livro);
  listaDeCapitulos.replaceChildren(
    ...livro.capitulos.map((nome) => {
      const item = document.createElement("li");
      const botao = document.createElement("button");
      botao.type = "button";
      botao.textContent = nome;
      botao.addEventListener("click", () => void abrirCapitulo(nome));
      item.append(botao);
      return item;
    }),
  );
  registrar(`livro: ${livro.capitulos.length} capítulos, css=${livro.css !== null}, avisos=${avisos.childElementCount}`);
  const [primeiro] = livro.capitulos;
  if (primeiro === undefined) {
    folha.hidden = true;
    vazio.hidden = false;
    vazio.textContent = "Esta pasta ainda não tem capítulos (.md).";
    return;
  }
  await abrirCapitulo(primeiro, true);
  await diagnosticar();
}

/** O que o navegador de fato aplicou, para o terminal e o teste funcional. */
async function diagnosticar(): Promise<void> {
  await new Promise((resolve) => requestAnimationFrame(resolve));
  const medir = (seletor: string) => {
    const linha = editor.view.contentDOM.querySelector(seletor);
    if (!linha) return "ausente";
    const estilo = getComputedStyle(linha);
    return `align=${estilo.textAlign} indent=${estilo.textIndent} style=${estilo.fontStyle} line-height=${estilo.lineHeight}`;
  };
  const rolagem = editor.view.scrollDOM;
  registrar(`linha do corpo: ${medir(".cm-line:not([data-md-novel-estilo]):not(.md-novel-titulo):not(.md-novel-marcador):not(.md-novel-vazia)")}`);
  registrar(`linha de estilo nomeado: ${medir(".cm-line[data-md-novel-estilo]")}`);
  registrar(`rolagem: área ${rolagem.clientHeight}px de ${rolagem.scrollHeight}px`);
  const roteiro = await invoke<{ digitar: string; fechar: boolean; exportar: boolean } | null>("roteiro_de_teste");
  if (roteiro) {
    editor.digitarNoFim(roteiro.digitar);
    registrar("digitado pelo roteiro de teste");
    if (roteiro.exportar) await exportar();
    if (roteiro.fechar) await invoke("teste_pedir_para_fechar");
  }
}

elemento("abrir-pasta").addEventListener("click", async () => {
  if (!(await gravador.salvarAgora())) {
    avisoDeGravacao ??= avisarDaGravacao(TEXTO_NAO_SALVO);
    return;
  }
  try {
    const livro = await invoke<LivroAberto | null>("escolher_pasta");
    if (livro) await mostrarLivro(livro);
  } catch (motivo) {
    avisar(comoErro(motivo).mensagem, "erro");
  }
});
botaoSalvar.addEventListener("click", () => void gravador.salvarAgora());
botaoExportar.addEventListener("click", () => void exportar());
// Sair da janela salva: o autor troca de programa sem pensar nisso.
window.addEventListener("blur", () => void gravador.salvarAgora());

// Fechar a janela: o Rust segura o fechamento e avisa aqui. Salva-se o que
// falta e só então a janela fecha. Se não deu para salvar, ela fica aberta
// com o aviso; pedir para fechar de novo fecha assim mesmo (o Rust deixa).
void listen("pedido-de-fechar", async () => {
  if (await gravador.salvarAgora()) {
    await invoke("fechar_janela");
    return;
  }
  avisar("Há texto não salvo neste capítulo. Feche de novo para sair sem salvar.", "erro");
});

invoke<LivroAberto | null>("livro_atual")
  .then((livro) => (livro ? mostrarLivro(livro) : undefined))
  .catch((motivo: unknown) => avisar(comoErro(motivo).mensagem, "erro"));
