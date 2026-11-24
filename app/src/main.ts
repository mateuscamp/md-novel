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
      `Criei o estilo.yaml em ${livro.pasta}. Para trocar a fonte, o tamanho, a entrelinha ou o recuo, abra esse arquivo num editor de texto: cada linha tem um comentário dizendo o que faz. Depois de mexer nele, feche e abra o md Novel para ver a mudança.`,
      "feito",
    );
    registrar(`estilo criado: css=${livro.css !== null}`);
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
  await zoomEmCurso;
  registrar(
    `conforto ao abrir: zoom=${rotuloDoZoom(zoom)} largura=${window.innerWidth} lateral=${getComputedStyle(elemento("lateral")).display} máquina=${maquinaLigada() ? "ligada" : "desligada"}`,
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
    registrar(`título colado: texto=${tamanho(".cm-line:not(.md-novel-titulo) .md-novel-t1")} corpo=${tamanho(".cm-line:not(.md-novel-titulo):not(.md-novel-marcador)")}`);
    if (roteiro.foco) definirFoco(true);
    await new Promise((resolve) => requestAnimationFrame(resolve));
    registrar(`foco: ${focoLigado() ? "ligado" : "desligado"}; ${["lateral", "barra", "status", "avisos", "sair-do-foco"].map((id) => `${id}=${getComputedStyle(elemento(id)).display}`).join(" ")}`);
    // De volta ao começo do capítulo, que é o que a captura do teste mostra.
    editor.view.scrollDOM.scrollTop = 0;
  }
}

/** Digita duas palavras erradas pelo caminho de entrada do navegador, que é
 *  o que o corretor do sistema acompanha. As pausas dão tempo de o teste
 *  fotografar a tela antes e depois: o sublinhado não aparece para a página,
 *  só para quem olha. */
async function roteiroDoCorretor(): Promise<void> {
  const espera = (ms: number) => new Promise((resolve) => window.setTimeout(resolve, ms));
  editor.focar();
  // A linha do cursor no meio da tela: no fim da janela o sublinhado ficaria
  // cortado, fora da fotografia.
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
  const larguraAntes = window.innerWidth;
  tecla("=", true);
  tecla("=", true);
  // Mais velho que o intervalo da roda: o primeiro evento sempre vale.
  window.dispatchEvent(new WheelEvent("wheel", { deltaY: 120, ctrlKey: true, bubbles: true, cancelable: true }));
  await zoomEmCurso;
  await quadro();
  registrar(`zoom: botão="${botaoDoZoom.textContent}" largura ${larguraAntes} -> ${window.innerWidth}`);
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
/** O pedido de zoom em curso, para o teste funcional esperar por ele. */
let zoomEmCurso: Promise<void> = Promise.resolve();

/** O zoom é da janela inteira, como o de um navegador: quem aplica é o Rust. */
function aplicarZoom(): void {
  botaoDoZoom.textContent = rotuloDoZoom(zoom);
  zoomEmCurso = invoke("definir_zoom", { porCento: zoom }).then(
    () => undefined,
    (motivo: unknown) => void avisar(comoErro(motivo).mensagem, "erro"),
  );
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
