// Conforto de quem escreve: zoom do texto, esconder a lateral, máquina de
// escrever e tela cheia. Aqui ficam as regras, puras; quem liga na janela é
// o `main.ts`.

export const ZOOM_PADRAO = 100;
/** A faixa do zoom, em por cento. */
export const ZOOM_MINIMO = 50;
export const ZOOM_MAXIMO = 300;
const PASSO_DO_ZOOM = 10;

export type AcaoDeZoom = "aumentar" | "diminuir" | "restaurar";

/** O zoom guardado, se for um inteiro da faixa escrito só com algarismos. */
export function lerZoom(guardado: string | null): number {
  if (guardado === null || !/^\d{2,3}$/.test(guardado)) return ZOOM_PADRAO;
  const zoom = Number(guardado);
  return zoom >= ZOOM_MINIMO && zoom <= ZOOM_MAXIMO ? zoom : ZOOM_PADRAO;
}

export function zoomDepoisDe(atual: number, acao: AcaoDeZoom): number {
  switch (acao) {
    case "aumentar":
      return Math.min(ZOOM_MAXIMO, atual + PASSO_DO_ZOOM);
    case "diminuir":
      return Math.max(ZOOM_MINIMO, atual - PASSO_DO_ZOOM);
    case "restaurar":
      return ZOOM_PADRAO;
  }
}

export function rotuloDoZoom(zoom: number): string {
  return `${zoom}%`;
}

/** O valor de `--md-novel-zoom`, por onde o CSS do livro e a folha multiplicam
 *  as medidas do texto. */
export function fatorDoZoom(zoom: number): string {
  return String(zoom / 100);
}

/** Ctrl+rolagem: para cima aproxima, para baixo afasta. Sem Ctrl a rolagem
 *  é do texto. */
export function acaoDaRoda(deltaY: number, ctrl: boolean): AcaoDeZoom | null {
  if (!ctrl || deltaY === 0) return null;
  return deltaY < 0 ? "aumentar" : "diminuir";
}

export type Atalho =
  | "zoom-aumentar"
  | "zoom-diminuir"
  | "zoom-restaurar"
  | "lateral"
  | "maquina"
  | "tela-cheia"
  | "buscar-no-livro";

/** As teclas de função ao lado da do modo foco (F8), e o zoom dos navegadores. */
export const TECLA_DA_LATERAL = "F9";
export const TECLA_DA_MAQUINA = "F10";
export const TECLA_DA_TELA_CHEIA = "F11";
/** Com Ctrl e Shift, como nos editores de código: a busca em todos os capítulos. */
const TECLA_DA_BUSCA_NO_LIVRO = "f";

/** O que uma tecla faz com a janela, ou `null` se ela é do texto. No meio da
 *  composição de um acento a tecla é de quem está compondo. */
export function atalhoDaTecla(tecla: string, ctrl: boolean, compondo = false, shift = false): Atalho | null {
  if (compondo) return null;
  if (ctrl) {
    // Ctrl+F sozinho é do editor: a busca dentro do capítulo.
    if (shift && tecla.toLowerCase() === TECLA_DA_BUSCA_NO_LIVRO) return "buscar-no-livro";
    if (tecla === "=" || tecla === "+") return "zoom-aumentar";
    if (tecla === "-") return "zoom-diminuir";
    if (tecla === "0") return "zoom-restaurar";
    return null;
  }
  if (tecla === TECLA_DA_LATERAL) return "lateral";
  if (tecla === TECLA_DA_MAQUINA) return "maquina";
  if (tecla === TECLA_DA_TELA_CHEIA) return "tela-cheia";
  return null;
}

/** O valor guardado de uma escolha de liga e desliga. */
export const LIGADO = "sim";
export const DESLIGADO = "nao";

export function lerLigado(guardado: string | null): boolean {
  return guardado === LIGADO;
}

/** Para o que vem ligado de fábrica: só o "não" guardado desliga. */
export function lerDesligado(guardado: string | null): boolean {
  return guardado === DESLIGADO;
}
