// O tema da moldura do app: claro, escuro ou o do sistema. Regra pura; quem
// lê o sistema e guarda a escolha é o `main.ts`.

export type Tema = "claro" | "escuro";
export type Preferencia = "sistema" | Tema;

/** A escolha guardada. Valor ausente ou desconhecido volta a seguir o sistema. */
export function lerPreferencia(guardado: string | null): Preferencia {
  return guardado === "claro" || guardado === "escuro" ? guardado : "sistema";
}

/** O tema que vai para a tela. */
export function temaEfetivo(preferencia: Preferencia, sistemaEscuro: boolean): Tema {
  if (preferencia === "sistema") return sistemaEscuro ? "escuro" : "claro";
  return preferencia;
}

/** A ordem do botão de tema: sistema, claro, escuro, e de volta. */
export function proximaPreferencia(preferencia: Preferencia): Preferencia {
  switch (preferencia) {
    case "sistema":
      return "claro";
    case "claro":
      return "escuro";
    case "escuro":
      return "sistema";
  }
}

export function rotuloDaPreferencia(preferencia: Preferencia): string {
  return `Tema: ${preferencia}`;
}
