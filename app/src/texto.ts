// O que se lê do texto de um capítulo sem interpretar o Markdown inteiro:
// quantas palavras tem e qual é o título. É a mesma regra de
// `crates/workspace/src/texto.rs`, repetida aqui para contar ao vivo enquanto
// o autor digita. Os casos de `crates/workspace/tests/fixtures` são lidos
// pelos testes dos dois lados e mantêm as duas implementações iguais.

// O Rust separa por `White_Space` do Unicode e chama de letra o que é
// `Alphabetic` ou número. O `\s` e o `trim` do JavaScript são outra lista
// (têm U+FEFF e não têm U+0085), então as classes vão escritas por extenso.
const ESPACOS = /\p{White_Space}+/u;
const LETRA_OU_NUMERO = /[\p{Alphabetic}\p{N}]/u;
const MARCADOR_DE_BLOCO = /^\p{White_Space}*:::/u;
const BORDAS = /^\p{White_Space}+|\p{White_Space}+$/gu;

const CERCA_DE_CODIGO = /^ {0,3}(`{3,}|~{3,})/;
/** Uma linha `:::` do Pandoc, com o que vier depois dos dois-pontos. */
const MARCADOR = /^:{3,}(.*)$/su;
/** Um ou mais `#` na primeira coluna, e o resto da linha. */
const TITULO = /^#+(.*)$/su;
const ESPACO_NO_COMECO = /^\p{White_Space}/u;

/** As linhas como o `str::lines` do Rust: sem o `\r` do fim. */
function linhasDe(texto: string): string[] {
  return texto.split("\n").map((linha) => (linha.endsWith("\r") ? linha.slice(0, -1) : linha));
}

const aparar = (texto: string) => texto.replace(BORDAS, "");

/**
 * Conta as palavras de um capítulo.
 *
 * Palavra é todo trecho entre espaços que tenha ao menos uma letra ou um
 * número: `—`, `#` e `* * *` não contam, `d'água` e `texto[^1]` contam uma
 * vez. As linhas que abrem e fecham um bloco `:::` são marcação, não texto.
 */
export function contarPalavras(texto: string): number {
  let palavras = 0;
  for (const linha of linhasDe(texto)) {
    if (MARCADOR_DE_BLOCO.test(linha)) continue;
    for (const trecho of linha.split(ESPACOS)) {
      if (LETRA_OU_NUMERO.test(trecho)) palavras += 1;
    }
  }
  return palavras;
}

/**
 * O título do capítulo: o texto do primeiro título Markdown (`# ...`) que
 * tenha texto. `null` se o capítulo não tem.
 *
 * Título aqui é o que o Pandoc lê como título, porque é ele quem escreve o
 * .docx: fora de bloco de código, na primeira coluna, e nunca colado num
 * parágrafo. Sem linha em branco antes, o `# ...` é texto do parágrafo de cima.
 */
export function tituloDoTexto(texto: string): string | null {
  /** A cerca (crases ou til) que abriu o bloco de código em que estamos. */
  let cerca: string | null = null;
  /** Quantos blocos `:::` estão abertos. */
  let blocosAbertos = 0;
  /** A linha de cima é texto de parágrafo. */
  let emParagrafo = false;
  for (const linha of linhasDe(texto)) {
    const marca = CERCA_DE_CODIGO.exec(linha)?.[1];
    if (cerca !== null) {
      // Fecha com o mesmo caractere, no mínimo do mesmo tamanho, e mais nada.
      const fecha =
        marca !== undefined &&
        marca[0] === cerca[0] &&
        marca.length >= cerca.length &&
        aparar(linha) === marca;
      if (fecha) cerca = null;
      continue;
    }
    if (marca !== undefined) {
      cerca = marca;
      emParagrafo = false;
      continue;
    }
    const depoisDosDoisPontos = MARCADOR.exec(linha)?.[1];
    if (depoisDosDoisPontos !== undefined) {
      const abre = aparar(depoisDosDoisPontos) !== "";
      // Um `:::` sem bloco aberto é texto comum, e segue adiante.
      if (abre || blocosAbertos > 0) {
        blocosAbertos += abre ? 1 : -1;
        emParagrafo = false;
        continue;
      }
    }
    const titulo = tituloDaLinha(linha);
    if (titulo !== null && !emParagrafo) {
      if (titulo !== "") return titulo;
      continue;
    }
    emParagrafo = aparar(linha) !== "";
  }
  return null;
}

/**
 * O texto de uma linha de título: um ou mais `#` na primeira coluna,
 * seguidos de espaço ou do fim da linha. Vazio para um título sem texto
 * (`#` sozinho), que para o Pandoc também é título. `null` se não é título.
 */
export function tituloDaLinha(linha: string): string | null {
  const resto = TITULO.exec(linha)?.[1];
  if (resto === undefined || !(resto === "" || ESPACO_NO_COMECO.test(resto))) return null;
  // O Pandoc tira as cerquilhas do fim, coladas no texto ou não: `# Um #` e
  // `# Um#` são "Um". Os asteriscos de ênfase não fazem parte do nome.
  return aparar(aparar(resto).replace(/#+$/, "").replaceAll("*", ""));
}
