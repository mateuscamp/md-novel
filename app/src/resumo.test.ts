import assert from "node:assert/strict";
import { test } from "node:test";
import {
  type CapituloNaLista,
  comTextoAoVivo,
  nomeDeExibicao,
  numero,
  rotuloDePalavras,
  totalDePalavras,
} from "./resumo.ts";

const LIVRO: CapituloNaLista[] = [
  { nome: "01-abertura.md", titulo: "Capítulo um", palavras: 1200 },
  { nome: "02-a-casa.MD", titulo: null, palavras: 34 },
  { nome: "03-quebrado.md", titulo: null, palavras: null },
];

test("o capítulo aparece pelo título; sem título, pelo nome do arquivo sem a extensão", () => {
  assert.deepEqual(LIVRO.map(nomeDeExibicao), ["Capítulo um", "02-a-casa", "03-quebrado"]);
  assert.equal(nomeDeExibicao({ nome: ".md", titulo: null, palavras: 0 }), ".md");
  assert.equal(nomeDeExibicao({ nome: "v1.2.md", titulo: null, palavras: 0 }), "v1.2");
});

test("o total soma os capítulos lidos e ignora o que não deu para ler", () => {
  assert.equal(totalDePalavras(LIVRO), 1234);
  assert.equal(totalDePalavras([]), 0);
});

test("número com ponto de milhar, como se escreve em português", () => {
  assert.deepEqual([0, 7, 999, 1000, 34567, 1234567].map(numero), [
    "0",
    "7",
    "999",
    "1.000",
    "34.567",
    "1.234.567",
  ]);
});

test("rótulo de palavras: singular, plural e travessão para o que não foi lido", () => {
  assert.equal(rotuloDePalavras(0), "0 palavras");
  assert.equal(rotuloDePalavras(1), "1 palavra");
  assert.equal(rotuloDePalavras(1234), "1.234 palavras");
  assert.equal(rotuloDePalavras(null), "—");
});

test("o capítulo aberto passa a valer pelo texto da tela; os outros ficam como estavam", () => {
  const aoVivo = comTextoAoVivo(LIVRO, "02-a-casa.MD", "# A casa\n\nEla abria as janelas.");
  assert.deepEqual(aoVivo, [
    LIVRO[0],
    { nome: "02-a-casa.MD", titulo: "A casa", palavras: 6 },
    LIVRO[2],
  ]);
  // A lista original não é mexida.
  assert.equal(LIVRO[1]?.titulo, null);
  // Nome que não está na lista não muda nada.
  assert.deepEqual(comTextoAoVivo(LIVRO, "outro.md", "texto"), LIVRO);
});
