import assert from "node:assert/strict";
import { test } from "node:test";
import { type Dependencias, ESPERA_PARA_SALVAR_MS, type Estado, Gravador } from "./gravador.ts";

/** Um disco de mentira com gravações que o teste termina quando quiser. */
function cenario() {
  const gravacoes: { nome: string; texto: string; terminar: () => void; falhar: (erro: unknown) => void }[] = [];
  const agendados: { tarefa: () => void; ms: number; cancelado: boolean }[] = [];
  const estados: Estado[] = [];
  let texto = "";
  const dependencias: Dependencias = {
    gravar: (nome, textoGravado) =>
      new Promise<void>((terminar, falhar) => gravacoes.push({ nome, texto: textoGravado, terminar, falhar })),
    texto: () => texto,
    agendar: (tarefa, ms) => {
      const agendamento = { tarefa, ms, cancelado: false };
      agendados.push(agendamento);
      return agendamento;
    },
    cancelar: (agendamento) => {
      (agendamento as { cancelado: boolean }).cancelado = true;
    },
    aoMudarEstado: (estado) => estados.push(estado),
  };
  const gravador = new Gravador(dependencias);
  return {
    gravador,
    gravacoes,
    estados,
    digitar(novo: string) {
      texto = novo;
      gravador.mudou();
    },
    /** Dispara o temporizador que estiver valendo, como o relógio faria. */
    passarOTempo() {
      const vivo = agendados.filter((agendamento) => !agendamento.cancelado).at(-1);
      assert.ok(vivo, "esperava um salvamento agendado");
      vivo.cancelado = true;
      vivo.tarefa();
    },
    agendamentosVivos: () => agendados.filter((agendamento) => !agendamento.cancelado).length,
    ultimaEspera: () => agendados.at(-1)?.ms,
  };
}

const pausa = () => new Promise((resolve) => setImmediate(resolve));
const ultimo = (estados: Estado[]) => estados.at(-1)?.tipo;

test("digitar agenda um salvamento, e só o último temporizador vale", async () => {
  const c = cenario();
  c.gravador.abrir("01.md");
  c.digitar("a");
  c.digitar("ab");
  assert.equal(c.agendamentosVivos(), 1);
  assert.equal(c.ultimaEspera(), ESPERA_PARA_SALVAR_MS);
  assert.equal(ultimo(c.estados), "editando");
  assert.equal(c.gravador.pendente, true);

  c.passarOTempo();
  await pausa();
  assert.deepEqual(
    c.gravacoes.map((gravacao) => [gravacao.nome, gravacao.texto]),
    [["01.md", "ab"]],
  );
  c.gravacoes[0]?.terminar();
  await pausa();
  assert.equal(ultimo(c.estados), "salvo");
  assert.equal(c.gravador.pendente, false);
});

test("sem mudança não grava nada", async () => {
  const c = cenario();
  c.gravador.abrir("01.md");
  assert.equal(await c.gravador.salvarAgora(), true);
  assert.equal(c.gravacoes.length, 0);
});

test("o que é digitado durante uma gravação é gravado em seguida, uma gravação por vez", async () => {
  const c = cenario();
  c.gravador.abrir("01.md");
  c.digitar("a");
  const salvando = c.gravador.salvarAgora();
  await pausa();
  c.digitar("ab");
  assert.equal(c.gravacoes.length, 1, "a segunda gravação espera a primeira");
  c.gravacoes[0]?.terminar();
  await pausa();
  assert.equal(c.gravacoes.length, 2);
  assert.equal(c.gravacoes[1]?.texto, "ab");
  assert.equal(c.gravador.pendente, true, "o texto novo ainda não está no disco");
  c.gravacoes[1]?.terminar();
  assert.equal(await salvando, true);
  assert.equal(ultimo(c.estados), "salvo");
  assert.equal(c.gravador.pendente, false);
});

test("salvarAgora espera a gravação em curso em vez de dar o texto por salvo", async () => {
  const c = cenario();
  c.gravador.abrir("01.md");
  c.digitar("a");
  c.passarOTempo();
  await pausa();
  let terminou = false;
  const espera = c.gravador.salvarAgora().then((ok) => {
    terminou = true;
    return ok;
  });
  await pausa();
  assert.equal(terminou, false, "trocar de capítulo ou fechar não pode passar na frente da gravação");
  c.gravacoes[0]?.falhar({ codigo: "mudou-por-fora", mensagem: "mudou no disco" });
  assert.equal(await espera, false, "a gravação falhou: o texto não está no disco");
});

test("conflito para o salvamento automático até o capítulo ser reaberto", async () => {
  const c = cenario();
  c.gravador.abrir("01.md");
  c.digitar("a");
  const primeira = c.gravador.salvarAgora();
  await pausa();
  c.gravacoes[0]?.falhar({ codigo: "mudou-por-fora", mensagem: "mudou no disco" });
  assert.equal(await primeira, false);
  assert.deepEqual(c.estados.at(-1), { tipo: "conflito", mensagem: "mudou no disco" });

  c.digitar("ab");
  assert.equal(c.agendamentosVivos(), 0, "em conflito não se agenda gravação");
  assert.equal(await c.gravador.salvarAgora(), false);
  assert.equal(c.gravacoes.length, 1, "nem se tenta gravar por cima");
  assert.equal(ultimo(c.estados), "conflito");

  c.gravador.abrir("01.md");
  assert.equal(c.gravador.pendente, false);
  c.digitar("texto do disco, editado");
  assert.equal(c.agendamentosVivos(), 1);
});

test("erro comum deixa o texto pendente e a próxima tentativa grava", async () => {
  const c = cenario();
  c.gravador.abrir("01.md");
  c.digitar("a");
  const primeira = c.gravador.salvarAgora();
  await pausa();
  c.gravacoes[0]?.falhar({ codigo: "erro", mensagem: "disco cheio" });
  assert.equal(await primeira, false);
  assert.deepEqual(c.estados.at(-1), { tipo: "erro", mensagem: "disco cheio" });
  assert.equal(c.gravador.pendente, true);

  const segunda = c.gravador.salvarAgora();
  await pausa();
  assert.equal(c.gravacoes.length, 2);
  c.gravacoes[1]?.terminar();
  assert.equal(await segunda, true);
});

test("erro que não veio no formato do app também vira mensagem", async () => {
  const c = cenario();
  c.gravador.abrir("01.md");
  c.digitar("a");
  const salvando = c.gravador.salvarAgora();
  await pausa();
  c.gravacoes[0]?.falhar(new Error("ponte caiu"));
  assert.equal(await salvando, false);
  assert.equal(ultimo(c.estados), "erro");
});

test("abrir outro capítulo cancela o temporizador e zera o estado", () => {
  const c = cenario();
  c.gravador.abrir("01.md");
  c.digitar("a");
  c.gravador.abrir("02.md");
  assert.equal(c.agendamentosVivos(), 0);
  assert.equal(c.gravador.pendente, false);
  assert.equal(ultimo(c.estados), "limpo");
});
