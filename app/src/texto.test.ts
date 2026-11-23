import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { contarPalavras, tituloDoTexto } from "./texto.ts";

/** As linhas `esperado|texto` de um arquivo de casos do Rust, com `\n` desfeito. */
function casos(arquivo: string): [string, string][] {
  const caminho = new URL(`../../crates/workspace/tests/fixtures/${arquivo}`, import.meta.url);
  const casos = readFileSync(caminho, "utf8")
    .split("\n")
    // Comentário começa com "# "; um caso começa pelo valor esperado.
    .filter((linha) => !linha.startsWith("# ") && linha.includes("|"))
    .map((linha): [string, string] => {
      const corte = linha.indexOf("|");
      return [linha.slice(0, corte), linha.slice(corte + 1).replaceAll("\\n", "\n")];
    });
  assert.ok(casos.length >= 8, `esperava os casos do arquivo, li ${casos.length}`);
  return casos;
}

test("conta as palavras como os casos compartilhados com o Rust", () => {
  for (const [esperado, texto] of casos("contagem.txt")) {
    assert.equal(contarPalavras(texto), Number(esperado), `texto: ${JSON.stringify(texto)}`);
  }
});

test("acha o título como os casos compartilhados com o Rust", () => {
  for (const [esperado, texto] of casos("titulos.txt")) {
    assert.equal(
      tituloDoTexto(texto),
      esperado === "" ? null : esperado,
      `texto: ${JSON.stringify(texto)}`,
    );
  }
});
