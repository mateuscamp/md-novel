import assert from "node:assert/strict";
import { test } from "node:test";
import { converter, substituicao } from "./tipografia.ts";

/** O que fica escrito depois de digitar `tecla` com `antes` na linha. */
function digitar(antes: string, tecla: string): string {
  const troca = substituicao(antes, tecla);
  if (troca === null) return antes + tecla;
  return antes.slice(0, antes.length - troca.apagar) + troca.inserir;
}

test("aspas duplas abrem depois de espaço ou de abertura e fecham depois de texto", () => {
  assert.equal(digitar("", '"'), "“");
  assert.equal(digitar("Ela disse ", '"'), "Ela disse “");
  assert.equal(digitar("Ela disse “vem", '"'), "Ela disse “vem”");
  assert.equal(digitar("Ela disse “vem.", '"'), "Ela disse “vem.”");
  assert.equal(digitar("(", '"'), "(“");
  assert.equal(digitar("— ", '"'), "— “");
  assert.equal(digitar("—", '"'), "—“");
});

test("o sinal de ênfase colado na aspa não muda se ela abre ou fecha", () => {
  // `*"fala"*`: o asterisco que abre a ênfase vem antes da aspa que abre.
  assert.equal(digitar("*", '"'), "*“");
  assert.equal(digitar("disse **", '"'), "disse **“");
  assert.equal(digitar("disse _", "'"), "disse _‘");
  // `"*fala*"`: o asterisco que fecha a ênfase vem antes da aspa que fecha.
  assert.equal(digitar("“*vem*", '"'), "“*vem*”");
});

test("aspas simples: apóstrofo depois de letra, abertura depois de espaço", () => {
  assert.equal(digitar("copo d", "'"), "copo d’");
  assert.equal(digitar("disse ", "'"), "disse ‘");
  assert.equal(digitar("disse ‘sim", "'"), "disse ‘sim’");
  assert.equal(digitar("“", "'"), "“‘");
});

test("dois hifens viram travessão, e o terceiro numa linha só de travessão devolve a quebra de cena", () => {
  assert.equal(digitar("-", "-"), "—");
  assert.equal(digitar("Ele parou -", "-"), "Ele parou —");
  // `---` sozinho na linha é a quebra de cena do Markdown: volta a ser hifens.
  assert.equal(digitar("—", "-"), "---");
  assert.equal(digitar("texto —", "-"), "texto —-");
  // Um hífen só é hífen: lista, palavra composta.
  assert.equal(digitar("", "-"), "-");
  assert.equal(digitar("guarda", "-"), "guarda-");
});

test("onde o Markdown precisa do sinal reto, nada é trocado", () => {
  // A linha do bloco `:::` leva aspas retas: `custom-style="Epígrafe"`.
  assert.equal(substituicao("::: {custom-style=", '"'), null);
  assert.equal(substituicao("  :::: {.a custom-style=", "'"), null);
  // Dentro de código em linha.
  assert.equal(substituicao("use `printf(", '"'), null);
  assert.equal(substituicao("use `a-", "-"), null);
  // Fechado o código, volta a valer.
  assert.equal(digitar("use `x` e ", '"'), "use `x` e “");
  // O começo de um comentário HTML (`<!--`) leva os dois hifens.
  assert.equal(substituicao("<!-", "-"), null);
  assert.equal(substituicao("texto <!-", "-"), null);
  // Linha de tabela: os hifens são da tabela.
  assert.equal(substituicao("| a | b |\n|-".split("\n")[1] ?? "", "-"), null);
});

test("o que não é aspa nem hífen, ou vem colado em bloco, fica como foi digitado", () => {
  assert.equal(substituicao("texto", "a"), null);
  assert.equal(substituicao("texto", '""'), null);
  assert.equal(substituicao("texto", ""), null);
});

// O que já estava escrito, ou foi colado, não é trocado ao digitar. O
// comando "Converter" troca de uma vez, no capítulo aberto, a pedido do autor.
const convertido = (texto: string) => converter(texto).texto;

test("converter troca os dois hifens por travessão, com ou sem a barra de escape", () => {
  assert.equal(convertido("\\-- Eu vou sair."), "— Eu vou sair.");
  assert.equal(convertido("-- Só preciso."), "— Só preciso.");
  assert.equal(convertido("Ele parou -- e voltou."), "Ele parou — e voltou.");
  // Três hifens no meio da linha são o travessão do Pandoc.
  assert.equal(convertido("Ele parou --- e voltou."), "Ele parou — e voltou.");
  // Um hífen só, e quatro ou mais, ficam.
  assert.equal(convertido("guarda-chuva e - item ---- fim"), "guarda-chuva e - item ---- fim");
});

test("converter troca aspas retas por curvas, abrindo e fechando", () => {
  assert.equal(convertido("Ela disse \"vem\" e 'vai'."), "Ela disse “vem” e ‘vai’.");
  assert.equal(convertido("copo d'água"), "copo d’água");
  assert.equal(convertido("*\"Entra\"*, disse."), "*“Entra”*, disse.");
});

test("converter não mexe onde o Markdown precisa do sinal reto", () => {
  const intocados = [
    "---",
    "  ---  ",
    '::: {custom-style="Epígrafe"}',
    "| a -- b | \"c\" |",
    "use `printf(\"%d\") -- x` aqui",
    "    recuado -- \"código\"",
    "<!-- nota -- \"rever\" -->",
    '[site](http://a.com/x--y "título")',
    'aspa escapada: \\"assim\\"',
  ];
  for (const linha of intocados) assert.equal(convertido(linha), linha, linha);
  const comCerca = "```\n-- \"x\"\n```\n-- fora";
  assert.equal(convertido(comCerca), "```\n-- \"x\"\n```\n— fora");
  // Depois do código em linha e do comentário, o resto da linha vale.
  assert.equal(convertido("`a--b` -- c"), "`a--b` — c");
  assert.equal(convertido("<!-- a -- b --> \"c\""), "<!-- a -- b --> “c”");
  // O cabeçalho YAML do começo do arquivo, se houver, é do Pandoc.
  const comCabecalho = '---\ntitle: "O livro"\nnota: a -- b\n---\n\n-- "Oi."';
  assert.equal(convertido(comCabecalho), '---\ntitle: "O livro"\nnota: a -- b\n---\n\n— “Oi.”');
  // Comentário de várias linhas.
  assert.equal(convertido("<!-- a\n-- \"b\"\n--> -- c"), "<!-- a\n-- \"b\"\n--> — c");
});

test("converter diz o que trocou, e o texto já convertido não muda de novo", () => {
  const original = "\\-- \"Oi\", disse.\n\n-- Tchau.";
  const { texto, mudancas } = converter(original);
  assert.equal(texto, "— “Oi”, disse.\n\n— Tchau.");
  assert.equal(mudancas.length, 4);
  // As mudanças são posições no texto original, em ordem: aplicá-las dá o texto.
  let refeito = original;
  for (const mudanca of [...mudancas].reverse()) {
    refeito = refeito.slice(0, mudanca.de) + mudanca.inserir + refeito.slice(mudanca.ate);
  }
  assert.equal(refeito, texto);
  assert.deepEqual(converter(texto), { texto, mudancas: [] });
});
