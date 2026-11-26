import assert from "node:assert/strict";
import { test } from "node:test";
import { achadoAindaVale, resumoDosAchados, termoDaSelecao } from "./achados.ts";

test("o resumo da busca diz quantos achados em quantos capítulos", () => {
  assert.equal(resumoDosAchados(0, 0, false), "Nada encontrado no livro.");
  assert.equal(resumoDosAchados(1, 1, false), "1 achado em 1 capítulo");
  assert.equal(resumoDosAchados(3, 1, false), "3 achados em 1 capítulo");
  assert.equal(resumoDosAchados(1234, 12, false), "1.234 achados em 12 capítulos");
});

test("busca cortada avisa que há mais do que a lista mostra", () => {
  assert.equal(
    resumoDosAchados(500, 7, true),
    "Mais de 500 achados: a lista mostra os primeiros 500, em 7 capítulos. Escreva mais letras para afinar.",
  );
});

test("a busca abre com o que está selecionado, se for um trecho curto de uma linha só", () => {
  assert.equal(termoDaSelecao("cigana"), "cigana");
  assert.equal(termoDaSelecao("  a cigana velha  "), "a cigana velha");
  assert.equal(termoDaSelecao(""), "");
  assert.equal(termoDaSelecao("   "), "");
  assert.equal(termoDaSelecao("duas\nlinhas"), "");
  assert.equal(termoDaSelecao("x".repeat(100)), "x".repeat(100));
  assert.equal(termoDaSelecao("x".repeat(101)), "");
});

test("o achado só vale se a palavra e a vizinhança dela ainda estão no mesmo lugar", () => {
  const texto = "A cigana velha e a cigana nova.";
  const segundo = { inicio: 19, fim: 25, antes: "A cigana velha e a ", trecho: "cigana", depois: " nova." };
  assert.equal(achadoAindaVale(texto, segundo), true);
  // Cortado com reticências dos dois lados, como vem do Rust.
  assert.equal(achadoAindaVale(texto, { ...segundo, antes: "…velha e a ", depois: " no…" }), true);
  // O autor apagou texto antes: a posição antiga cai noutra "cigana".
  const editado = "A velha e a cigana cigana nova.";
  assert.equal(editado.slice(19, 25), "cigana");
  assert.equal(achadoAindaVale(editado, segundo), false);
  // A palavra mudou, ou o capítulo ficou mais curto que a posição.
  assert.equal(achadoAindaVale("A cigana velha e a CIGANA nova.", segundo), false);
  assert.equal(achadoAindaVale("curto", segundo), false);
  // O que vem depois mudou.
  assert.equal(achadoAindaVale("A cigana velha e a cigana velha.", segundo), false);
});
