import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { FRASES_DA_BUSCA } from "./busca.ts";

// O painel de busca é do CodeMirror e vem em inglês; cada texto dele passa
// por `phrase(...)`. Este teste lê os textos do pacote instalado: atualizar
// o pacote e ganhar um texto novo sem tradução reprova aqui.
test("todo texto do painel de busca do CodeMirror tem tradução", () => {
  const pacote = readFileSync(
    new URL("../node_modules/@codemirror/search/dist/index.js", import.meta.url),
    "utf8",
  );
  const doPacote = [...new Set([...pacote.matchAll(/phrase\((?:view, )?"([^"]+)"/g)].map(([, frase]) => frase))].sort();
  assert.ok(doPacote.length >= 15, `achei só ${doPacote.length} textos no pacote`);
  assert.deepEqual(Object.keys(FRASES_DA_BUSCA).sort(), doPacote);
  for (const [original, traducao] of Object.entries(FRASES_DA_BUSCA)) {
    assert.notEqual(traducao.trim(), "", original);
    // O `$` é onde o CodeMirror põe o número: a tradução precisa mantê-lo.
    assert.equal(traducao.includes("$"), original.includes("$"), original);
  }
});
