import assert from "node:assert/strict";
import { test } from "node:test";
import { nomeParaEditar, sugestaoDeNome, teclaDeRenomear } from "./capitulo.ts";

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

test("o campo de renomear mostra o nome sem o .md, que entra sozinho", () => {
  assert.equal(nomeParaEditar("01-a-casa.md"), "01-a-casa");
  assert.equal(nomeParaEditar("02 São João.MD"), "02 São João");
  assert.equal(nomeParaEditar("notas.md.md"), "notas.md");
  assert.equal(nomeParaEditar(".md"), ".md");
});

test("F2 no capítulo abre o campo de renomear, menos no meio de um acento", () => {
  assert.equal(teclaDeRenomear("F2", false), true);
  assert.equal(teclaDeRenomear("F2", true), false);
  for (const outra of ["F1", "F3", "Enter", "f2", "2"]) assert.equal(teclaDeRenomear(outra, false), false, outra);
});
