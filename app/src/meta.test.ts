import assert from "node:assert/strict";
import { test } from "node:test";
import { diaDe, escreverHoje, lerHoje, lerMeta, META_MAXIMA, rotuloDeHoje } from "./meta.ts";

test("o dia é o do relógio do autor, não o de Greenwich", () => {
  assert.equal(diaDe(new Date(2026, 10, 24, 0, 5)), "2026-11-24");
  assert.equal(diaDe(new Date(2026, 0, 3, 23, 59)), "2026-01-03");
});

test("no mesmo dia a conta parte do total guardado; em outro dia, do total de agora", () => {
  const guardado = escreverHoje("2026-11-24", 1000);
  assert.equal(lerHoje(guardado, "2026-11-24", 1300), 1000);
  assert.equal(lerHoje(guardado, "2026-11-25", 1300), 1300);
  for (const ruim of [null, "", "{", "7", '{"dia":"2026-11-24"}', '{"dia":"2026-11-24","base":-1}', '{"dia":"2026-11-24","base":1.5}']) {
    assert.equal(lerHoje(ruim, "2026-11-24", 1300), 1300, String(ruim));
  }
});

test("a meta é um inteiro de 1 a META_MAXIMA; o resto é sem meta", () => {
  assert.equal(lerMeta("500"), 500);
  assert.equal(lerMeta(" 1200 "), 1200);
  assert.equal(lerMeta(String(META_MAXIMA)), META_MAXIMA);
  for (const ruim of [null, "", "0", "-5", "abc", "12.5", "1e3", String(META_MAXIMA + 1)]) {
    assert.equal(lerMeta(ruim), null, String(ruim));
  }
});

test("o rótulo diz as palavras de hoje, com a meta quando há uma", () => {
  assert.equal(rotuloDeHoje(0, null), "Hoje: 0 palavras");
  assert.equal(rotuloDeHoje(1, null), "Hoje: 1 palavra");
  assert.equal(rotuloDeHoje(1312, null), "Hoje: 1.312 palavras");
  assert.equal(rotuloDeHoje(312, 500), "Hoje: 312 de 500 palavras");
  assert.equal(rotuloDeHoje(500, 500), "Hoje: 500 de 500 palavras ✓");
  // Cortou mais do que escreveu: a conta do dia fica negativa, e é o que aparece.
  assert.equal(rotuloDeHoje(-40, 500), "Hoje: -40 de 500 palavras");
});
