import assert from "node:assert/strict";
import { test } from "node:test";
import { enterCriaParagrafo } from "./paragrafo.ts";

test("numa linha de texto, de título ou de fala, o Enter cria um parágrafo novo", () => {
  for (const linha of ["Era uma vez.", "# Capítulo um", "— Vem.", "  texto com dois espaços antes", "*ênfase*"]) {
    assert.equal(enterCriaParagrafo(linha), true, linha);
  }
});

test("no começo de um parágrafo que já tem linha em branco em cima, o Enter abre uma linha só", () => {
  // Como no Word: Enter no começo do parágrafo o empurra uma linha para baixo.
  assert.equal(enterCriaParagrafo("Texto", { noComeco: true, deCimaVazia: true }), false);
  // No começo de uma linha colada na de cima, separa as duas em parágrafos.
  assert.equal(enterCriaParagrafo("Texto", { noComeco: true, deCimaVazia: false }), true);
  assert.equal(enterCriaParagrafo("Texto", { noComeco: false, deCimaVazia: true }), true);
});

test("onde uma linha em branco estragaria o Markdown, o Enter é o de sempre", () => {
  const linhas = [
    "",
    "   ",
    "- item de lista",
    "* outro",
    "+ outro",
    "1. numerado",
    "2) numerado",
    "> citação",
    "| a | b |",
    '::: {custom-style="Epígrafe"}',
    ":::",
    "    código recuado",
    "\tcódigo recuado",
  ];
  for (const linha of linhas) assert.equal(enterCriaParagrafo(linha), false, JSON.stringify(linha));
});
