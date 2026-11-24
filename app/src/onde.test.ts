import assert from "node:assert/strict";
import { test } from "node:test";
import { escreverOnde, lerOnde } from "./onde.ts";

const CAPITULOS = ["01.md", "02.md"];

test("o lugar guardado volta se o capítulo ainda existe no livro", () => {
  const guardado = escreverOnde("02.md", 137);
  assert.deepEqual(lerOnde(guardado, CAPITULOS), { capitulo: "02.md", posicao: 137 });
});

test("capítulo que sumiu, ou registro estragado, não vale: abre no começo", () => {
  assert.equal(lerOnde(escreverOnde("03.md", 5), CAPITULOS), null);
  for (const ruim of [null, "", "{", "[]", "null", '"02.md"', '{"capitulo":2,"posicao":1}']) {
    assert.equal(lerOnde(ruim, CAPITULOS), null, String(ruim));
  }
});

test("posição que não é um inteiro de zero para cima vira o começo do capítulo", () => {
  // Sem posição nenhuma também.
  assert.deepEqual(lerOnde('{"capitulo":"02.md"}', CAPITULOS), { capitulo: "02.md", posicao: 0 });
  for (const ruim of [-1, 1.5, "7", null, 1e20]) {
    const guardado = JSON.stringify({ capitulo: "01.md", posicao: ruim });
    assert.deepEqual(lerOnde(guardado, CAPITULOS), { capitulo: "01.md", posicao: 0 }, String(ruim));
  }
});
