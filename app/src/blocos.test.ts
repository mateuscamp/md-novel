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

test("título é a linha que começa com um a seis # e um espaço", () => {
  const marcas = marcarLinhas(["# Capítulo um", "###### Seis", "####### Sete", "#semespaço", "texto # não é", "#"]);
  assert.deepEqual(
    marcas.map((marca) => marca.titulo),
    [true, true, false, false, false, false],
  );
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
