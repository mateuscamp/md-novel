import assert from "node:assert/strict";
import { test } from "node:test";
import { chavesParaMudar } from "./livros.ts";

test("renomear o livro leva junto onde o autor parou e a conta de hoje", () => {
  const chaves = [
    "md-novel.tema",
    "md-novel.onde:/textos/o-dom-da-pele-md",
    "md-novel.hoje:/textos/o-dom-da-pele-md",
    "md-novel.onde:/textos/md",
  ];
  assert.deepEqual(chavesParaMudar(chaves, "/textos/o-dom-da-pele-md", "/textos/O dom da pele"), [
    ["md-novel.onde:/textos/o-dom-da-pele-md", "md-novel.onde:/textos/O dom da pele"],
    ["md-novel.hoje:/textos/o-dom-da-pele-md", "md-novel.hoje:/textos/O dom da pele"],
  ]);
});

test("o registro de outro livro fica onde está, mesmo com o nome parecido", () => {
  const chaves = [
    "md-novel.onde:/textos/livro 2",
    "md-novel.hoje:/textos/livro/sub",
    "md-novel.onde:/outros/textos/livro",
    "md-novel.meta",
  ];
  assert.deepEqual(chavesParaMudar(chaves, "/textos/livro", "/textos/Livro"), []);
  // Sem mudança de caminho, nada a mudar.
  assert.deepEqual(chavesParaMudar(["md-novel.onde:/a"], "/a", "/a"), []);
});
