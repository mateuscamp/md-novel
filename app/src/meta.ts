// As palavras escritas hoje em cada livro, e a meta do dia. A conta é o
// total do livro agora menos o total de quando o dia começou; aqui ficam as
// regras, puras.

import { numero } from "./resumo.ts";

export const META_MAXIMA = 100_000;

/** O dia no relógio do autor, `AAAA-MM-DD`. */
export function diaDe(data: Date): string {
  const dois = (valor: number) => String(valor).padStart(2, "0");
  return `${data.getFullYear()}-${dois(data.getMonth() + 1)}-${dois(data.getDate())}`;
}

export function escreverHoje(dia: string, base: number): string {
  return JSON.stringify({ dia, base });
}

/** O total de palavras de onde a conta de hoje parte: o guardado, se é do
 *  mesmo dia; senão o total de agora (dia novo, ou registro estragado). */
export function lerHoje(guardado: string | null, dia: string, total: number): number {
  if (guardado === null) return total;
  let lido: unknown;
  try {
    lido = JSON.parse(guardado);
  } catch {
    return total;
  }
  if (typeof lido !== "object" || lido === null) return total;
  const registro = lido as Record<string, unknown>;
  const base = registro.base;
  const valida = typeof base === "number" && Number.isSafeInteger(base) && base >= 0;
  return registro.dia === dia && valida ? base : total;
}

/** A meta digitada ou guardada: um inteiro de 1 a `META_MAXIMA`. */
export function lerMeta(guardado: string | null): number | null {
  const texto = guardado?.trim() ?? "";
  if (!/^\d{1,6}$/.test(texto)) return null;
  const meta = Number(texto);
  return meta >= 1 && meta <= META_MAXIMA ? meta : null;
}

export function rotuloDeHoje(escritas: number, meta: number | null): string {
  if (meta === null) return `Hoje: ${numero(escritas)} ${escritas === 1 ? "palavra" : "palavras"}`;
  return `Hoje: ${numero(escritas)} de ${numero(meta)} palavras${escritas >= meta ? " ✓" : ""}`;
}
