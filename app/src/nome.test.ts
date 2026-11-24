import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

// O projeto tem dois nomes, e cada um tem o seu lugar (ADR 0003): `md-novel`,
// sem espaço e sem maiúscula, em pasta, comando, identificador e classe de
// CSS; "md Novel" onde uma pessoa lê. Aqui ficam os lugares que a pessoa vê
// antes de qualquer outro: a janela, a página e a primeira linha do README.
const NOME = "md Novel";

const ler = (caminho: string) => readFileSync(new URL(caminho, import.meta.url), "utf8");

test("a janela leva o nome que as pessoas leem", () => {
  const configuracao = JSON.parse(ler("../src-tauri/tauri.conf.json")) as {
    productName: string;
    identifier: string;
    app: { windows: { title: string }[] };
  };
  assert.equal(configuracao.productName, NOME);
  assert.deepEqual(
    configuracao.app.windows.map((janela) => janela.title),
    [NOME],
  );
  // O identificador vira nome de pasta: continua sem espaço e sem maiúscula.
  assert.equal(configuracao.identifier, "io.github.mateuscamp.md-novel");
});

test("a página e o README levam o nome que as pessoas leem", () => {
  const pagina = ler("../index.html");
  assert.ok(pagina.includes(`<title>${NOME}</title>`), "título da página");
  assert.ok(pagina.includes(`<h1 id="nome-do-livro">${NOME}</h1>`), "cabeçalho da lateral sem livro aberto");
  assert.ok(ler("../../README.md").startsWith(`# ${NOME}\n`), "primeira linha do README");
});
