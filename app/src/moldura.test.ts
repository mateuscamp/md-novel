import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

const pagina = readFileSync(new URL("../index.html", import.meta.url), "utf8");

/** O que está escrito na abertura e dentro de um botão da página. */
function botao(id: string): { abertura: string; dentro: string } {
  const achado = new RegExp(`<button id="${id}"([^>]*)>([\\s\\S]*?)</button>`).exec(pagina);
  assert.ok(achado, `falta o botão #${id}`);
  return { abertura: achado[1] ?? "", dentro: achado[2] ?? "" };
}

// O autor pediu símbolo no lugar do nome em "Máquina" e "Foco" (2026-11-24);
// o da lateral já era símbolo. O desenho vai na própria página, em SVG, para
// não depender de a fonte do sistema ter o caractere.
test("lateral, máquina de escrever e foco são botões de símbolo, desenhados na página", () => {
  for (const id of ["alternar-lateral", "maquina", "foco"]) {
    const { dentro } = botao(id);
    assert.match(dentro, /<svg[\s\S]*<\/svg>/, id);
    assert.equal(dentro.replace(/<svg[\s\S]*<\/svg>/, "").trim(), "", `${id}: só o símbolo, sem texto`);
    assert.match(dentro, /aria-hidden="true"/, `${id}: o desenho não é lido em voz alta`);
  }
});

test("todo botão sem texto tem nome para leitor de tela e dica para o mouse", () => {
  const botoes = [...pagina.matchAll(/<button([^>]*)>([\s\S]*?)<\/button>/g)];
  assert.ok(botoes.length > 10, `achei só ${botoes.length} botões`);
  for (const [, abertura = "", dentro = ""] of botoes) {
    // Botão de símbolo ou de ×. O botão vazio na página é o que o programa
    // preenche com texto ao rodar (as palavras de hoje), e não entra aqui.
    const deSimbolo = dentro.includes("<svg") || dentro.trim() === "×";
    if (!deSimbolo) continue;
    assert.match(abertura, /aria-label="[^"]+"/, abertura);
    assert.match(abertura, /title="[^"]+"/, abertura);
  }
});
