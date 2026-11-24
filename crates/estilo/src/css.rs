//! Lado CSS do compilador: as mesmas propriedades que vão para o Word.

use std::fmt::Write as _;

use crate::erro::ErroEstilo;
use crate::modelo::{Alinhamento, Estilo, EstiloParagrafo};

const CABECALHO: &str =
    "/* Gerado pelo md Novel a partir do estilo.yaml. Não edite aqui: mude o estilo.yaml. */\n";

/// Seletor CSS dos parágrafos do corpo. No editor do md Novel é
/// `.md-novel-corpo`; em outro editor (Zettlr), o seletor da linha de texto dele.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeletorCss(String);

impl SeletorCss {
    /// Aceita um seletor ou lista de seletores (`.a, .b`). Recusa o que
    /// escaparia da regra ou a deixaria muda: chaves, ponto e vírgula,
    /// comentário, `<`, barra invertida e aspas ou parênteses sem par.
    pub fn novo(texto: &str) -> Result<Self, ErroEstilo> {
        let seletor = texto.trim();
        let proibido = |c: char| c.is_control() || matches!(c, '{' | '}' | ';' | '<' | '\\');
        let valido = !seletor.is_empty()
            && !seletor.chars().any(proibido)
            && !seletor.contains("/*")
            && !seletor.contains("*/")
            && pares_fechados(seletor);
        if !valido {
            return Err(ErroEstilo::SeletorInvalido {
                recebido: texto.to_owned(),
            });
        }
        Ok(SeletorCss(seletor.to_owned()))
    }
}

/// Aspas, parênteses e colchetes abrem e fecham na ordem. O seletor vai
/// dentro de `:is(...)`, então um `)` solto fecharia o `:is` antes da hora.
fn pares_fechados(seletor: &str) -> bool {
    let mut abertos: Vec<char> = Vec::new();
    let mut aspas: Option<char> = None;
    for c in seletor.chars() {
        match (aspas, c) {
            (Some(aberta), _) if c == aberta => aspas = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => aspas = Some(c),
            (None, '(' | '[') => abertos.push(c),
            (None, ')') if abertos.pop() != Some('(') => return false,
            (None, ']') if abertos.pop() != Some('[') => return false,
            (None, _) => {}
        }
    }
    aspas.is_none() && abertos.is_empty()
}

impl Default for SeletorCss {
    fn default() -> Self {
        SeletorCss(".md-novel-corpo".to_owned())
    }
}

/// Gera o CSS do editor.
///
/// O corpo usa `seletor_corpo`. Cada estilo nomeado usa
/// `:is(<seletor do corpo>)[data-md-novel-estilo="Nome"]`: o editor põe esse
/// atributo no mesmo elemento que casa com o seletor do corpo, em cada
/// parágrafo do bloco (não no contêiner), porque o Word aplica o espaçamento
/// por parágrafo.
pub fn gerar_css(estilo: &Estilo, seletor_corpo: &SeletorCss) -> String {
    let mut css = String::from(CABECALHO);
    escrever_regra(&mut css, &seletor_corpo.0, &estilo.corpo);
    for (nome, paragrafo) in &estilo.estilos {
        // As duas regras trazem todas as propriedades, então a nomeada precisa
        // ser mais específica que a do corpo, seja qual for o seletor dele:
        // `:is(corpo)` vale o mesmo que o corpo, e o atributo soma por cima.
        // O nome foi validado na leitura (letras, números, espaço e hífen) e
        // cabe na string CSS sem escape.
        let seletor = format!(
            ":is({corpo})[data-md-novel-estilo=\"{nome}\"]",
            corpo = seletor_corpo.0
        );
        escrever_regra(&mut css, &seletor, paragrafo);
    }
    css
}

fn escrever_regra(css: &mut String, seletor: &str, paragrafo: &EstiloParagrafo) {
    // Sem `..`: propriedade nova quebra a compilação aqui até ser traduzida.
    let EstiloParagrafo {
        fonte,
        tamanho,
        entrelinha,
        alinhamento,
        recuo_primeira_linha,
        espaco_antes,
        espaco_depois,
        italico,
        negrito,
    } = paragrafo;
    let alinhamento = match alinhamento {
        Alinhamento::Esquerda => "left",
        Alinhamento::Centro => "center",
        Alinhamento::Direita => "right",
        Alinhamento::Justificado => "justify",
    };
    let italico = if *italico { "italic" } else { "normal" };
    let negrito = if *negrito { "bold" } else { "normal" };
    // A entrelinha do Word é múltiplo da linha simples DA FONTE (cerca de 1.2
    // do corpo), não do corpo como no CSS. O editor mede a fonte e define
    // --md-novel-linha-simples; sem isso vale 1.2 (ADR 0002).
    // Espaço antes e depois vão em padding, não em margin: margens de blocos
    // vizinhos se fundem no CSS (vale a maior), e o Word soma as duas.
    // Escrever em String não falha, então o resultado do `write!` é descartado.
    let _ = write!(
        css,
        "\n{seletor} {{\n  \
         font-family: \"{fonte}\";\n  \
         font-size: {tamanho};\n  \
         line-height: calc(var(--md-novel-linha-simples, 1.2) * {entrelinha});\n  \
         text-align: {alinhamento};\n  \
         text-indent: {recuo_primeira_linha};\n  \
         padding-top: {espaco_antes};\n  \
         padding-bottom: {espaco_depois};\n  \
         font-style: {italico};\n  \
         font-weight: {negrito};\n}}\n",
        fonte = fonte.nome(),
        tamanho = tamanho.medida(),
        entrelinha = entrelinha.multiplo(),
    );
}
