import assert from "node:assert/strict";
import { test } from "node:test";
import { lerPreferencia, proximaPreferencia, rotuloDaPreferencia, temaEfetivo } from "./tema.ts";

test("sem escolha guardada, ou com valor que não se conhece, segue o sistema", () => {
  assert.equal(lerPreferencia(null), "sistema");
  assert.equal(lerPreferencia(""), "sistema");
  assert.equal(lerPreferencia("roxo"), "sistema");
  assert.equal(lerPreferencia("claro"), "claro");
  assert.equal(lerPreferencia("escuro"), "escuro");
  assert.equal(lerPreferencia("sistema"), "sistema");
});

test("a escolha do autor vence o sistema; 'sistema' segue o que o sistema diz", () => {
  assert.equal(temaEfetivo("sistema", false), "claro");
  assert.equal(temaEfetivo("sistema", true), "escuro");
  assert.equal(temaEfetivo("claro", true), "claro");
  assert.equal(temaEfetivo("escuro", false), "escuro");
});

test("o botão passa por sistema, claro e escuro, e volta", () => {
  assert.equal(proximaPreferencia("sistema"), "claro");
  assert.equal(proximaPreferencia("claro"), "escuro");
  assert.equal(proximaPreferencia("escuro"), "sistema");
});

test("o rótulo do botão diz a escolha atual", () => {
  assert.equal(rotuloDaPreferencia("sistema"), "Tema: sistema");
  assert.equal(rotuloDaPreferencia("claro"), "Tema: claro");
  assert.equal(rotuloDaPreferencia("escuro"), "Tema: escuro");
});
