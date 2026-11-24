import assert from "node:assert/strict";
import { test } from "node:test";
import { substituicao } from "./tipografia.ts";

/** O que fica escrito depois de digitar `tecla` com `antes` na linha. */
function digitar(antes: string, tecla: string): string {
  const troca = substituicao(antes, tecla);
  if (troca === null) return antes + tecla;
  return antes.slice(0, antes.length - troca.apagar) + troca.inserir;
}

test("aspas duplas abrem depois de espaço ou de abertura e fecham depois de texto", () => {
  assert.equal(digitar("", '"'), "“");
  assert.equal(digitar("Ela disse ", '"'), "Ela disse “");
  assert.equal(digitar("Ela disse “vem", '"'), "Ela disse “vem”");
  assert.equal(digitar("Ela disse “vem.", '"'), "Ela disse “vem.”");
  assert.equal(digitar("(", '"'), "(“");
  assert.equal(digitar("— ", '"'), "— “");
  assert.equal(digitar("—", '"'), "—“");
});

test("o sinal de ênfase colado na aspa não muda se ela abre ou fecha", () => {
  // `*"fala"*`: o asterisco que abre a ênfase vem antes da aspa que abre.
  assert.equal(digitar("*", '"'), "*“");
  assert.equal(digitar("disse **", '"'), "disse **“");
  assert.equal(digitar("disse _", "'"), "disse _‘");
  // `"*fala*"`: o asterisco que fecha a ênfase vem antes da aspa que fecha.
  assert.equal(digitar("“*vem*", '"'), "“*vem*”");
});

test("aspas simples: apóstrofo depois de letra, abertura depois de espaço", () => {
  assert.equal(digitar("copo d", "'"), "copo d’");
  assert.equal(digitar("disse ", "'"), "disse ‘");
  assert.equal(digitar("disse ‘sim", "'"), "disse ‘sim’");
  assert.equal(digitar("“", "'"), "“‘");
});

test("dois hifens viram travessão, e o terceiro numa linha só de travessão devolve a quebra de cena", () => {
  assert.equal(digitar("-", "-"), "—");
  assert.equal(digitar("Ele parou -", "-"), "Ele parou —");
  // `---` sozinho na linha é a quebra de cena do Markdown: volta a ser hifens.
  assert.equal(digitar("—", "-"), "---");
  assert.equal(digitar("texto —", "-"), "texto —-");
  // Um hífen só é hífen: lista, palavra composta.
  assert.equal(digitar("", "-"), "-");
  assert.equal(digitar("guarda", "-"), "guarda-");
});

test("onde o Markdown precisa do sinal reto, nada é trocado", () => {
  // A linha do bloco `:::` leva aspas retas: `custom-style="Epígrafe"`.
  assert.equal(substituicao("::: {custom-style=", '"'), null);
  assert.equal(substituicao("  :::: {.a custom-style=", "'"), null);
  // Dentro de código em linha.
  assert.equal(substituicao("use `printf(", '"'), null);
  assert.equal(substituicao("use `a-", "-"), null);
  // Fechado o código, volta a valer.
  assert.equal(digitar("use `x` e ", '"'), "use `x` e “");
  // O começo de um comentário HTML (`<!--`) leva os dois hifens.
  assert.equal(substituicao("<!-", "-"), null);
  assert.equal(substituicao("texto <!-", "-"), null);
  // Linha de tabela: os hifens são da tabela.
  assert.equal(substituicao("| a | b |\n|-".split("\n")[1] ?? "", "-"), null);
});

test("o que não é aspa nem hífen, ou vem colado em bloco, fica como foi digitado", () => {
  assert.equal(substituicao("texto", "a"), null);
  assert.equal(substituicao("texto", '""'), null);
  assert.equal(substituicao("texto", ""), null);
});
