import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

// As dependências da interface são uma decisão (ADR 0003): o editor
// (CodeMirror 6), o analisador de Markdown e a ponte do Tauri. Pacote novo
// entra aqui junto com o motivo no ADR; é o mesmo papel dos `fronteira.rs`
// dos crates.
const DEPENDENCIAS = [
  "@codemirror/commands",
  "@codemirror/language",
  "@codemirror/search",
  "@codemirror/state",
  "@codemirror/view",
  "@lezer/highlight",
  "@lezer/markdown",
  "@tauri-apps/api",
];

test("as dependências do app são as do ADR 0003, com versão exata", () => {
  const pacote = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8")) as {
    dependencies: Record<string, string>;
  };
  assert.deepEqual(Object.keys(pacote.dependencies).sort(), DEPENDENCIAS);
  // Versão exata: atualizar é uma mudança que alguém fez, não uma que chegou.
  for (const [nome, versao] of Object.entries(pacote.dependencies)) {
    assert.match(versao, /^\d+\.\d+\.\d+$/, `${nome}: ${versao}`);
  }
});
