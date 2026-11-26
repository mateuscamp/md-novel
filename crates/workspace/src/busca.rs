//! Buscar no livro inteiro (ADR 0003, "Buscar no livro"): o autor nem
//! sempre sabe em que capítulo está o que procura. Só lê; nada é gravado.
//!
//! A comparação não distingue maiúscula nem acento, e é feita aqui, sem
//! dependência: minúsculas pelo Unicode, e os acentos do português e das
//! línguas vizinhas (as letras acentuadas do Latin-1 e os acentos soltos,
//! que alguns programas gravam separados da letra).

use std::ops::Range;

use crate::erro::ErroWorkspace;
use crate::livro::Livro;
use crate::texto::titulo_do_texto;

/// O teto de achados de uma busca no livro. Uma lista maior do que isto
/// ninguém percorre, e uma letra só num romance daria dezenas de milhares.
pub const MAXIMO_DE_ACHADOS: usize = 500;
/// Quanto da linha aparece em volta do achado, em caracteres.
const CONTEXTO_ANTES: usize = 40;
const CONTEXTO_DEPOIS: usize = 60;

/// Um lugar do capítulo onde o termo aparece.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Achado {
    /// Onde começa e acaba no texto do capítulo como o editor o recebe (LF,
    /// sem marca de ordem de bytes), em unidades de UTF-16, que é como o
    /// editor conta as posições.
    pub inicio: usize,
    pub fim: usize,
    /// A linha em volta, para a lista: o que vem antes, o achado como está
    /// escrito e o que vem depois. Cortada com reticências.
    pub antes: String,
    pub trecho: String,
    pub depois: String,
}

/// Os achados de um capítulo, na ordem do texto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AchadosNoCapitulo {
    pub nome: String,
    pub titulo: Option<String>,
    pub achados: Vec<Achado>,
}

/// O resultado de uma busca no livro: só os capítulos com achado, na ordem
/// do livro.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Busca {
    pub capitulos: Vec<AchadosNoCapitulo>,
    /// Havia mais achados do que `MAXIMO_DE_ACHADOS`: a lista parou ali.
    pub cortada: bool,
}

/// A letra sem acento, para as acentuadas do Latin-1 (já em minúscula).
fn sem_acento(letra: char) -> char {
    match letra {
        'à'..='å' => 'a',
        'ç' => 'c',
        'è'..='ë' => 'e',
        'ì'..='ï' => 'i',
        'ñ' => 'n',
        'ò'..='ö' => 'o',
        'ù'..='ü' => 'u',
        'ý' | 'ÿ' => 'y',
        outra => outra,
    }
}

/// O acento escrito à parte da letra (`e` + U+0301).
fn acento_solto(letra: char) -> bool {
    ('\u{300}'..='\u{36f}').contains(&letra)
}

/// As letras como a busca as compara, cada uma com o índice do caractere do
/// texto de onde veio. O acento solto some: fica com a letra antes dele.
/// O índice cabe em 32 bits (um capítulo tem no máximo 16 MiB), e assim a
/// busca gasta 12 bytes por caractere em vez de 28.
fn dobrar(texto: &[char]) -> Vec<(char, u32)> {
    let mut dobrado = Vec::with_capacity(texto.len());
    for (indice, &letra) in texto.iter().enumerate() {
        let Ok(indice) = u32::try_from(indice) else {
            // Além de 4 bilhões de caracteres não há capítulo: o resto fica sem busca.
            break;
        };
        for minuscula in letra.to_lowercase() {
            if !acento_solto(minuscula) {
                dobrado.push((sem_acento(minuscula), indice));
            }
        }
    }
    dobrado
}

/// Os lugares de `texto` onde `termo` aparece, sem diferenciar maiúscula nem
/// acento, sem sobreposição, até `limite`. Um termo vazio ou só de espaços
/// não acha nada.
pub fn buscar_no_texto(texto: &str, termo: &str, limite: usize) -> Vec<Achado> {
    let letras: Vec<char> = texto.chars().collect();
    let dobrado = dobrar(&letras);
    let procurado: Vec<char> = dobrar(&termo.chars().collect::<Vec<_>>())
        .into_iter()
        .map(|(letra, _)| letra)
        .collect();
    let mut achados = Vec::new();
    if termo.trim().is_empty() || procurado.is_empty() {
        return achados;
    }
    // A posição em UTF-16 só é contada até os achados, que vêm em ordem:
    // `contado` caracteres do texto somam `unidades`.
    let (mut contado, mut unidades) = (0, 0);
    let mut em_utf16 = |ate: usize| {
        // Fora de ordem não deve acontecer; se acontecer, reconta do começo
        // em vez de fatiar ao contrário.
        if ate < contado {
            (contado, unidades) = (0, 0);
        }
        unidades += letras[contado..ate]
            .iter()
            .map(|letra| letra.len_utf16())
            .sum::<usize>();
        contado = ate;
        unidades
    };

    let mut lugar = 0;
    while achados.len() < limite && lugar + procurado.len() <= dobrado.len() {
        let bate = dobrado[lugar..lugar + procurado.len()]
            .iter()
            .zip(&procurado)
            .all(|((letra, _), procurada)| letra == procurada);
        if !bate {
            lugar += 1;
            continue;
        }
        let inicio = dobrado[lugar].1 as usize;
        // Até depois do último caractere, levando junto os acentos soltos dele.
        let mut fim = dobrado[lugar + procurado.len() - 1].1 as usize + 1;
        while letras.get(fim).is_some_and(|&letra| acento_solto(letra)) {
            fim += 1;
        }
        let (inicio_em_utf16, fim_em_utf16) = (em_utf16(inicio), em_utf16(fim));
        achados.push(achado(&letras, inicio..fim, inicio_em_utf16..fim_em_utf16));
        lugar += procurado.len();
    }
    achados
}

/// O achado com a linha em volta dele. `lugar` é em caracteres, e
/// `no_editor`, o mesmo trecho em unidades de UTF-16.
fn achado(letras: &[char], lugar: Range<usize>, no_editor: Range<usize>) -> Achado {
    let (inicio, fim) = (lugar.start, lugar.end);
    let comeco_da_linha = letras[..inicio]
        .iter()
        .rposition(|&letra| letra == '\n')
        .map_or(0, |quebra| quebra + 1);
    let fim_da_linha = letras[fim..]
        .iter()
        .position(|&letra| letra == '\n')
        .map_or(letras.len(), |quebra| fim + quebra);
    let de = inicio.saturating_sub(CONTEXTO_ANTES).max(comeco_da_linha);
    let ate = (fim + CONTEXTO_DEPOIS).min(fim_da_linha);
    let mut antes: String = letras[de..inicio].iter().collect();
    if de > comeco_da_linha {
        antes.insert(0, '…');
    }
    let mut depois: String = letras[fim..ate].iter().collect();
    if ate < fim_da_linha {
        depois.push('…');
    }
    Achado {
        inicio: no_editor.start,
        fim: no_editor.end,
        antes,
        trecho: letras[inicio..fim].iter().collect(),
        depois,
    }
}

impl Livro {
    /// Procura `termo` em todos os capítulos, na ordem do livro, no texto
    /// como o editor o recebe. Para em `MAXIMO_DE_ACHADOS`. Capítulo que não
    /// dá para ler como capítulo fica de fora: o motivo aparece quando o
    /// autor o abre.
    pub fn buscar(&self, termo: &str) -> Result<Busca, ErroWorkspace> {
        let mut busca = Busca {
            capitulos: Vec::new(),
            cortada: false,
        };
        let mut restam = MAXIMO_DE_ACHADOS;
        for nome in self.capitulos()? {
            let Ok(capitulo) = self.ler_da_lista(&nome) else {
                continue;
            };
            // Um a mais do que cabe: é como se sabe que havia mais.
            let mut achados = buscar_no_texto(capitulo.texto(), termo, restam + 1);
            if achados.len() > restam {
                achados.truncate(restam);
                busca.cortada = true;
            }
            restam -= achados.len();
            if !achados.is_empty() {
                busca.capitulos.push(AchadosNoCapitulo {
                    titulo: titulo_do_texto(capitulo.texto()),
                    nome,
                    achados,
                });
            }
            if busca.cortada {
                break;
            }
        }
        Ok(busca)
    }
}
