// Aspas curvas e travessão enquanto se digita, como no novelWriter. Aqui fica
// a regra, pura: o que entra no lugar da tecla, dado o que já está na linha.
// Quem liga no editor (e deixa de fora os blocos de código) é o `editor.ts`.

import { marcarLinhas } from "./blocos.ts";

export interface Troca {
  /** Quantos caracteres antes do cursor saem junto. */
  apagar: number;
  /** O que entra no lugar deles e da tecla digitada. */
  inserir: string;
}

/** Depois destes, uma aspa abre; depois de qualquer outra coisa, fecha. */
const ANTES_DE_ABRIR = /[\s([{—–“‘-]$/u;

/**
 * O que digitar `digitado` vira, com `antes` sendo o texto da linha até o
 * cursor. `null` deixa a tecla como veio.
 *
 * Nada é trocado onde o Markdown precisa do sinal reto: na linha de um bloco
 * `:::` (o `custom-style="..."` leva aspas retas), dentro de código em linha
 * e numa linha de tabela.
 */
export function substituicao(antes: string, digitado: string): Troca | null {
  if (digitado !== '"' && digitado !== "'" && digitado !== "-") return null;
  if (antes.trimStart().startsWith(":::")) return null;
  if (antes.includes("|")) return null;
  const crases = antes.split("`").length - 1;
  if (crases % 2 === 1) return null;

  if (digitado === "-") {
    // A linha só com o travessão que acabou de nascer de `--`: o terceiro
    // hífen é a quebra de cena (`---`), que o Pandoc precisa ler como tal.
    if (antes === "—") return { apagar: 1, inserir: "---" };
    // O começo de um comentário HTML (`<!--`) leva os dois hifens.
    if (antes.endsWith("<!-")) return null;
    if (antes.endsWith("-") && !antes.endsWith("--")) return { apagar: 1, inserir: "—" };
    return null;
  }
  // O sinal de ênfase colado na aspa (`*"fala"*`) não conta: vale o que vem
  // antes dele.
  const semEnfase = antes.replace(/[*_]+$/u, "");
  const abre = semEnfase === "" || ANTES_DE_ABRIR.test(semEnfase);
  if (digitado === '"') return { apagar: 0, inserir: abre ? "“" : "”" };
  return { apagar: 0, inserir: abre ? "‘" : "’" };
}

/** Uma troca no texto: o trecho de `de` a `ate` vira `inserir`. */
export interface Mudanca {
  de: number;
  ate: number;
  inserir: string;
}

const SO_HIFENS = /^\s*-{3,}\s*$/;
const CODIGO_RECUADO = /^(?: {4}|\t)/;

/**
 * O que trocar num texto já escrito, de uma vez: `\\--`, `--` e `---` viram
 * travessão, e as aspas retas viram curvas, pela mesma regra de quem digita.
 * É o comando "Converter" do app, que o autor pede para o capítulo aberto.
 *
 * Fica como está o que o Markdown precisa reto: bloco de código cercado ou
 * recuado, código em linha, linha de bloco `:::`, linha de tabela, a linha
 * só de hifens (quebra de cena), o cabeçalho YAML do começo, comentário HTML, o endereço de um link e o
 * sinal escapado com barra.
 */
export function converter(texto: string): { texto: string; mudancas: Mudanca[] } {
  const linhas = texto.split("\n");
  const marcas = marcarLinhas(linhas);
  const mudancas: Mudanca[] = [];
  let emComentario = false;
  // O cabeçalho YAML do Pandoc: da primeira linha `---` até a que o fecha.
  let emCabecalho = linhas[0]?.trim() === "---";
  let inicio = 0;
  linhas.forEach((linha, indice) => {
    const base = inicio;
    inicio += linha.length + 1;
    const noCabecalho = emCabecalho;
    if (emCabecalho && indice > 0 && /^(?:---|\.\.\.)\s*$/.test(linha)) emCabecalho = false;
    const intocada =
      noCabecalho ||
      marcas[indice]?.codigo === true ||
      linha.trimStart().startsWith(":::") ||
      linha.includes("|") ||
      CODIGO_RECUADO.test(linha) ||
      SO_HIFENS.test(linha);
    // O que já foi lido da linha, como fica depois das trocas: é por ele que
    // a aspa sabe se abre ou fecha.
    let lido = "";
    let i = 0;
    const copiarAte = (fim: number) => {
      lido += linha.slice(i, fim);
      i = fim;
    };
    const trocar = (ate: number, inserir: string) => {
      mudancas.push({ de: base + i, ate: base + ate, inserir });
      lido += inserir;
      i = ate;
    };
    while (i < linha.length) {
      if (emComentario) {
        const fim = linha.indexOf("-->", i);
        emComentario = fim === -1;
        copiarAte(fim === -1 ? linha.length : fim + 3);
        continue;
      }
      if (intocada) {
        copiarAte(linha.length);
        continue;
      }
      const letra = linha[i] ?? "";
      if (linha.startsWith("<!--", i)) {
        emComentario = true;
        copiarAte(i + 4);
      } else if (letra === "`") {
        // Código em linha: até a mesma quantidade de crases; sem fecho, o
        // resto da linha fica como está.
        const crases = /^`+/.exec(linha.slice(i))?.[0] ?? "`";
        const fecho = linha.indexOf(crases, i + crases.length);
        copiarAte(fecho === -1 ? linha.length : fecho + crases.length);
      } else if (letra === "]" && linha[i + 1] === "(") {
        // O endereço de um link, com o título entre aspas retas.
        const fecho = linha.indexOf(")", i);
        copiarAte(fecho === -1 ? linha.length : fecho + 1);
      } else if (letra === "\\") {
        // `\--` é como outros programas guardam os dois hifens da fala.
        if (linha.startsWith("\\--", i) && linha[i + 3] !== "-") trocar(i + 3, "—");
        else copiarAte(i + 2);
      } else if (letra === "-") {
        const hifens = /^-+/.exec(linha.slice(i))?.[0].length ?? 1;
        if (hifens === 2 || hifens === 3) trocar(i + hifens, "—");
        else copiarAte(i + hifens);
      } else if (letra === '"' || letra === "'") {
        const troca = substituicao(lido, letra);
        if (troca === null) copiarAte(i + 1);
        else trocar(i + 1, troca.inserir);
      } else {
        copiarAte(i + 1);
      }
    }
  });
  let convertido = texto;
  for (const mudanca of [...mudancas].reverse()) {
    convertido = convertido.slice(0, mudanca.de) + mudanca.inserir + convertido.slice(mudanca.ate);
  }
  return { texto: convertido, mudancas };
}

/** O que o app diz ao abrir um capítulo que tem sinais para converter;
 *  `null` se não tem nenhum. É só a oferta: quem converte é o autor, no botão. */
export function ofertaDeConversao(trocas: number): string | null {
  if (trocas === 0) return null;
  const quantos = String(trocas).replace(/\B(?=(\d{3})+$)/g, ".");
  return trocas === 1
    ? "Este capítulo tem 1 sinal escrito à moda antiga: hífen duplo no lugar do travessão, ou aspa reta. Quer trocar?"
    : `Este capítulo tem ${quantos} sinais escritos à moda antiga: hifens duplos no lugar do travessão, ou aspas retas. Quer trocar?`;
}
