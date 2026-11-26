import assert from "node:assert/strict";
import { test } from "node:test";
import {
  acaoDaPinca,
  acaoDaRoda,
  atalhoDaTecla,
  fatorDoZoom,
  lerDesligado,
  lerLigado,
  lerZoom,
  modificadorDaRoda,
  rotuloDoZoom,
  ZOOM_MAXIMO,
  ZOOM_MINIMO,
  zoomDepoisDe,
} from "./conforto.ts";

test("o zoom guardado vale se for um número da faixa; senão é 100%", () => {
  assert.equal(lerZoom("130"), 130);
  assert.equal(lerZoom(String(ZOOM_MINIMO)), ZOOM_MINIMO);
  assert.equal(lerZoom(String(ZOOM_MAXIMO)), ZOOM_MAXIMO);
  for (const ruim of [null, "", "abc", "12.5", "40", "310", "-100", "1e2", " 120"]) {
    assert.equal(lerZoom(ruim), 100, String(ruim));
  }
});

test("o fator do zoom é o por cento dividido por cem, escrito como o CSS lê", () => {
  assert.equal(fatorDoZoom(100), "1");
  assert.equal(fatorDoZoom(110), "1.1");
  assert.equal(fatorDoZoom(50), "0.5");
  assert.equal(fatorDoZoom(300), "3");
});

test("o zoom anda de 10 em 10 e para nas pontas da faixa", () => {
  assert.equal(zoomDepoisDe(100, "aumentar"), 110);
  assert.equal(zoomDepoisDe(100, "diminuir"), 90);
  assert.equal(zoomDepoisDe(170, "restaurar"), 100);
  assert.equal(zoomDepoisDe(ZOOM_MAXIMO, "aumentar"), ZOOM_MAXIMO);
  assert.equal(zoomDepoisDe(ZOOM_MINIMO, "diminuir"), ZOOM_MINIMO);
  assert.equal(rotuloDoZoom(110), "110%");
});

test("a rolagem só mexe no zoom com o Ctrl apertado: para cima aproxima", () => {
  assert.equal(acaoDaRoda(-120, true), "aumentar");
  assert.equal(acaoDaRoda(53, true), "diminuir");
  assert.equal(acaoDaRoda(0, true), null);
  // Sem Ctrl a rolagem é do texto.
  assert.equal(acaoDaRoda(-120, false), null);
});

test("na roda, Ctrl vale sempre (inclusive na pinça); Cmd vale no Mac e Super nunca vale no Linux", () => {
  // No Linux: só Ctrl sem Alt.
  assert.equal(modificadorDaRoda(true, false, false, false), true);
  assert.equal(modificadorDaRoda(false, true, false, false), false);
  assert.equal(modificadorDaRoda(true, false, true, false), false);
  // No Mac: Ctrl (pinça do trackpad ou Ctrl+rolagem) e Cmd+rolagem valem, sem Alt.
  assert.equal(modificadorDaRoda(true, false, false, true), true);
  assert.equal(modificadorDaRoda(false, true, false, true), true);
  assert.equal(modificadorDaRoda(false, false, false, true), false);
  assert.equal(modificadorDaRoda(true, false, true, true), false);
  assert.equal(modificadorDaRoda(false, true, true, true), false);
});

test("a pinça no trackpad aproxima e afasta de 10% em 10% a partir da base do gesto", () => {
  // Movimento menor que um degrau ainda não muda o zoom.
  assert.equal(acaoDaPinca(1.05, 1), null);
  assert.equal(acaoDaPinca(0.95, 1), null);
  // Abrir os dedos 10% ou mais aproxima e move a base.
  assert.deepEqual(acaoDaPinca(1.12, 1), { acao: "aumentar", novaBase: 1.12 });
  // Fechar os dedos a partir da nova base afasta quando cai 10%.
  assert.equal(acaoDaPinca(1.05, 1.12), null);
  assert.deepEqual(acaoDaPinca(1.0, 1.12), { acao: "diminuir", novaBase: 1.0 });
  // Escalas inválidas não mexem no zoom.
  for (const ruim of [0, -1, Number.NaN, Number.POSITIVE_INFINITY]) {
    assert.equal(acaoDaPinca(ruim, 1), null, String(ruim));
    assert.equal(acaoDaPinca(1.2, ruim), null, String(ruim));
  }
});

test("as teclas de conforto: zoom com Ctrl, lateral, máquina de escrever e tela cheia", () => {
  assert.equal(atalhoDaTecla("=", true), "zoom-aumentar");
  assert.equal(atalhoDaTecla("+", true), "zoom-aumentar");
  assert.equal(atalhoDaTecla("-", true), "zoom-diminuir");
  assert.equal(atalhoDaTecla("0", true), "zoom-restaurar");
  assert.equal(atalhoDaTecla("F9", false), "lateral");
  assert.equal(atalhoDaTecla("F10", false), "maquina");
  assert.equal(atalhoDaTecla("F11", false), "tela-cheia");
  // Sem Ctrl, as teclas de zoom são texto do capítulo.
  for (const tecla of ["=", "+", "-", "0"]) assert.equal(atalhoDaTecla(tecla, false), null, tecla);
  assert.equal(atalhoDaTecla("a", true), null);
  // No meio da composição de um acento a tecla é de quem está compondo.
  assert.equal(atalhoDaTecla("F9", false, true), null);
  assert.equal(atalhoDaTecla("=", true, true), null);
});

test("uma escolha de liga e desliga guardada só vale se for exatamente \"sim\"", () => {
  assert.equal(lerLigado("sim"), true);
  for (const outro of [null, "", "nao", "true", "1", "SIM"]) assert.equal(lerLigado(outro), false, String(outro));
  // O que vem ligado de fábrica só desliga com o "nao" guardado.
  assert.equal(lerDesligado("nao"), true);
  for (const outro of [null, "", "sim", "false", "NAO"]) assert.equal(lerDesligado(outro), false, String(outro));
});

test("Ctrl+Shift+F abre a busca no livro; Ctrl+F sozinho continua sendo a busca do capítulo", () => {
  assert.equal(atalhoDaTecla("F", true, false, true), "buscar-no-livro");
  assert.equal(atalhoDaTecla("f", true, false, true), "buscar-no-livro");
  // Sem Shift é do editor (a busca no capítulo); sem Ctrl é texto.
  assert.equal(atalhoDaTecla("f", true), null);
  assert.equal(atalhoDaTecla("F", true, false, false), null);
  assert.equal(atalhoDaTecla("F", false, false, true), null);
  assert.equal(atalhoDaTecla("F", true, true, true), null, "no meio de um acento");
  // O Shift não muda as outras teclas: em alguns teclados o + pede Shift.
  assert.equal(atalhoDaTecla("+", true, false, true), "zoom-aumentar");
});
