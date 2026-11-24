import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import {
  camposDe,
  comCampo,
  comEstiloNovo,
  type EstiloNoPainel,
  herdado,
  juntarMedida,
  nomeDeEstiloNovo,
  partirMedida,
  semEstilo,
} from "./painel.ts";

const ESTILO: EstiloNoPainel = {
  corpo: {
    fonte: "EB Garamond",
    tamanho: "12pt",
    entrelinha: 1.5,
    alinhamento: "justificado",
    "recuo-primeira-linha": "1.25cm",
    "espaco-antes": "0pt",
    "espaco-depois": "0pt",
    italico: false,
    negrito: false,
  },
  estilos: { Epígrafe: { alinhamento: "direita", italico: true } },
  existe: true,
};

// Propriedade nova de estilo precisa de campo no painel: cada chave do
// estilo tem no `index.html` um elemento marcado com ela, que é onde o painel
// mostra se o estilo nomeado a herda.
test("toda propriedade do estilo tem campo no painel, e nenhum campo sobra", () => {
  const pagina = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  const marcados = [...pagina.matchAll(/data-campo="([^"]+)"/g)].map(([, campo]) => campo).sort();
  assert.deepEqual(marcados, Object.keys(ESTILO.corpo).sort());
});

test("o estilo nomeado mostra o que troca e, no resto, o que herda do texto do livro", () => {
  assert.deepEqual(camposDe(ESTILO, null), ESTILO.corpo);
  const epigrafe = camposDe(ESTILO, "Epígrafe");
  assert.equal(epigrafe.alinhamento, "direita");
  assert.equal(epigrafe.italico, true);
  assert.equal(epigrafe.tamanho, "12pt");
  assert.equal(herdado(ESTILO, "Epígrafe", "tamanho"), true);
  assert.equal(herdado(ESTILO, "Epígrafe", "alinhamento"), false);
  // No texto do livro nada é herdado.
  assert.equal(herdado(ESTILO, null, "tamanho"), false);
});

test("mudar o texto do livro muda o estilo nomeado no que ele herda, e só nisso", () => {
  const maior = comCampo(ESTILO, null, "tamanho", "14pt");
  assert.equal(maior.corpo.tamanho, "14pt");
  assert.equal(camposDe(maior, "Epígrafe").tamanho, "14pt");
  const alinhado = comCampo(ESTILO, null, "alinhamento", "esquerda");
  assert.equal(camposDe(alinhado, "Epígrafe").alinhamento, "direita");
  // O original não é mexido.
  assert.equal(ESTILO.corpo.tamanho, "12pt");
});

test("mudar um campo do estilo nomeado troca só nele; voltar ao valor do texto do livro volta a herdar", () => {
  const trocado = comCampo(ESTILO, "Epígrafe", "tamanho", "10pt");
  assert.equal(camposDe(trocado, "Epígrafe").tamanho, "10pt");
  assert.equal(trocado.corpo.tamanho, "12pt");
  assert.equal(herdado(trocado, "Epígrafe", "tamanho"), false);
  const devolvido = comCampo(trocado, "Epígrafe", "tamanho", "12pt");
  assert.equal(herdado(devolvido, "Epígrafe", "tamanho"), true);
  assert.deepEqual(devolvido.estilos.Epígrafe, ESTILO.estilos.Epígrafe);
  // Estilo que não existe: nada muda.
  assert.deepEqual(comCampo(ESTILO, "Não existe", "tamanho", "10pt"), ESTILO);
});

test("estilo novo nasce herdando tudo, e apagar tira só ele", () => {
  const comNovo = comEstiloNovo(ESTILO, "Carta");
  assert.deepEqual(comNovo.estilos.Carta, {});
  assert.deepEqual(camposDe(comNovo, "Carta"), ESTILO.corpo);
  assert.deepEqual(Object.keys(comNovo.estilos), ["Epígrafe", "Carta"]);
  assert.deepEqual(semEstilo(comNovo, "Epígrafe").estilos, { Carta: {} });
  assert.deepEqual(ESTILO.estilos, { Epígrafe: { alinhamento: "direita", italico: true } });
});

test("o nome do estilo novo perde os espaços de fora e não pode ser vazio nem repetido", () => {
  assert.equal(nomeDeEstiloNovo(ESTILO, "  Carta do avô "), "Carta do avô");
  assert.equal(nomeDeEstiloNovo(ESTILO, "   "), null);
  assert.equal(nomeDeEstiloNovo(ESTILO, "Epígrafe"), null);
  // O Word não distingue maiúscula em nome de estilo.
  assert.equal(nomeDeEstiloNovo(ESTILO, "epígrafe"), null);
});

test("a medida do arquivo vira número e unidade para o campo, e volta", () => {
  assert.deepEqual(partirMedida("12pt"), { numero: 12, unidade: "pt" });
  assert.deepEqual(partirMedida("1.25cm"), { numero: 1.25, unidade: "cm" });
  assert.deepEqual(partirMedida("0mm"), { numero: 0, unidade: "mm" });
  for (const ruim of ["", "12", "pt", "1,25cm", "12 pt", "-1pt", "12px"]) assert.equal(partirMedida(ruim), null, ruim);
  assert.equal(juntarMedida(14, "pt"), "14pt");
  assert.equal(juntarMedida(0.75, "cm"), "0.75cm");
});
