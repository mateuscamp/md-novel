// Quando salvar: a máquina de estados do salvamento automático, sem tela e
// sem Tauri, para ser testada sozinha. É a parte da interface que pode perder
// texto, então uma gravação por vez e nenhuma decisão escondida.

export type Estado =
  | { tipo: "limpo" }
  | { tipo: "editando" }
  | { tipo: "salvo" }
  | { tipo: "conflito"; mensagem: string }
  | { tipo: "erro"; mensagem: string };

export interface ErroDeGravacao {
  codigo: "erro" | "mudou-por-fora";
  mensagem: string;
}

export interface Dependencias {
  /** Grava o capítulo; rejeita com um `ErroDeGravacao`. */
  gravar(nome: string, texto: string): Promise<unknown>;
  /** O texto do editor agora. */
  texto(): string;
  agendar(tarefa: () => void, ms: number): unknown;
  cancelar(agendamento: unknown): void;
  aoMudarEstado(estado: Estado): void;
}

/** Tempo parado depois da última tecla até salvar sozinho. */
export const ESPERA_PARA_SALVAR_MS = 1500;

export class Gravador {
  private readonly dependencias: Dependencias;
  private capitulo: string | null = null;
  private haMudancas = false;
  private conflito: string | null = null;
  private agendamento: unknown;
  private emCurso: Promise<boolean> | null = null;

  constructor(dependencias: Dependencias) {
    this.dependencias = dependencias;
  }

  /** Passa a acompanhar o capítulo recém-lido do disco, sem mudanças. */
  abrir(nome: string): void {
    this.cancelarAgendamento();
    this.capitulo = nome;
    this.haMudancas = false;
    this.conflito = null;
    this.dependencias.aoMudarEstado({ tipo: "limpo" });
  }

  /** O texto do editor mudou. */
  mudou(): void {
    this.haMudancas = true;
    // Em conflito nada é gravado por cima: só reabrir o capítulo destrava.
    if (this.conflito !== null) return;
    this.dependencias.aoMudarEstado({ tipo: "editando" });
    this.cancelarAgendamento();
    this.agendamento = this.dependencias.agendar(() => void this.salvarAgora(), ESPERA_PARA_SALVAR_MS);
  }

  /** Há texto que ainda não está no disco. */
  get pendente(): boolean {
    return this.haMudancas || this.emCurso !== null;
  }

  /**
   * Salva o que falta e espera. `false` se sobrou texto fora do disco.
   *
   * Uma gravação por vez: quem chama durante uma gravação recebe a mesma
   * promessa, que só resolve quando não há mais nada a gravar. Trocar de
   * capítulo e fechar a janela esperam por ela.
   */
  salvarAgora(): Promise<boolean> {
    this.cancelarAgendamento();
    this.emCurso ??= this.gravarAteLimpar().finally(() => {
      this.emCurso = null;
    });
    return this.emCurso;
  }

  private cancelarAgendamento(): void {
    if (this.agendamento === undefined) return;
    this.dependencias.cancelar(this.agendamento);
    this.agendamento = undefined;
  }

  private async gravarAteLimpar(): Promise<boolean> {
    // O que for digitado durante uma gravação levanta `haMudancas` de novo,
    // e o laço grava outra vez com o texto mais novo.
    while (this.haMudancas) {
      if (this.conflito !== null || this.capitulo === null) return false;
      this.haMudancas = false;
      try {
        await this.dependencias.gravar(this.capitulo, this.dependencias.texto());
      } catch (motivo) {
        this.haMudancas = true;
        const erro = comoErroDeGravacao(motivo);
        if (erro.codigo === "mudou-por-fora") {
          this.conflito = erro.mensagem;
          this.dependencias.aoMudarEstado({ tipo: "conflito", mensagem: erro.mensagem });
        } else {
          this.dependencias.aoMudarEstado({ tipo: "erro", mensagem: erro.mensagem });
        }
        return false;
      }
    }
    if (this.conflito !== null) return false;
    if (this.capitulo !== null) this.dependencias.aoMudarEstado({ tipo: "salvo" });
    return true;
  }
}

function comoErroDeGravacao(motivo: unknown): ErroDeGravacao {
  if (typeof motivo === "object" && motivo !== null && "codigo" in motivo && "mensagem" in motivo) {
    return motivo as ErroDeGravacao;
  }
  return { codigo: "erro", mensagem: motivo instanceof Error ? motivo.message : String(motivo) };
}
