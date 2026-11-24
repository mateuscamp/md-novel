import assert from "node:assert/strict";
import { test } from "node:test";
import { marcarLinhas } from "./blocos.ts";

const estilos = (texto: string) => marcarLinhas(texto.split("\n")).map((marca) => marca.estilo);

test("linhas dentro de um bloco custom-style recebem o estilo; os marcadores não", () => {
  const marcas = marcarLinhas(['::: {custom-style="Epígrafe"}', "A pele guarda.", ":::", "Corpo."]);
  assert.deepEqual(
    marcas.map((marca) => marca.estilo),
    [null, "Epígrafe", null, null],
  );
  assert.deepEqual(
    marcas.map((marca) => marca.marcador),
    [true, false, true, false],
  );
});

test("todas as linhas do bloco recebem o estilo, inclusive as vazias", () => {
  assert.deepEqual(estilos('::: {custom-style="Carta"}\nQuerida,\n\nescrevo de longe.\n:::'), [
    null,
    "Carta",
    "Carta",
    "Carta",
    null,
  ]);
});

test("aceita outros atributos, aspas simples e mais de três dois-pontos", () => {
  assert.deepEqual(estilos('::::: {.carta custom-style="Carta do avô" lang=pt}\ntexto\n:::::'), [
    null,
    "Carta do avô",
    null,
  ]);
  assert.deepEqual(estilos(":::{custom-style='Placa'}\ntexto\n:::"), [null, "Placa", null]);
});

test("bloco sem custom-style não muda o estilo, mas conta para o fechamento", () => {
  assert.deepEqual(estilos('::: {custom-style="Carta"}\n::: aviso\ndentro\n:::\nainda carta\n:::\nfora'), [
    null,
    null,
    "Carta",
    null,
    "Carta",
    null,
    null,
  ]);
  assert.deepEqual(estilos("::: {.nota}\nsem estilo\n:::"), [null, null, null]);
});

test("bloco dentro de bloco usa o estilo mais interno", () => {
  assert.deepEqual(
    estilos('::: {custom-style="Carta"}\na\n::: {custom-style="Placa"}\nb\n:::\nc\n:::'),
    [null, "Carta", null, "Placa", null, "Carta", null],
  );
});

test("fechamento sem abertura é linha comum, e bloco não fechado vai até o fim", () => {
  const solto = marcarLinhas([":::", "texto"]);
  assert.deepEqual(
    solto.map((marca) => [marca.estilo, marca.marcador]),
    [
      [null, false],
      [null, false],
    ],
  );
  assert.deepEqual(estilos('::: {custom-style="Carta"}\na\nb'), [null, "Carta", "Carta"]);
});

// Até 2026-11-23 este teste dizia "um a seis #" e dava `####### Sete` como
// texto: era a regra do CommonMark. Quem escreve o .docx é o Pandoc, que
// aceita mais de seis (`pandoc -t native` dá `Header 7`), e a regra passou a
// ser a dele, a mesma de `tituloDoTexto`.
test("título é a linha que começa com # e um espaço, como no Pandoc", () => {
  const marcas = marcarLinhas(["# Capítulo um", "###### Seis", "####### Sete", "#semespaço", "texto # não é"]);
  assert.deepEqual(
    marcas.map((marca) => marca.titulo),
    [true, true, true, false, false],
  );
  assert.deepEqual(
    marcarLinhas(["#", "   # recuado"]).map((marca) => marca.titulo),
    [true, false],
  );
});

test("título colado num parágrafo é texto do parágrafo, como no Pandoc", () => {
  const titulos = (texto: string) => marcarLinhas(texto.split("\n")).map((marca) => marca.titulo);
  assert.deepEqual(titulos("texto\n# Colado\n\n# Depois"), [false, false, false, true]);
  // Depois de outro título, de um bloco `:::` ou de uma cerca de código, vale.
  assert.deepEqual(titulos("# Um\n## Dois"), [true, true]);
  assert.deepEqual(titulos("::: {.a}\n# Dentro\ntexto\n:::\n# Depois"), [false, true, false, false, true]);
  assert.deepEqual(titulos("```\ncódigo\n```\n# Depois"), [false, false, false, true]);
  // Um `:::` solto é texto: o título colado nele não vale.
  assert.deepEqual(titulos(":::\n# Colado no solto"), [false, false]);
});

test("texto vazio dá uma marca por linha", () => {
  assert.equal(marcarLinhas([""]).length, 1);
  assert.equal(marcarLinhas([]).length, 0);
});

test("dentro de um bloco de código cercado nada é marcador, título nem estilo", () => {
  const texto = [
    '::: {custom-style="Carta"}',
    "antes",
    "```markdown",
    '::: {custom-style="Placa"}',
    "# não é título",
    ":::",
    "```",
    "depois",
    ":::",
    "fora",
    "~~~~",
    ":::",
    "~~~",
    "ainda código: a cerca de fechamento precisa ter o mesmo tamanho",
    "~~~~",
    "# Título de verdade",
  ];
  const marcas = marcarLinhas(texto);
  assert.deepEqual(
    marcas.map((marca) => marca.estilo),
    [null, "Carta", null, null, null, null, null, "Carta", null, null, null, null, null, null, null, null],
  );
  assert.deepEqual(
    marcas.map((marca) => marca.marcador),
    [true, false, false, false, false, false, false, false, true, false, false, false, false, false, false, false],
  );
  assert.deepEqual(
    marcas.map((marca) => marca.titulo),
    [...Array(15).fill(false), true],
  );
});

// Parágrafos como no Word (ADR 0003): a linha em branco que o Markdown pede
// entre um parágrafo e outro some da tela. É a linha vazia logo depois de
// uma linha com texto, quando ainda há linha depois dela.
test("a linha em branco entre dois parágrafos é separadora; as outras linhas em branco não", () => {
  const separadoras = (texto: string) => marcarLinhas(texto.split("\n")).map((marca) => marca.separador);
  assert.deepEqual(separadoras("A\n\nB"), [false, true, false]);
  // Da segunda linha em branco seguida em diante, o espaço é do autor e aparece.
  assert.deepEqual(separadoras("A\n\n\nB"), [false, true, false, false]);
  // A última linha do capítulo é onde o cursor fica ao começar um parágrafo.
  assert.deepEqual(separadoras("A\n"), [false, false]);
  assert.deepEqual(separadoras("A\n\n"), [false, true, false]);
  // No começo do capítulo não há parágrafo antes.
  assert.deepEqual(separadoras("\nA"), [false, false]);
  // Depois de título e de marcador de bloco também é separadora.
  assert.deepEqual(separadoras("# T\n\ntexto\n\n::: {.a}\nx\n:::\n\nfim"), [
    false, true, false, true, false, false, false, true, false,
  ]);
  // Linha só com espaços não é vazia: fica à vista.
  assert.deepEqual(separadoras("A\n  \nB"), [false, false, false]);
});

test("dentro de um bloco de código nenhuma linha é separadora, e a linha é marcada como código", () => {
  const marcas = marcarLinhas("antes\n\n```\na\n\nb\n```\n\ndepois".split("\n"));
  assert.deepEqual(
    marcas.map((marca) => marca.separador),
    [false, true, false, false, false, false, false, true, false],
  );
  assert.deepEqual(
    marcas.map((marca) => marca.codigo),
    [false, false, true, true, true, true, true, false, false],
  );
});
