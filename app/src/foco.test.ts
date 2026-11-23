import assert from "node:assert/strict";
import { test } from "node:test";
import { teclaDoFoco } from "./foco.ts";

test("F8 liga e desliga o modo foco", () => {
  assert.equal(teclaDoFoco("F8", false), "alternar");
  assert.equal(teclaDoFoco("F8", true), "alternar");
});

test("Esc sai do modo foco, e fora dele continua sendo do editor", () => {
  assert.equal(teclaDoFoco("Escape", true), "sair");
  assert.equal(teclaDoFoco("Escape", false), null);
});

test("as outras teclas não mexem no modo foco", () => {
  assert.equal(teclaDoFoco("a", true), null);
  assert.equal(teclaDoFoco("F9", false), null);
});

test("no meio da composição de um acento, o Esc é de quem está compondo", () => {
  assert.equal(teclaDoFoco("Escape", true, true), null);
  assert.equal(teclaDoFoco("F8", true, true), null);
  assert.equal(teclaDoFoco("Escape", true, false), "sair");
});
