// A janela: a lateral com o livro, a folha do editor, os avisos e a barra de
// status. Quem lê e grava é o Rust; aqui só se pede e se mostra.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { sugestaoDeNome } from "./capitulo.ts";
import {
  type AcaoDeZoom,
  acaoDaRoda,
  type Atalho,
  atalhoDaTecla,
  DESLIGADO,
  fatorDoZoom,
  LIGADO,
  lerDesligado,
  lerLigado,
  lerZoom,
  rotuloDoZoom,
  zoomDepoisDe,
} from "./conforto.ts";
import { criarEditor } from "./editor.ts";
import { teclaDoFoco } from "./foco.ts";
import { fonteInstalada, linhaSimples } from "./fonte.ts";
import { type Estado, Gravador } from "./gravador.ts";
import {
  type Alinhamento,
  type Campos,
  camposDe,
  comCampo,
  comEstiloNovo,
  type EstiloNoPainel,
  FONTES_SUGERIDAS,
  herdado,
  juntarMedida,
  nomeDeEstiloNovo,
  partirMedida,
  semEstilo,
} from "./painel.ts";
import { diaDe, escreverHoje, lerHoje, lerMeta, META_MAXIMA, rotuloDeHoje } from "./meta.ts";
import { escreverOnde, lerOnde } from "./onde.ts";
import {
  type CapituloNaLista,
  comTextoAoVivo,
  nomeDeExibicao,
  numero,
  rotuloDePalavras,
  totalDePalavras,
} from "./resumo.ts";
import { lerPreferencia, type Preferencia, proximaPreferencia, rotuloDaPreferencia, temaEfetivo } from "./tema.ts";
import { contarPalavras } from "./texto.ts";
import { ofertaDeConversao } from "./tipografia.ts";

interface LivroAberto {
  pasta: string;
  nome: string;
  capitulos: CapituloNaLista[];
  css: string | null;
  fontes: string[];
  avisos: string[];
  /** O aviso da pasta sem estilo.yaml, que ganha o botão de criar o estilo. */
  aviso_sem_estilo: string | null;
}

/** O que o corretor ortográfico carregou ao abrir o app (`EstadoDoCorretor` do Rust). */
interface EstadoDoCorretor {
  linguas: string[];
  aviso: string | null;
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
const resumoDoLivro = elemento("resumo-do-livro");
const listaDeCapitulos = elemento<HTMLOListElement>("capitulos");
const nomeDoCapitulo = elemento("nome-do-capitulo");
const arquivoDoCapitulo = elemento("arquivo-do-capitulo");
const estado = elemento("estado");
const palavrasDoCapitulo = elemento("palavras-do-capitulo");
const palavrasDaSelecao = elemento("palavras-da-selecao");
const palavrasDoLivro = elemento("palavras-do-livro");
const botaoSalvar = elemento<HTMLButtonElement>("salvar");
const botaoExportar = elemento<HTMLButtonElement>("exportar");
const botaoTema = elemento<HTMLButtonElement>("tema");
const botaoSairDoFoco = elemento<HTMLButtonElement>("sair-do-foco");
const botaoDaLateral = elemento<HTMLButtonElement>("alternar-lateral");
const botaoDaMaquina = elemento<HTMLButtonElement>("maquina");
const botaoDoZoom = elemento<HTMLButtonElement>("zoom");
const botaoDasAspas = elemento<HTMLButtonElement>("aspas");
const botaoDeConverter = elemento<HTMLButtonElement>("converter");
const botaoDeHoje = elemento<HTMLButtonElement>("hoje");
const botaoNovoCapitulo = elemento<HTMLButtonElement>("novo-capitulo");
const campoDoCapituloNovo = elemento<HTMLInputElement>("nome-do-capitulo-novo");
const campoDaMeta = elemento<HTMLInputElement>("campo-da-meta");
const avisos = elemento("avisos");
const vazio = elemento("vazio");
const textoDoVazio = elemento("texto-do-vazio");
const folha = elemento("folha");

// ---- Tema ----

// A escolha do autor fica no armazenamento da própria janela, que o sistema
// guarda na pasta de dados do app, fora da pasta do livro.
const CHAVE_DO_TEMA = "md-novel.tema";
const sistemaEscuro = window.matchMedia("(prefers-color-scheme: dark)");

function preferenciaGuardada(): Preferencia {
  try {
    return lerPreferencia(window.localStorage.getItem(CHAVE_DO_TEMA));
  } catch {
    // Sem armazenamento (perfil só de leitura): segue o sistema.
    return "sistema";
  }
}

let preferenciaDeTema = preferenciaGuardada();

function aplicarTema(): void {
  document.documentElement.dataset.tema = temaEfetivo(preferenciaDeTema, sistemaEscuro.matches);
  botaoTema.textContent = rotuloDaPreferencia(preferenciaDeTema);
}

// Antes de tudo: a janela já abre na cor certa.
aplicarTema();
sistemaEscuro.addEventListener("change", aplicarTema);
botaoTema.addEventListener("click", () => {
  preferenciaDeTema = proximaPreferencia(preferenciaDeTema);
  try {
    window.localStorage.setItem(CHAVE_DO_TEMA, preferenciaDeTema);
  } catch {
    // Não deu para guardar: o tema vale até a janela fechar.
  }
  aplicarTema();
});

// ---- Livro, capítulos e editor ----

// O CSS do livro entra numa folha de estilo construída, que troca inteira
// quando o livro muda.
const cssDoLivro = new CSSStyleSheet();
document.adoptedStyleSheets = [...document.adoptedStyleSheets, cssDoLivro];

/** A lista do livro. O capítulo aberto vale pelo texto que está na tela. */
let capitulos: CapituloNaLista[] = [];
let capituloAtual: string | null = null;
/** O aviso do problema de gravação em curso: um só, trocado a cada mudança. */
let avisoDeGravacao: HTMLElement | null = null;
/** Os avisos da última exportação, trocados a cada nova. */
let avisosDaExportacao: HTMLElement[] = [];
/** O aviso da pasta sem estilo.yaml, com o botão que cria o estilo. */
let avisoSemEstilo: HTMLElement | null = null;
let criandoEstilo = false;

const gravador = new Gravador({
  gravar: (nome, texto) =>
    invoke<boolean>("salvar_capitulo", { nome, texto }).then(() => registrar(`salvo: ${nome}`)),
  texto: () => editor.texto(),
  agendar: (tarefa, ms) => window.setTimeout(tarefa, ms),
  cancelar: (agendamento) => window.clearTimeout(agendamento as number),
  aoMudarEstado: mostrarEstadoDaGravacao,
});
const editor = criarEditor(folha, {
  aoMudar: () => {
    gravador.mudou();
    agendarContagem();
  },
  aoSalvar: () => void gravador.salvarAgora(),
  aoSelecionar: () => {
    mostrarSelecao();
    agendarOnde();
  },
  maquinaDeEscrever: () => maquinaLigada(),
  tipografia: () => tipografiaLigada(),
});

// ---- Contagem de palavras: lateral, barra do capítulo e barra de status ----

/** Contar a cada tecla seria trabalho à toa: conta-se quando a digitação
 *  dá uma folga. */
const ESPERA_PARA_CONTAR_MS = 150;
let contagemAgendada: number | undefined;

function agendarContagem(): void {
  window.clearTimeout(contagemAgendada);
  contagemAgendada = window.setTimeout(mostrarContagem, ESPERA_PARA_CONTAR_MS);
}

function botaoDoCapitulo(nome: string): HTMLButtonElement | undefined {
  return [...listaDeCapitulos.querySelectorAll("button")].find((botao) => botao.dataset.nome === nome);
}

/** Preenche o item da lateral com o que se sabe do capítulo. */
function desenharItem(botao: HTMLButtonElement, capitulo: CapituloNaLista): void {
  const [titulo, palavras, arquivo] = botao.children;
  if (!titulo || !palavras || !arquivo) return;
  titulo.textContent = nomeDeExibicao(capitulo);
  palavras.textContent = capitulo.palavras === null ? "—" : numero(capitulo.palavras);
  // O nome do arquivo só aparece embaixo quando em cima está o título.
  arquivo.textContent = capitulo.titulo === null ? "" : capitulo.nome;
  botao.title =
    capitulo.palavras === null
      ? `${capitulo.nome}: não foi possível ler este arquivo como capítulo. Abra-o para ver o motivo.`
      : `${capitulo.nome}: ${rotuloDePalavras(capitulo.palavras)}`;
}

function desenharLista(): void {
  listaDeCapitulos.replaceChildren(
    ...capitulos.map((capitulo) => {
      const item = document.createElement("li");
      const botao = document.createElement("button");
      botao.type = "button";
      botao.dataset.nome = capitulo.nome;
      for (const classe of ["titulo", "palavras", "arquivo"]) {
        const parte = document.createElement("span");
        parte.className = classe;
        botao.append(parte);
      }
      desenharItem(botao, capitulo);
      botao.addEventListener("click", () => void abrirCapitulo(capitulo.nome));
      item.append(botao);
      return item;
    }),
  );
}

/** Reconta o capítulo aberto pelo texto da tela e mostra em todo lugar. */
function mostrarContagem(): void {
  window.clearTimeout(contagemAgendada);
  if (capituloAtual !== null) capitulos = comTextoAoVivo(capitulos, capituloAtual, editor.texto());
  const aberto = capitulos.find((capitulo) => capitulo.nome === capituloAtual);
  const total = totalDePalavras(capitulos);
  resumoDoLivro.textContent =
    capitulos.length === 0
      ? ""
      : `${numero(capitulos.length)} ${capitulos.length === 1 ? "capítulo" : "capítulos"} · ${rotuloDePalavras(total)}`;
  palavrasDoLivro.textContent = capitulos.length === 0 ? "" : `Livro: ${rotuloDePalavras(total)}`;
  mostrarHoje(total);
  if (aberto === undefined) {
    nomeDoCapitulo.textContent = "";
    arquivoDoCapitulo.textContent = "";
    palavrasDoCapitulo.textContent = "";
    return;
  }
  nomeDoCapitulo.textContent = nomeDeExibicao(aberto);
  arquivoDoCapitulo.textContent = aberto.titulo === null ? "" : aberto.nome;
  palavrasDoCapitulo.textContent = `Capítulo: ${rotuloDePalavras(aberto.palavras)}`;
  const botao = botaoDoCapitulo(aberto.nome);
  if (botao) desenharItem(botao, aberto);
}

function mostrarSelecao(): void {
  const selecionado = editor.selecao();
  palavrasDaSelecao.textContent = selecionado === "" ? "" : `Seleção: ${rotuloDePalavras(contarPalavras(selecionado))}`;
}

// ---- Avisos e gravação ----

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
    for (const botao of listaDeCapitulos.querySelectorAll("button")) {
      botao.setAttribute("aria-current", String(botao.dataset.nome === nome));
    }
    mostrarContagem();
    mostrarSelecao();
    vazio.hidden = true;
    folha.hidden = false;
    editor.focar();
    registrar(`capítulo: ${nome}, ${editor.view.state.doc.lines} linhas`);
    botaoDeConverter.disabled = false;
    oferecerConversao();
    guardarOnde();
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

/** Os avisos de fonte que falta, trocados a cada vez que o estilo muda. */
let avisosDeFonte: HTMLElement[] = [];

function aplicarEstilo(livro: LivroAberto): void {
  cssDoLivro.replaceSync(livro.css ?? "");
  avisosDeFonte.forEach((aviso) => aviso.remove());
  avisosDeFonte = [];
  const [corpo] = livro.fontes;
  if (corpo === undefined) {
    document.documentElement.style.removeProperty("--md-novel-linha-simples");
    return;
  }
  document.documentElement.style.setProperty("--md-novel-linha-simples", String(linhaSimples(corpo)));
  for (const fonte of livro.fontes) {
    if (!fonteInstalada(fonte)) {
      avisosDeFonte.push(
        avisar(
          `A fonte "${fonte}" do estilo do livro não está instalada neste computador: a tela está usando outra, e o Word vai escolher a dele. Instale a fonte, ou escolha outra no painel "Estilo".`,
        ),
      );
    }
  }
}

// ---- A sessão de escrita: onde parou, e as palavras de hoje ----

// Como o tema, ficam no armazenamento da janela, fora da pasta do livro:
// um registro por livro, com o caminho da pasta (que veio do Rust) na chave.
/** A pasta do livro aberto, como o Rust a mostrou. */
let pastaDoLivro: string | null = null;
const chaveDeOnde = (pasta: string) => `md-novel.onde:${pasta}`;
const chaveDeHoje = (pasta: string) => `md-novel.hoje:${pasta}`;
const CHAVE_DA_META = "md-novel.meta";

function guardarOnde(): void {
  if (pastaDoLivro === null || capituloAtual === null) return;
  guardar(chaveDeOnde(pastaDoLivro), escreverOnde(capituloAtual, editor.posicao()));
}

/** Andar com o cursor não grava a cada tecla: grava quando ele sossega. */
const ESPERA_PARA_GUARDAR_ONDE_MS = 400;
let ondeAgendado: number | undefined;
function agendarOnde(): void {
  window.clearTimeout(ondeAgendado);
  ondeAgendado = window.setTimeout(guardarOnde, ESPERA_PARA_GUARDAR_ONDE_MS);
}

let meta = lerMeta(guardado(CHAVE_DA_META));
/** O dia da conta e o total de palavras do livro quando esse dia começou. */
let hoje: { dia: string; base: number } | null = null;

function mostrarHoje(total: number): void {
  if (pastaDoLivro === null || capitulos.length === 0) {
    botaoDeHoje.hidden = true;
    return;
  }
  const dia = diaDe(new Date());
  // Livro recém-aberto, ou virou a meia-noite com a janela aberta.
  if (hoje === null || hoje.dia !== dia) {
    hoje = { dia, base: lerHoje(guardado(chaveDeHoje(pastaDoLivro)), dia, total) };
    guardar(chaveDeHoje(pastaDoLivro), escreverHoje(hoje.dia, hoje.base));
  }
  botaoDeHoje.textContent = rotuloDeHoje(total - hoje.base, meta);
  botaoDeHoje.hidden = !campoDaMeta.hidden;
}

function fecharCampoDaMeta(): void {
  campoDaMeta.hidden = true;
  botaoDeHoje.hidden = pastaDoLivro === null || capitulos.length === 0;
}

botaoDeHoje.addEventListener("click", () => {
  campoDaMeta.value = meta === null ? "" : String(meta);
  campoDaMeta.hidden = false;
  botaoDeHoje.hidden = true;
  campoDaMeta.focus();
  campoDaMeta.select();
});
campoDaMeta.addEventListener("blur", fecharCampoDaMeta);
campoDaMeta.addEventListener("keydown", (evento) => {
  if (evento.key === "Escape") {
    fecharCampoDaMeta();
    editor.focar();
    return;
  }
  if (evento.key !== "Enter") return;
  const digitada = campoDaMeta.value.trim();
  const nova = lerMeta(digitada);
  // Campo vazio tira a meta; o que não é um número da faixa fica para corrigir.
  if (digitada !== "" && nova === null) {
    avisar(`A meta precisa ser um número de palavras de 1 a ${numero(META_MAXIMA)}; recebi "${digitada}".`, "erro");
    return;
  }
  meta = nova;
  guardar(CHAVE_DA_META, nova === null ? "" : String(nova));
  fecharCampoDaMeta();
  mostrarContagem();
  editor.focar();
});

// ---- O painel de estilo ----

// O autor muda o estilo num formulário, como no Word, e cada mudança aparece
// na hora no texto e vai para o estilo.yaml. Quem valida e grava é o Rust.
const painelDoEstilo = elemento("painel-do-estilo");
const botaoDoEstilo = elemento<HTMLButtonElement>("abrir-estilo");
const estiloEscolhidoNoPainel = elemento<HTMLSelectElement>("estilo-escolhido");
const botaoNovoEstilo = elemento<HTMLButtonElement>("novo-estilo");
const botaoApagarEstilo = elemento<HTMLButtonElement>("apagar-estilo");
const campoDoNomeDoEstilo = elemento<HTMLInputElement>("nome-do-estilo-novo");
const campoDaFonte = elemento<HTMLInputElement>("campo-fonte");
const campoDaEntrelinha = elemento<HTMLInputElement>("campo-entrelinha");
const campoDoAlinhamento = elemento("campo-alinhamento");
const campoDoItalico = elemento<HTMLInputElement>("campo-italico");
const campoDoNegrito = elemento<HTMLInputElement>("campo-negrito");
const erroDoEstilo = elemento("erro-do-estilo");
const estadoDoEstilo = elemento("estado-do-estilo");
const dicaDoEstilo = elemento("dica-do-estilo");

/** Os campos de medida: o do número e onde aparece a unidade. */
const MEDIDAS: { campo: "tamanho" | "recuo-primeira-linha" | "espaco-antes" | "espaco-depois"; numero: HTMLInputElement; unidade: HTMLElement }[] = [
  { campo: "tamanho", numero: elemento<HTMLInputElement>("campo-tamanho"), unidade: elemento("unidade-tamanho") },
  { campo: "recuo-primeira-linha", numero: elemento<HTMLInputElement>("campo-recuo"), unidade: elemento("unidade-recuo") },
  { campo: "espaco-antes", numero: elemento<HTMLInputElement>("campo-antes"), unidade: elemento("unidade-antes") },
  { campo: "espaco-depois", numero: elemento<HTMLInputElement>("campo-depois"), unidade: elemento("unidade-depois") },
];

/** O estilo como está no painel; `null` com o painel fechado. */
let estiloNoPainel: EstiloNoPainel | null = null;
/** O parágrafo que o painel mostra: `null` é o texto do livro. */
let paragrafoNoPainel: string | null = null;
const TEXTO_DO_LIVRO = "Texto do livro";

/** Mostra no formulário o parágrafo escolhido. */
function preencherPainel(): void {
  if (estiloNoPainel === null) return;
  // Em ordem de nome, como ficam no arquivo e na próxima abertura do painel.
  const nomes = Object.keys(estiloNoPainel.estilos).sort((um, outro) => um.localeCompare(outro, "pt"));
  estiloEscolhidoNoPainel.replaceChildren(
    new Option(TEXTO_DO_LIVRO, ""),
    ...nomes.map((nome) => new Option(nome, nome)),
  );
  estiloEscolhidoNoPainel.value = paragrafoNoPainel ?? "";
  botaoApagarEstilo.disabled = paragrafoNoPainel === null;
  const campos = camposDe(estiloNoPainel, paragrafoNoPainel);
  campoDaFonte.value = campos.fonte;
  for (const { campo, numero, unidade } of MEDIDAS) {
    const medida = partirMedida(campos[campo]);
    numero.value = medida === null ? "" : String(medida.numero);
    unidade.textContent = medida?.unidade ?? "";
  }
  campoDaEntrelinha.value = String(campos.entrelinha);
  for (const botao of campoDoAlinhamento.querySelectorAll("button")) {
    botao.setAttribute("aria-checked", String(botao.dataset.alinhamento === campos.alinhamento));
  }
  campoDoItalico.checked = campos.italico;
  campoDoNegrito.checked = campos.negrito;
  for (const marcado of painelDoEstilo.querySelectorAll<HTMLElement>("[data-campo]")) {
    const campo = marcado.dataset.campo as keyof Campos;
    marcado.classList.toggle("herdado", herdado(estiloNoPainel, paragrafoNoPainel, campo));
  }
  dicaDoEstilo.textContent =
    paragrafoNoPainel === null
      ? "Vale para todo o texto do livro, na tela e no .docx."
      : `Para usar num trecho, escreva antes dele ::: {custom-style="${paragrafoNoPainel}"} e, depois dele, ::: numa linha só. O que este estilo não muda, ele herda do texto do livro.`;
}

/** Uma gravação por vez, e só a última mudança espera a anterior acabar. */
const ESPERA_PARA_GRAVAR_O_ESTILO_MS = 350;
let gravacaoDoEstiloAgendada: number | undefined;
let gravacaoDoEstilo: Promise<void> = Promise.resolve();

async function gravarEstilo(estilo: EstiloNoPainel): Promise<void> {
  try {
    const livro = await invoke<LivroAberto>("salvar_estilo", { painel: estilo });
    aplicarEstilo(livro);
    editor.view.requestMeasure();
    if (estiloNoPainel !== null) estiloNoPainel = { ...estiloNoPainel, existe: true };
    avisoSemEstilo?.remove();
    avisoSemEstilo = null;
    erroDoEstilo.hidden = true;
    estadoDoEstilo.textContent = "Salvo no estilo.yaml do livro.";
    registrar("estilo salvo pelo painel");
  } catch (motivo) {
    // O que está no formulário fica: o autor corrige o campo e a gravação tenta de novo.
    erroDoEstilo.textContent = comoErro(motivo).mensagem;
    erroDoEstilo.hidden = false;
    estadoDoEstilo.textContent = "";
    registrar("estilo não salvo pelo painel");
  }
}

/** Grava já o que está agendado, com o estilo como está agora no painel. */
function gravarEstiloAgendado(): void {
  if (gravacaoDoEstiloAgendada === undefined) return;
  window.clearTimeout(gravacaoDoEstiloAgendada);
  gravacaoDoEstiloAgendada = undefined;
  const estilo = estiloNoPainel;
  if (estilo !== null) gravacaoDoEstilo = gravacaoDoEstilo.then(() => gravarEstilo(estilo));
}

function agendarGravacaoDoEstilo(): void {
  estadoDoEstilo.textContent = "Salvando…";
  window.clearTimeout(gravacaoDoEstiloAgendada);
  gravacaoDoEstiloAgendada = window.setTimeout(gravarEstiloAgendado, ESPERA_PARA_GRAVAR_O_ESTILO_MS);
}

function mudarCampoDoEstilo<K extends keyof Campos>(campo: K, valor: Campos[K]): void {
  if (estiloNoPainel === null) return;
  estiloNoPainel = comCampo(estiloNoPainel, paragrafoNoPainel, campo, valor);
  preencherPainel();
  agendarGravacaoDoEstilo();
}

async function abrirPainelDoEstilo(): Promise<void> {
  try {
    estiloNoPainel = await invoke<EstiloNoPainel>("estilo_para_editar");
  } catch (motivo) {
    avisar(comoErro(motivo).mensagem, "erro");
    return;
  }
  paragrafoNoPainel = null;
  erroDoEstilo.hidden = true;
  estadoDoEstilo.textContent = estiloNoPainel.existe
    ? ""
    : "Este livro ainda não tem estilo: estes são os valores do modelo. A primeira mudança cria o estilo.yaml do livro.";
  elemento("fontes-sugeridas").replaceChildren(
    ...FONTES_SUGERIDAS.filter(fonteInstalada).map((fonte) => new Option(fonte, fonte)),
  );
  painelDoEstilo.hidden = false;
  botaoDoEstilo.setAttribute("aria-pressed", "true");
  preencherPainel();
}

function fecharPainelDoEstilo(): void {
  // A mudança feita um instante antes de fechar não se perde.
  gravarEstiloAgendado();
  painelDoEstilo.hidden = true;
  botaoDoEstilo.setAttribute("aria-pressed", "false");
  campoDoNomeDoEstilo.hidden = true;
  estiloNoPainel = null;
}

botaoDoEstilo.addEventListener("click", () => {
  if (painelDoEstilo.hidden) void abrirPainelDoEstilo();
  else fecharPainelDoEstilo();
});
elemento("fechar-o-painel").addEventListener("click", fecharPainelDoEstilo);
estiloEscolhidoNoPainel.addEventListener("change", () => {
  paragrafoNoPainel = estiloEscolhidoNoPainel.value === "" ? null : estiloEscolhidoNoPainel.value;
  preencherPainel();
});
// A fonte só vale quando o autor termina de escrever o nome (Enter, sair do
// campo ou escolher da lista): gravar a cada letra criaria fontes que não existem.
campoDaFonte.addEventListener("change", () => {
  const fonte = campoDaFonte.value.trim();
  if (fonte !== "") mudarCampoDoEstilo("fonte", fonte);
});
for (const { campo, numero, unidade } of MEDIDAS) {
  numero.addEventListener("input", () => {
    // Campo vazio no meio da digitação: espera o número.
    if (Number.isNaN(numero.valueAsNumber) || numero.valueAsNumber < 0) return;
    mudarCampoDoEstilo(campo, juntarMedida(numero.valueAsNumber, unidade.textContent || (campo === "recuo-primeira-linha" ? "cm" : "pt")));
  });
}
campoDaEntrelinha.addEventListener("input", () => {
  if (Number.isNaN(campoDaEntrelinha.valueAsNumber) || campoDaEntrelinha.valueAsNumber <= 0) return;
  mudarCampoDoEstilo("entrelinha", campoDaEntrelinha.valueAsNumber);
});
campoDoAlinhamento.addEventListener("click", (evento) => {
  const botao = evento.target instanceof Element ? evento.target.closest("button") : null;
  const alinhamento = botao?.dataset.alinhamento;
  if (alinhamento !== undefined) mudarCampoDoEstilo("alinhamento", alinhamento as Alinhamento);
});
campoDoItalico.addEventListener("change", () => mudarCampoDoEstilo("italico", campoDoItalico.checked));
campoDoNegrito.addEventListener("change", () => mudarCampoDoEstilo("negrito", campoDoNegrito.checked));

botaoNovoEstilo.addEventListener("click", () => {
  campoDoNomeDoEstilo.value = "";
  campoDoNomeDoEstilo.hidden = false;
  campoDoNomeDoEstilo.focus();
});
campoDoNomeDoEstilo.addEventListener("keydown", (evento) => {
  if (evento.key === "Escape") {
    campoDoNomeDoEstilo.hidden = true;
    return;
  }
  if (evento.key !== "Enter" || estiloNoPainel === null) return;
  const nome = nomeDeEstiloNovo(estiloNoPainel, campoDoNomeDoEstilo.value);
  if (nome === null) {
    erroDoEstilo.textContent = `O nome do estilo novo não pode ser vazio nem repetir um que já existe; recebi "${campoDoNomeDoEstilo.value}".`;
    erroDoEstilo.hidden = false;
    return;
  }
  campoDoNomeDoEstilo.hidden = true;
  estiloNoPainel = comEstiloNovo(estiloNoPainel, nome);
  paragrafoNoPainel = nome;
  preencherPainel();
  agendarGravacaoDoEstilo();
});
botaoApagarEstilo.addEventListener("click", () => {
  if (estiloNoPainel === null || paragrafoNoPainel === null) return;
  estiloNoPainel = semEstilo(estiloNoPainel, paragrafoNoPainel);
  paragrafoNoPainel = null;
  preencherPainel();
  agendarGravacaoDoEstilo();
});

// ---- Capítulo novo ----

function fecharCampoDoCapituloNovo(): void {
  campoDoCapituloNovo.hidden = true;
  botaoNovoCapitulo.hidden = pastaDoLivro === null;
}

/** O Rust confere o nome e cria o arquivo vazio; aqui a lista ganha o
 *  capítulo e ele abre. O que está digitado no capítulo atual é salvo antes. */
let criandoCapitulo = false;
async function criarCapitulo(nome: string): Promise<void> {
  // Um segundo Enter antes da resposta daria "já existe" sem motivo.
  if (criandoCapitulo) return;
  criandoCapitulo = true;
  try {
    if (!(await gravador.salvarAgora())) {
      avisoDeGravacao ??= avisarDaGravacao(TEXTO_NAO_SALVO);
      return;
    }
    const criado = await invoke<{ nome: string; livro: LivroAberto }>("criar_capitulo", { nome });
    fecharCampoDoCapituloNovo();
    capitulos = criado.livro.capitulos;
    desenharLista();
    await abrirCapitulo(criado.nome);
    registrar(`capítulo criado: ${criado.nome}`);
  } catch (motivo) {
    avisar(comoErro(motivo).mensagem, "erro");
  } finally {
    criandoCapitulo = false;
  }
}

botaoNovoCapitulo.addEventListener("click", () => {
  campoDoCapituloNovo.value = sugestaoDeNome(capitulos.map((capitulo) => capitulo.nome));
  campoDoCapituloNovo.hidden = false;
  botaoNovoCapitulo.hidden = true;
  campoDoCapituloNovo.focus();
  const fim = campoDoCapituloNovo.value.length;
  campoDoCapituloNovo.setSelectionRange(fim, fim);
});
campoDoCapituloNovo.addEventListener("blur", fecharCampoDoCapituloNovo);
campoDoCapituloNovo.addEventListener("keydown", (evento) => {
  if (evento.key === "Escape") {
    fecharCampoDoCapituloNovo();
    editor.focar();
  } else if (evento.key === "Enter") {
    void criarCapitulo(campoDoCapituloNovo.value);
  }
});

// O corretor é do sistema: o Rust liga e diz se achou o dicionário. O aviso
// de dicionário ausente aparece uma vez por abertura do app, não a cada livro.
const estadoDoCorretor = invoke<EstadoDoCorretor>("estado_do_corretor").catch(
  (): EstadoDoCorretor => ({ linguas: [], aviso: null }),
);
let avisoDoCorretorMostrado = false;

const ROTULO_DE_CRIAR_O_ESTILO = "Criar estilo.yaml";

/** O botão do aviso da pasta sem estilo: o Rust grava o modelo na pasta do
 *  livro e devolve o CSS. O capítulo aberto não é relido: o que está
 *  digitado continua na tela e segue para o disco pelo caminho de sempre. */
async function criarEstilo(): Promise<void> {
  // Um segundo clique antes da resposta daria "já existe" sem motivo.
  if (criandoEstilo) return;
  criandoEstilo = true;
  try {
    const livro = await invoke<LivroAberto>("criar_estilo");
    avisoSemEstilo?.remove();
    avisoSemEstilo = null;
    aplicarEstilo(livro);
    // A altura das linhas mudou com o CSS novo.
    editor.view.requestMeasure();
    avisar(
      'Criei o estilo do livro a partir do modelo. Ajuste a fonte, o tamanho, a entrelinha e o recuo no painel "Estilo", ao lado: cada mudança aparece na hora no texto.',
      "feito",
    );
    registrar(`estilo criado: css=${livro.css !== null}`);
    await abrirPainelDoEstilo();
  } catch (motivo) {
    avisar(comoErro(motivo).mensagem, "erro");
  } finally {
    criandoEstilo = false;
  }
}

async function mostrarLivro(livro: LivroAberto): Promise<void> {
  capituloAtual = null;
  capitulos = livro.capitulos;
  pastaDoLivro = livro.pasta;
  hoje = null;
  fecharCampoDoCapituloNovo();
  // O painel é do livro que estava aberto.
  fecharPainelDoEstilo();
  nomeDoLivro.textContent = livro.nome;
  nomeDoLivro.title = livro.pasta;
  avisos.replaceChildren();
  avisoDeGravacao = null;
  avisosDaExportacao = [];
  avisoSemEstilo = null;
  botaoExportar.disabled = false;
  livro.avisos.forEach((aviso) => avisar(aviso));
  const corretor = await estadoDoCorretor;
  if (corretor.aviso !== null && !avisoDoCorretorMostrado) {
    avisoDoCorretorMostrado = true;
    avisar(corretor.aviso);
  }
  if (livro.aviso_sem_estilo !== null) {
    avisoSemEstilo = avisar(livro.aviso_sem_estilo, "", {
      rotulo: ROTULO_DE_CRIAR_O_ESTILO,
      fazer: () => void criarEstilo(),
    });
  }
  aplicarEstilo(livro);
  desenharLista();
  mostrarContagem();
  mostrarSelecao();
  registrar(`livro: ${livro.capitulos.length} capítulos, css=${livro.css !== null}, avisos=${avisos.childElementCount}`);
  const [primeiro] = livro.capitulos;
  if (primeiro === undefined) {
    folha.hidden = true;
    vazio.hidden = false;
    textoDoVazio.textContent = "Esta pasta ainda não tem capítulos (.md).";
    return;
  }
  // Reabre onde o autor parou neste livro, se o capítulo ainda existe.
  const onde = lerOnde(
    guardado(chaveDeOnde(livro.pasta)),
    livro.capitulos.map((capitulo) => capitulo.nome),
  );
  await abrirCapitulo(onde?.capitulo ?? primeiro.nome, true);
  if (onde !== null && capituloAtual === onde.capitulo) {
    editor.irPara(onde.posicao);
    // Abrir o capítulo guardou o começo dele: o lugar certo volta já, sem
    // esperar o cursor sossegar.
    guardarOnde();
  }
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
  // O cursor tem a altura da letra, não a da linha (entrelinha de 1,5). Mede-se
  // numa linha do corpo: o capítulo abre com o cursor no título, de letra
  // maior. Depois o cursor volta para onde estava.
  const primeira = editor.view.contentDOM.querySelector(
    ".cm-line:not([data-md-novel-estilo]):not(.md-novel-titulo):not(.md-novel-marcador):not(.md-novel-vazia)",
  );
  const ondeEstava = editor.posicao();
  if (primeira !== null) editor.view.dispatch({ selection: { anchor: editor.view.posAtDOM(primeira) + 1 } });
  await new Promise((resolve) => requestAnimationFrame(resolve));
  await new Promise((resolve) => requestAnimationFrame(resolve));
  const cursor = editor.view.dom.querySelector(".cm-cursor-primary");
  const alturaDoCursor = cursor === null ? null : Math.round(cursor.getBoundingClientRect().height);
  editor.view.dispatch({ selection: { anchor: ondeEstava } });
  registrar(
    `cursor: altura=${alturaDoCursor ?? "ausente"}px letra=${primeira === null ? "ausente" : getComputedStyle(primeira).fontSize} linha=${primeira === null ? "ausente" : getComputedStyle(primeira).lineHeight}`,
  );
  registrar(
    `conforto ao abrir: zoom=${rotuloDoZoom(zoom)} ${medidasDoZoom()} lateral=${getComputedStyle(elemento("lateral")).display} máquina=${maquinaLigada() ? "ligada" : "desligada"}`,
  );
  const corretor = await estadoDoCorretor;
  registrar(`corretor: línguas=${corretor.linguas.join(",")} aviso=${corretor.aviso === null ? "não" : "sim"}`);
  registrar(`onde: capítulo=${capituloAtual} posição=${editor.posicao()} hoje="${botaoDeHoje.textContent}"`);
  const roteiro = await invoke<RoteiroDeTeste | null>("roteiro_de_teste");
  if (roteiro) {
    // O tema do roteiro vale só para esta janela: não é guardado.
    if (roteiro.tema !== null) {
      preferenciaDeTema = lerPreferencia(roteiro.tema);
      aplicarTema();
    }
    // Já os cliques no botão são os do autor: a escolha fica guardada.
    for (let clique = 0; clique < roteiro.cliques_no_tema; clique += 1) botaoTema.click();
    registrarAparencia();
    registrar(`contagem antes: ${contagemNaTela()}`);
    editor.digitarNoFim(roteiro.digitar);
    registrar("digitado pelo roteiro de teste");
    if (roteiro.criar_estilo) {
      const botao = avisoSemEstilo?.querySelector("button:not(.fechar)");
      registrar(`botão do estilo: "${botao?.textContent ?? "ausente"}"`);
      // O clique de verdade, como o do autor; o aviso some quando o estilo chega.
      (botao as HTMLButtonElement | null | undefined)?.click();
      for (let espera = 0; espera < 100 && avisoSemEstilo !== null; espera += 1) {
        await new Promise((resolve) => window.setTimeout(resolve, 50));
      }
      await new Promise((resolve) => requestAnimationFrame(resolve));
      const classes = [...avisos.children].map((aviso) => aviso.className || "comum").join(",");
      registrar(
        `depois de criar o estilo: ${medir(".cm-line:not([data-md-novel-estilo]):not(.md-novel-titulo):not(.md-novel-marcador):not(.md-novel-vazia)")} avisos=${classes}`,
      );
    }
    if (roteiro.escrita) await roteiroDaEscrita();
    if (roteiro.sessao) await roteiroDaSessao();
    if (roteiro.capitulo_novo) await roteiroDoCapituloNovo();
    if (roteiro.corretor) await roteiroDoCorretor();
    if (roteiro.painel_de_estilo) await roteiroDoPainelDeEstilo();
    if (roteiro.paragrafos) await roteiroDosParagrafos();
    if (roteiro.conforto) await roteiroDoConforto();
    if (roteiro.exportar) await exportar();
    if (roteiro.fechar) {
      await invoke("teste_pedir_para_fechar");
      return;
    }
    // A contagem espera a digitação dar uma folga; o que se registra é o que
    // o autor vê depois dela.
    await new Promise((resolve) => window.setTimeout(resolve, ESPERA_PARA_CONTAR_MS * 3));
    registrar(`contagem depois: ${contagemNaTela()}`);
    // O fim do capítulo, onde o roteiro digitou, está desenhado agora: é lá
    // que o livro de teste tem um título colado num parágrafo.
    const tamanho = (seletor: string) => {
      const achado = editor.view.contentDOM.querySelector(seletor);
      return achado ? getComputedStyle(achado).fontSize : "ausente";
    };
    // O realce chega um instante depois do texto: o analisador do Markdown
    // trabalha em fatias, e o fim do capítulo é o último a ficar pronto.
    for (
      let vez = 0;
      vez < 40 && editor.view.contentDOM.querySelector(".cm-line:not(.md-novel-titulo) .md-novel-t1") === null;
      vez += 1
    ) {
      await new Promise((resolve) => window.setTimeout(resolve, 50));
    }
    registrar(`título colado: texto=${tamanho(".cm-line:not(.md-novel-titulo) .md-novel-t1")} corpo=${tamanho(".cm-line:not(.md-novel-titulo):not(.md-novel-marcador)")}`);
    if (roteiro.foco) definirFoco(true);
    await new Promise((resolve) => requestAnimationFrame(resolve));
    registrar(`foco: ${focoLigado() ? "ligado" : "desligado"}; ${["lateral", "barra", "status", "avisos", "sair-do-foco"].map((id) => `${id}=${getComputedStyle(elemento(id)).display}`).join(" ")}`);
    // De volta ao começo do capítulo, que é o que a captura do teste mostra.
    editor.view.scrollDOM.scrollTop = 0;
  }
}

/** Parágrafos como no Word e a conversão dos sinais: mede a linha em branco
 *  escondida, aperta Enter e Backspace como o autor e converte um trecho. */
async function roteiroDosParagrafos(): Promise<void> {
  const quadro = () => new Promise((resolve) => requestAnimationFrame(resolve));
  const tecla = (key: string, ctrlKey = false) =>
    editor.view.contentDOM.dispatchEvent(new KeyboardEvent("keydown", { key, ctrlKey, bubbles: true, cancelable: true }));
  const fim = (letras: number) => JSON.stringify(editor.texto().slice(-letras));
  // O capítulo do teste abre com uma fala à moda antiga: o app oferece trocar.
  const botaoDaOferta = ofertaDeConverter?.querySelector<HTMLButtonElement>("button:not(.fechar)");
  const oferta = ofertaDeConverter?.querySelector("span")?.textContent ?? "ausente";
  const antesDaOferta = editor.texto().includes('\\-- "Oi", disse.');
  botaoDaOferta?.click();
  await quadro();
  registrar(
    `oferta: "${oferta}" botão="${botaoDaOferta?.textContent ?? "ausente"}" antes=${antesDaOferta} depois=${editor.texto().includes("— “Oi”, disse.")} some=${ofertaDeConverter === null}`,
  );
  // A linha em branco entre dois parágrafos que estão na tela.
  editor.view.scrollDOM.scrollTop = 0;
  await quadro();
  await quadro();
  const linhas = [...editor.view.contentDOM.querySelectorAll<HTMLElement>(".cm-line")];
  const separadora = linhas.findIndex(
    (linha, indice) =>
      linha.classList.contains("md-novel-separador") &&
      indice > 0 &&
      indice + 1 < linhas.length &&
      !linhas[indice - 1]?.classList.contains("md-novel-titulo") &&
      !linhas[indice + 1]?.classList.contains("md-novel-marcador"),
  );
  const [cima, meio, baixo] = [linhas[separadora - 1], linhas[separadora], linhas[separadora + 1]].map((linha) =>
    linha?.getBoundingClientRect(),
  );
  registrar(
    `parágrafos: separadoras=${linhas.filter((linha) => linha.classList.contains("md-novel-separador")).length} altura=${meio?.height} vão=${cima && baixo ? Math.round(baixo.top - cima.bottom) : "ausente"}px`,
  );
  // Enter no fim do capítulo: parágrafo novo, com a linha em branco no arquivo.
  editor.irPara(editor.texto().length);
  editor.focar();
  tecla("Enter");
  for (const letra of "Novo.") {
    document.execCommand("insertText", false, letra);
    await new Promise((resolve) => window.setTimeout(resolve, 0));
  }
  await quadro();
  registrar(`parágrafos: enter fim=${fim(8)}`);
  // Backspace no começo do parágrafo novo: junta com o de cima.
  editor.irPara(editor.texto().length - "Novo.".length);
  tecla("Backspace");
  await quadro();
  registrar(`parágrafos: backspace fim=${fim(8)}`);
  // Um trecho como outros programas gravam, e o botão de converter.
  editor.digitarNoFim('\n\n\\-- "Oi", disse.');
  const antes = fim(16);
  botaoDeConverter.click();
  await quadro();
  const convertido = fim(14);
  tecla("z", true);
  await quadro();
  registrar(`converter: antes=${antes} depois=${convertido} desfeito=${fim(16)}`);
}

/** Muda o estilo pelo painel, como o autor faria com o mouse e o teclado, e
 *  registra o que o texto mostra depois de cada gravação. */
async function roteiroDoPainelDeEstilo(): Promise<void> {
  const espera = (ms: number) => new Promise((resolve) => window.setTimeout(resolve, ms));
  /** A gravação agendada e a em curso acabaram. */
  const gravado = async () => {
    await espera(ESPERA_PARA_GRAVAR_O_ESTILO_MS * 2);
    await gravacaoDoEstilo;
    await new Promise((resolve) => requestAnimationFrame(resolve));
  };
  const linhaDoCorpo = () => {
    const linha = editor.view.contentDOM.querySelector(
      ".cm-line:not([data-md-novel-estilo]):not(.md-novel-titulo):not(.md-novel-marcador):not(.md-novel-vazia)",
    );
    if (linha === null) return "ausente";
    const estilo = getComputedStyle(linha);
    return `tamanho=${estilo.fontSize} alinhamento=${estilo.textAlign}`;
  };
  const opcoes = () => [...estiloEscolhidoNoPainel.options].map((opcao) => opcao.textContent).join("|");
  const numeroDe = (campo: string) => MEDIDAS.find((medida) => medida.campo === campo)?.numero;
  const escolher = (nome: string) => {
    estiloEscolhidoNoPainel.value = nome;
    estiloEscolhidoNoPainel.dispatchEvent(new Event("change", { bubbles: true }));
  };
  editor.view.scrollDOM.scrollTop = 0;
  botaoDoEstilo.click();
  for (let vez = 0; vez < 100 && painelDoEstilo.hidden; vez += 1) await espera(50);
  const tamanho = numeroDe("tamanho");
  registrar(
    `painel: aberto=${!painelDoEstilo.hidden} fonte="${campoDaFonte.value}" tamanho=${tamanho?.value}${elemento("unidade-tamanho").textContent} opções=${opcoes()} antes[${linhaDoCorpo()}]`,
  );
  // O texto do livro: tamanho 14 e alinhado à esquerda.
  if (tamanho !== undefined) {
    tamanho.value = "14";
    tamanho.dispatchEvent(new Event("input", { bubbles: true }));
  }
  campoDoAlinhamento.querySelector<HTMLButtonElement>('[data-alinhamento="esquerda"]')?.click();
  await gravado();
  registrar(`painel: texto do livro depois[${linhaDoCorpo()}] estado="${estadoDoEstilo.textContent}"`);
  // Um estilo nomeado herda o tamanho novo, e mostra isso.
  escolher("Epígrafe");
  const marcaDoTamanho = painelDoEstilo.querySelector('[data-campo="tamanho"]');
  registrar(
    `painel: epígrafe tamanho=${tamanho?.value} herdado=${marcaDoTamanho?.classList.contains("herdado")} itálico=${campoDoItalico.checked}`,
  );
  // Estilo novo, em negrito; e um dos que existiam, apagado.
  botaoNovoEstilo.click();
  campoDoNomeDoEstilo.value = "Destaque";
  campoDoNomeDoEstilo.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
  campoDoNegrito.click();
  // Cada gravação termina antes do passo seguinte: senão a de um passo
  // levaria junto a mudança do outro, e o teste não saberia qual gravou.
  await gravado();
  escolher("Separador");
  botaoApagarEstilo.click();
  await gravado();
  // Um valor que o Word não guarda: o erro aparece no painel e nada é gravado.
  if (tamanho !== undefined) {
    tamanho.value = "5000";
    tamanho.dispatchEvent(new Event("input", { bubbles: true }));
  }
  await gravado();
  registrar(`painel: opções=${opcoes()} erro=${erroDoEstilo.hidden ? "nenhum" : erroDoEstilo.textContent}`);
}

/** Digita duas palavras erradas pelo caminho de entrada do navegador, que é
 *  o que o corretor do sistema acompanha. As pausas dão tempo de o teste
 *  fotografar a tela antes e depois: o sublinhado não aparece para a página,
 *  só para quem olha. */
async function roteiroDoCorretor(): Promise<void> {
  const espera = (ms: number) => new Promise((resolve) => window.setTimeout(resolve, ms));
  editor.focar();
  // A linha do cursor no meio da tela: no fim da janela o sublinhado ficaria
  // cortado, fora da fotografia. A última linha do capítulo só chega ao meio
  // com a folga da máquina de escrever, ligada aqui sem guardar a escolha.
  definirMaquina(true);
  editor.centrar();
  await espera(300);
  registrar("corretor: antes de digitar");
  await espera(2500);
  for (const letra of " Uma palavrra erradda aqui.") {
    document.execCommand("insertText", false, letra);
    await espera(0);
  }
  editor.centrar();
  await espera(1500);
  registrar("corretor: depois de digitar");
  await espera(2500);
}

/** Cria um capítulo pelo botão e pelo campo da lateral, e tenta de novo com
 *  o mesmo nome. */
async function roteiroDoCapituloNovo(): Promise<void> {
  const espera = (ms: number) => new Promise((resolve) => window.setTimeout(resolve, ms));
  const enter = () =>
    campoDoCapituloNovo.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
  botaoNovoCapitulo.click();
  const sugestao = campoDoCapituloNovo.value;
  campoDoCapituloNovo.value = `${sugestao}o-rio`;
  enter();
  const esperado = `${sugestao}o-rio.md`;
  for (let vez = 0; vez < 100 && capituloAtual !== esperado; vez += 1) await espera(50);
  registrar(
    `capítulo novo: sugestão="${sugestao}" atual=${capituloAtual} lista=${capitulos.map((capitulo) => capitulo.nome).join(",")} letras=${editor.texto().length} campo=${campoDoCapituloNovo.hidden ? "fechado" : "aberto"}`,
  );
  botaoNovoCapitulo.click();
  campoDoCapituloNovo.value = `${sugestao}o-rio`;
  enter();
  await espera(400);
  const erros = [...avisos.querySelectorAll("p.erro span")].map((aviso) => aviso.textContent).join(" | ");
  registrar(`capítulo repetido: ${erros} lista=${capitulos.length}`);
}

/** Define a meta pelo campo, troca de capítulo pelo clique na lateral e
 *  deixa o cursor num lugar conhecido, para a próxima abertura conferir. */
async function roteiroDaSessao(): Promise<void> {
  const espera = (ms: number) => new Promise((resolve) => window.setTimeout(resolve, ms));
  botaoDeHoje.click();
  campoDaMeta.value = "50";
  campoDaMeta.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
  await espera(ESPERA_PARA_CONTAR_MS * 3);
  registrar(`hoje: "${botaoDeHoje.textContent}" campo=${campoDaMeta.hidden ? "fechado" : "aberto"}`);
  const segundo = capitulos[1]?.nome;
  if (segundo !== undefined) botaoDoCapitulo(segundo)?.click();
  for (let vez = 0; vez < 100 && capituloAtual !== segundo; vez += 1) await espera(50);
  editor.irPara(7);
  await espera(ESPERA_PARA_GUARDAR_ONDE_MS * 2);
  registrar(`onde guardado: capítulo=${capituloAtual} posição=${editor.posicao()}`);
}

/** Digita pelo caminho de entrada do navegador (não pela API do editor),
 *  para as aspas e o travessão passarem por onde passam com o autor, e abre
 *  a busca com a tecla. */
async function roteiroDaEscrita(): Promise<void> {
  const quadro = () => new Promise((resolve) => requestAnimationFrame(resolve));
  // Uma tecla por vez, com uma folga entre elas: é assim que o editor as
  // recebe do autor. Coladas num bloco só, elas são texto colado, e texto
  // colado não é trocado.
  const digitar = async (texto: string) => {
    for (const letra of texto) {
      document.execCommand("insertText", false, letra);
      await new Promise((resolve) => window.setTimeout(resolve, 0));
    }
  };
  editor.focar();
  await digitar(" \"oi\" -- d'");
  await quadro();
  registrar(`tipografia: ligada fim=[${editor.texto().slice(-10)}]`);
  botaoDasAspas.click();
  await digitar(" \"x\" --");
  await quadro();
  registrar(`tipografia: desligada fim=[${editor.texto().slice(-7)}] apertado=${botaoDasAspas.getAttribute("aria-pressed")}`);
  botaoDasAspas.click();
  editor.view.contentDOM.dispatchEvent(
    new KeyboardEvent("keydown", { key: "f", ctrlKey: true, bubbles: true, cancelable: true }),
  );
  await quadro();
  const painel = editor.view.dom.querySelector(".cm-search");
  const botoes = [...(painel?.querySelectorAll("button") ?? [])].map((botao) => botao.textContent).join("|");
  registrar(
    `busca: painel=${painel !== null} campo="${painel?.querySelector("input")?.getAttribute("placeholder") ?? ""}" botões=${botoes}`,
  );
  editor.buscarProximo("neblina");
  registrar(`busca: achou="${editor.selecao()}"`);
  // Por último, porque deixa um bloco de código aberto no fim do capítulo:
  // dentro dele a tecla fica como foi digitada.
  editor.digitarNoFim("\n\n```\n");
  editor.focar();
  await digitar('"c" --');
  await quadro();
  registrar(`tipografia: em código fim=[${editor.texto().slice(-6)}]`);
}

/** O que o zoom muda (o texto e a folha) e o que não muda (a moldura). */
function medidasDoZoom(): string {
  const linha = editor.view.contentDOM.querySelector(
    ".cm-line:not([data-md-novel-estilo]):not(.md-novel-titulo):not(.md-novel-marcador):not(.md-novel-vazia)",
  );
  const texto = linha === null ? "ausente" : getComputedStyle(linha).fontSize;
  const folhaLarga = Math.round(editor.view.contentDOM.getBoundingClientRect().width);
  return `texto=${texto} folha=${folhaLarga}px moldura=${getComputedStyle(nomeDoLivro).fontSize} janela=${window.innerWidth}px`;
}

/** As teclas e a rolagem do autor, como eventos de verdade na janela, e o
 *  que a tela mostra depois de cada uma. */
async function roteiroDoConforto(): Promise<void> {
  const quadro = () => new Promise((resolve) => requestAnimationFrame(resolve));
  const tecla = (key: string, ctrlKey = false) =>
    window.dispatchEvent(new KeyboardEvent("keydown", { key, ctrlKey, bubbles: true, cancelable: true }));
  /** Distância entre o cursor e o meio da área que rola. */
  const desvioDoCursor = (): string => {
    const cursor = editor.view.coordsAtPos(editor.view.state.selection.main.head);
    const area = editor.view.scrollDOM.getBoundingClientRect();
    if (cursor === null) return "sem cursor";
    return `${Math.round(Math.abs((cursor.top + cursor.bottom) / 2 - (area.top + area.bottom) / 2))}px de ${Math.round(area.height)}px`;
  };
  const antesDoZoom = medidasDoZoom();
  tecla("=", true);
  tecla("=", true);
  // Mais velho que o intervalo da roda: o primeiro evento sempre vale.
  window.dispatchEvent(new WheelEvent("wheel", { deltaY: 120, ctrlKey: true, bubbles: true, cancelable: true }));
  await quadro();
  await quadro();
  registrar(`zoom: botão="${botaoDoZoom.textContent}" antes[${antesDoZoom}] depois[${medidasDoZoom()}]`);
  tecla("F9");
  registrar(`lateral: ${getComputedStyle(elemento("lateral")).display} apertado=${botaoDaLateral.getAttribute("aria-pressed")}`);
  await quadro();
  await quadro();
  const antes = desvioDoCursor();
  tecla("F10");
  await quadro();
  await quadro();
  await quadro();
  registrar(`máquina: ${maquinaLigada() ? "ligada" : "desligada"} antes=${antes} depois=${desvioDoCursor()}`);
  // Entra e sai: no display de teste não há gerenciador de janelas para
  // conferir o tamanho, só que o pedido é aceito.
  try {
    const entrou = await invoke<boolean>("alternar_tela_cheia");
    const saiu = await invoke<boolean>("alternar_tela_cheia");
    registrar(`tela cheia: pedido aceito (${entrou} e depois ${saiu})`);
  } catch (motivo) {
    registrar(`tela cheia: recusado: ${comoErro(motivo).mensagem}`);
  }
}

/** O que a interface faz sozinha no teste funcional (`RoteiroDeTeste` do Rust). */
interface RoteiroDeTeste {
  digitar: string;
  fechar: boolean;
  exportar: boolean;
  tema: string | null;
  cliques_no_tema: number;
  foco: boolean;
  criar_estilo: boolean;
  conforto: boolean;
  escrita: boolean;
  sessao: boolean;
  capitulo_novo: boolean;
  corretor: boolean;
  painel_de_estilo: boolean;
  paragrafos: boolean;
}

/** As contagens como estão escritas na tela. */
function contagemNaTela(): string {
  const item = capituloAtual === null ? undefined : botaoDoCapitulo(capituloAtual);
  return [
    `capítulo="${palavrasDoCapitulo.textContent}"`,
    `livro="${palavrasDoLivro.textContent}"`,
    `resumo="${resumoDoLivro.textContent}"`,
    `item="${item?.querySelector(".titulo")?.textContent} | ${item?.querySelector(".palavras")?.textContent} | ${item?.querySelector(".arquivo")?.textContent}"`,
  ].join(" ");
}

/** As cores e o realce que o navegador de fato aplicou. */
function registrarAparencia(): void {
  const conteudo = editor.view.contentDOM;
  const de = (seletor: string, propriedade: "fontStyle" | "fontWeight" | "fontSize" | "color") => {
    const achado = conteudo.querySelector(seletor);
    return achado ? getComputedStyle(achado)[propriedade] : "ausente";
  };
  registrar(
    `tema: ${document.documentElement.dataset.tema}; botão="${botaoTema.textContent}" mesa=${getComputedStyle(document.body).backgroundColor} papel=${getComputedStyle(conteudo).backgroundColor} tinta=${getComputedStyle(conteudo).color}`,
  );
  registrar(
    `realce: ênfase=${de(".md-novel-enfase:not(.md-novel-sinal)", "fontStyle")} forte=${de(".md-novel-forte:not(.md-novel-sinal)", "fontWeight")} título=${de(".md-novel-t1", "fontSize")} corpo=${de(".cm-line:not(.md-novel-titulo):not(.md-novel-marcador)", "fontSize")} sinal=${de(".md-novel-sinal", "color")}`,
  );
}

async function escolherPasta(): Promise<void> {
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
}

// ---- Modo foco ----

const focoLigado = () => document.documentElement.dataset.foco === "ligado";

/** No modo foco somem a lateral e as barras. Os avisos ficam: texto que não
 *  foi salvo não pode passar despercebido. */
function definirFoco(ligado: boolean): void {
  if (ligado) document.documentElement.dataset.foco = "ligado";
  else delete document.documentElement.dataset.foco;
  botaoSairDoFoco.hidden = !ligado;
  if (capituloAtual !== null) editor.focar();
}

// ---- Conforto: zoom, lateral, máquina de escrever e tela cheia ----

// Como o tema, estas escolhas ficam no armazenamento da janela, fora da
// pasta do livro.
const CHAVE_DO_ZOOM = "md-novel.zoom";
const CHAVE_DA_LATERAL_ESCONDIDA = "md-novel.lateral-escondida";
const CHAVE_DA_MAQUINA = "md-novel.maquina";

function guardado(chave: string): string | null {
  try {
    return window.localStorage.getItem(chave);
  } catch {
    return null;
  }
}

function guardar(chave: string, valor: string): void {
  try {
    window.localStorage.setItem(chave, valor);
  } catch {
    // Não deu para guardar: a escolha vale até a janela fechar.
  }
}

let zoom = lerZoom(guardado(CHAVE_DO_ZOOM));

/** O zoom é só do texto: o CSS do livro e a folha multiplicam as medidas por
 *  `--md-novel-zoom`. A lateral e as barras ficam do tamanho de sempre. */
function aplicarZoom(): void {
  botaoDoZoom.textContent = rotuloDoZoom(zoom);
  document.documentElement.style.setProperty("--md-novel-zoom", fatorDoZoom(zoom));
  // As linhas mudaram de altura.
  editor.view.requestMeasure();
}

function mudarZoom(acao: AcaoDeZoom): void {
  const novo = zoomDepoisDe(zoom, acao);
  if (novo === zoom) return;
  zoom = novo;
  guardar(CHAVE_DO_ZOOM, String(zoom));
  aplicarZoom();
}

const lateralEscondida = () => document.documentElement.dataset.lateral === "escondida";

function definirLateral(escondida: boolean): void {
  if (escondida) document.documentElement.dataset.lateral = "escondida";
  else delete document.documentElement.dataset.lateral;
  botaoDaLateral.setAttribute("aria-pressed", String(escondida));
}

const maquinaLigada = () => document.documentElement.dataset.maquina === "ligada";

function definirMaquina(ligada: boolean): void {
  if (ligada) document.documentElement.dataset.maquina = "ligada";
  else delete document.documentElement.dataset.maquina;
  botaoDaMaquina.setAttribute("aria-pressed", String(ligada));
}

// Aspas curvas e travessão ao digitar: ligado de fábrica, e o botão da
// barra de baixo desliga para quem prefere o sinal reto.
const CHAVE_DAS_ASPAS = "md-novel.aspas";
const tipografiaLigada = () => botaoDasAspas.getAttribute("aria-pressed") === "true";

function definirTipografia(ligada: boolean): void {
  botaoDasAspas.setAttribute("aria-pressed", String(ligada));
}

function fazerAtalho(atalho: Atalho): void {
  switch (atalho) {
    case "zoom-aumentar":
      mudarZoom("aumentar");
      break;
    case "zoom-diminuir":
      mudarZoom("diminuir");
      break;
    case "zoom-restaurar":
      mudarZoom("restaurar");
      break;
    case "lateral":
      definirLateral(!lateralEscondida());
      guardar(CHAVE_DA_LATERAL_ESCONDIDA, lateralEscondida() ? LIGADO : DESLIGADO);
      break;
    case "maquina":
      definirMaquina(!maquinaLigada());
      guardar(CHAVE_DA_MAQUINA, maquinaLigada() ? LIGADO : DESLIGADO);
      if (maquinaLigada()) editor.centrar();
      break;
    case "tela-cheia":
      void invoke("alternar_tela_cheia").catch((motivo: unknown) => avisar(comoErro(motivo).mensagem, "erro"));
      break;
  }
}

// Ao abrir: as escolhas guardadas valem antes de o livro aparecer.
definirLateral(lerLigado(guardado(CHAVE_DA_LATERAL_ESCONDIDA)));
definirMaquina(lerLigado(guardado(CHAVE_DA_MAQUINA)));
definirTipografia(!lerDesligado(guardado(CHAVE_DAS_ASPAS)));
aplicarZoom();

botaoDaLateral.addEventListener("click", () => fazerAtalho("lateral"));
botaoDaMaquina.addEventListener("click", () => fazerAtalho("maquina"));
botaoDoZoom.addEventListener("click", () => fazerAtalho("zoom-restaurar"));
// O que já estava escrito não é trocado ao digitar: o autor pede, para o
// capítulo aberto, e desfaz com Ctrl+Z se não gostar.
/** O aviso que oferece a conversão no capítulo aberto, e o que diz que ela foi feita. */
let ofertaDeConverter: HTMLElement | null = null;
let avisoDeConvertido: HTMLElement | null = null;

function converterSinais(): void {
  const trocas = editor.converterSinais();
  ofertaDeConverter?.remove();
  ofertaDeConverter = null;
  avisoDeConvertido?.remove();
  avisoDeConvertido = avisar(
    trocas === 0
      ? "Este capítulo não tem hifens duplos nem aspas retas para converter."
      : `Troquei ${numero(trocas)} ${trocas === 1 ? "sinal" : "sinais"} deste capítulo por travessão e aspas curvas. Ctrl+Z desfaz.`,
    "feito",
  );
  registrar(`convertido: ${trocas}`);
  editor.focar();
}

/** Ao abrir um capítulo que tem sinais à moda antiga, o app oferece trocar.
 *  Só oferece: o botão do fim da barra de baixo passa despercebido, e o app
 *  não muda o manuscrito sem o autor pedir. */
function oferecerConversao(): void {
  ofertaDeConverter?.remove();
  avisoDeConvertido?.remove();
  avisoDeConvertido = null;
  const oferta = ofertaDeConversao(editor.sinaisAConverter());
  ofertaDeConverter = oferta === null ? null : avisar(oferta, "", { rotulo: "Converter", fazer: converterSinais });
}

botaoDeConverter.addEventListener("click", converterSinais);
botaoDasAspas.addEventListener("click", () => {
  definirTipografia(!tipografiaLigada());
  guardar(CHAVE_DAS_ASPAS, tipografiaLigada() ? LIGADO : DESLIGADO);
  editor.focar();
});

/** A tecla que acompanha os atalhos: Cmd no Mac, Ctrl nos outros. No Linux a
 *  tecla Super é do gerenciador de janelas (no Hyprland, Super+rolagem troca
 *  de área de trabalho), e com Alt a tecla é um caractere do teclado. */
const NO_MAC = /mac/i.test(navigator.platform);
const comModificador = (evento: KeyboardEvent | WheelEvent): boolean =>
  (NO_MAC ? evento.metaKey : evento.ctrlKey) && !evento.altKey;

/** A rolagem do touchpad manda dezenas de eventos por gesto: um passo de
 *  zoom a cada intervalo, não um por evento. */
const INTERVALO_DA_RODA_MS = 80;
let ultimaRoda = 0;
window.addEventListener(
  "wheel",
  (evento) => {
    const acao = acaoDaRoda(evento.deltaY, comModificador(evento));
    if (acao === null) return;
    // Com Ctrl a rolagem é do zoom: o texto não rola junto.
    evento.preventDefault();
    if (evento.timeStamp - ultimaRoda < INTERVALO_DA_RODA_MS) return;
    ultimaRoda = evento.timeStamp;
    mudarZoom(acao);
  },
  { passive: false, capture: true },
);

elemento("foco").addEventListener("click", () => definirFoco(true));
botaoSairDoFoco.addEventListener("click", () => definirFoco(false));
// Na fase de captura: a tecla chega aqui antes do editor.
window.addEventListener(
  "keydown",
  (evento) => {
    // Com o cursor no painel de busca, o Esc fecha o painel (é do editor),
    // não sai do modo foco.
    const noPainelDaBusca = evento.target instanceof Element && evento.target.closest(".cm-panels") !== null;
    const acao =
      noPainelDaBusca && evento.key === "Escape" ? null : teclaDoFoco(evento.key, focoLigado(), evento.isComposing);
    if (acao !== null) {
      evento.preventDefault();
      evento.stopPropagation();
      definirFoco(acao === "alternar" ? !focoLigado() : false);
      return;
    }
    const atalho = atalhoDaTecla(evento.key, comModificador(evento), evento.isComposing);
    if (atalho === null) return;
    evento.preventDefault();
    evento.stopPropagation();
    fazerAtalho(atalho);
  },
  true,
);

elemento("abrir-pasta").addEventListener("click", () => void escolherPasta());
elemento("abrir-pasta-do-vazio").addEventListener("click", () => void escolherPasta());
botaoSalvar.addEventListener("click", () => void gravador.salvarAgora());
botaoExportar.addEventListener("click", () => void exportar());
// Sair da janela salva: o autor troca de programa sem pensar nisso.
window.addEventListener("blur", () => void gravador.salvarAgora());

// Fechar a janela: o Rust segura o fechamento e avisa aqui. Salva-se o que
// falta e só então a janela fecha. Se não deu para salvar, ela fica aberta
// com o aviso; pedir para fechar de novo fecha assim mesmo (o Rust deixa).
void listen("pedido-de-fechar", async () => {
  guardarOnde();
  if (await gravador.salvarAgora()) {
    await invoke("fechar_janela");
    return;
  }
  avisar("Há texto não salvo neste capítulo. Feche de novo para sair sem salvar.", "erro");
});

invoke<LivroAberto | null>("livro_atual")
  .then((livro) => (livro ? mostrarLivro(livro) : undefined))
  .catch((motivo: unknown) => avisar(comoErro(motivo).mensagem, "erro"));
