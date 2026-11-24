import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import {
  acaoDaRoda,
  atalhoDaTecla,
  lerLigado,
  lerZoom,
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

test("a faixa do zoom é a mesma na interface e no Rust, que é quem confere", () => {
  const rust = readFileSync(new URL("../src-tauri/src/janela.rs", import.meta.url), "utf8");
  const constante = (nome: string) => Number(new RegExp(`pub const ${nome}: u32 = (\\d+);`).exec(rust)?.[1]);
  assert.equal(constante("ZOOM_MINIMO"), ZOOM_MINIMO);
  assert.equal(constante("ZOOM_MAXIMO"), ZOOM_MAXIMO);
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
});
