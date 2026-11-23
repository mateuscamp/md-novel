// O modo foco: só a folha na tela. Aqui fica a regra das teclas, pura.

/** A tecla que liga e desliga o modo foco (a mesma do novelWriter). */
export const TECLA_DO_FOCO = "F8";

/** O que uma tecla faz com o modo foco, ou `null` se não é com ele. Fora do
 *  modo foco o Esc não é nosso: o editor usa para desfazer a seleção. E no
 *  meio da composição de um acento (`´` e depois `a`) a tecla é de quem está
 *  compondo: o Esc ali cancela o acento, não sai do foco. */
export function teclaDoFoco(tecla: string, focoLigado: boolean, compondo = false): "alternar" | "sair" | null {
  if (compondo) return null;
  if (tecla === TECLA_DO_FOCO) return "alternar";
  if (tecla === "Escape" && focoLigado) return "sair";
  return null;
}
