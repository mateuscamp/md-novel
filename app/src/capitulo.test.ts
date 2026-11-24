import assert from "node:assert/strict";
import { test } from "node:test";
import { sugestaoDeNome } from "./capitulo.ts";

test("a sugestão para o capítulo novo continua a numeração do último", () => {
  assert.equal(sugestaoDeNome(["01-abertura.md", "02-a-casa.md"]), "03-");
  assert.equal(sugestaoDeNome(["67-patinete.md"]), "68-");
  assert.equal(sugestaoDeNome(["09-a.md"]), "10-");
  assert.equal(sugestaoDeNome(["99-a.md"]), "100-");
  assert.equal(sugestaoDeNome(["007 o agente.md"]), "008 ");
  assert.equal(sugestaoDeNome(["1.md", "2.md"]), "3");
});

test("livro vazio começa do 01; sem número no último capítulo, sem sugestão", () => {
  assert.equal(sugestaoDeNome([]), "01-");
  assert.equal(sugestaoDeNome(["01-a.md", "epilogo.md"]), "");
  assert.equal(sugestaoDeNome(["capitulo9.md"]), "");
});
