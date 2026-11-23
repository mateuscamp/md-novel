//! Lado Word do compilador: reescreve o `word/styles.xml` do reference.docx
//! do Pandoc com as mesmas propriedades que vão para o CSS.

use std::io::{Cursor, Read, Write};

use quick_xml::events::{BytesEnd, BytesStart, Event};
use quick_xml::{Reader, Writer};
use zip::write::SimpleFileOptions;
use zip::{DateTime, ZipArchive, ZipWriter};

use crate::erro::ErroEstilo;
use crate::modelo::{Alinhamento, Estilo, EstiloParagrafo, chave_de_colisao};

const CAMINHO_DOS_ESTILOS: &str = "word/styles.xml";
/// O styles.xml do Pandoc tem cerca de 20 kB. O teto evita que um modelo
/// adulterado (bomba de zip) estoure a memória.
const TAMANHO_MAXIMO_DO_STYLES: u64 = 4 * 1024 * 1024;
/// Estilo que o Pandoc aplica em todo parágrafo comum.
const ID_DO_CORPO: &str = "BodyText";

/// Gera o reference.docx a partir do modelo padrão do Pandoc
/// (`pandoc --print-default-data-file reference.docx`, obtido por `crates/exportar`).
///
/// O corpo vira o estilo "Body Text", que o Pandoc usa em todo parágrafo comum.
/// Cada estilo nomeado vira um estilo de parágrafo com o mesmo nome, que o
/// Pandoc aplica em `::: {custom-style="Nome"}`. O resto do modelo é copiado
/// sem recompressão.
pub fn gerar_reference_docx(modelo: &[u8], estilo: &Estilo) -> Result<Vec<u8>, ErroEstilo> {
    let mut entrada = ZipArchive::new(Cursor::new(modelo))?;
    let mut saida = ZipWriter::new(Cursor::new(Vec::new()));
    let mut achou_styles = false;
    for indice in 0..entrada.len() {
        let arquivo = entrada.by_index_raw(indice)?;
        if arquivo.name()? != CAMINHO_DOS_ESTILOS {
            saida.raw_copy_file(arquivo)?;
            continue;
        }
        drop(arquivo);
        let styles = ler_styles(&mut entrada, indice)?;
        let novo = reescrever_styles(&styles, estilo)?;
        // Data fixa: gerar duas vezes devolve os mesmos bytes.
        let opcoes = SimpleFileOptions::default().last_modified_time(DateTime::default());
        saida.start_file(CAMINHO_DOS_ESTILOS, opcoes)?;
        saida.write_all(&novo)?;
        achou_styles = true;
    }
    if !achou_styles {
        return Err(inesperado("o zip não tem word/styles.xml"));
    }
    Ok(saida.finish()?.into_inner())
}

fn ler_styles(
    entrada: &mut ZipArchive<Cursor<&[u8]>>,
    indice: usize,
) -> Result<String, ErroEstilo> {
    let mut styles = String::new();
    entrada
        .by_index(indice)?
        .take(TAMANHO_MAXIMO_DO_STYLES + 1)
        .read_to_string(&mut styles)?;
    if styles.len() as u64 > TAMANHO_MAXIMO_DO_STYLES {
        return Err(inesperado("word/styles.xml passa de 4 MiB"));
    }
    Ok(styles)
}

fn inesperado(motivo: &str) -> ErroEstilo {
    ErroEstilo::ModeloInesperado {
        motivo: motivo.to_owned(),
    }
}

/// Copia o styles.xml evento por evento, trocando a formatação do corpo e
/// acrescentando os estilos nomeados antes de `</w:styles>`.
fn reescrever_styles(styles: &str, estilo: &Estilo) -> Result<Vec<u8>, ErroEstilo> {
    let mut leitor = Reader::from_str(styles);
    let mut escritor = Writer::new(Vec::new());
    // Ids e nomes que o modelo já usa, para recusar estilo nomeado que colida.
    let mut existentes: Vec<String> = Vec::new();
    let mut profundidade = 0usize;
    let mut dentro_do_corpo = false;
    let mut achou_corpo = false;
    loop {
        let evento = leitor.read_event()?;
        match &evento {
            Event::Start(inicio) => {
                if profundidade == 0 && inicio.name().as_ref() != "w:styles" {
                    return Err(inesperado("a raiz do styles.xml não é w:styles"));
                }
                if profundidade == 1 && inicio.name().as_ref() == "w:style" {
                    let id = atributo(inicio, "w:styleId")?;
                    dentro_do_corpo = id.as_deref() == Some(ID_DO_CORPO);
                    achou_corpo |= dentro_do_corpo;
                    existentes.extend(id);
                }
                if profundidade == 2 && dentro_do_corpo && e_formatacao(inicio) {
                    // A formatação antiga do corpo sai inteira; a nova entra no fechamento.
                    leitor.read_to_end(inicio.name())?;
                    continue;
                }
                profundidade += 1;
            }
            Event::Empty(vazio) => {
                if profundidade == 2 && vazio.name().as_ref() == "w:name" {
                    let nome = atributo(vazio, "w:val")?;
                    existentes.extend(nome);
                }
                if profundidade == 2 && dentro_do_corpo && e_formatacao(vazio) {
                    continue;
                }
            }
            Event::End(_) => {
                profundidade = profundidade
                    .checked_sub(1)
                    .ok_or_else(|| inesperado("fechamento sem abertura no styles.xml"))?;
                if profundidade == 1 && dentro_do_corpo {
                    escrever_formatacao(&mut escritor, &estilo.corpo)?;
                    dentro_do_corpo = false;
                }
                if profundidade == 0 {
                    if !achou_corpo {
                        return Err(inesperado("não achei o estilo BodyText"));
                    }
                    for (nome, paragrafo) in &estilo.estilos {
                        recusar_colisao(nome, &existentes)?;
                        escrever_estilo_nomeado(&mut escritor, nome, paragrafo)?;
                    }
                }
            }
            Event::Eof => break,
            Event::Text(_)
            | Event::CData(_)
            | Event::Comment(_)
            | Event::Decl(_)
            | Event::PI(_)
            | Event::DocType(_)
            | Event::GeneralRef(_) => {}
        }
        escritor.write_event(evento)?;
    }
    if profundidade != 0 || !achou_corpo {
        return Err(inesperado("styles.xml incompleto ou sem w:styles"));
    }
    Ok(escritor.into_inner())
}

fn e_formatacao(elemento: &BytesStart<'_>) -> bool {
    matches!(elemento.name().as_ref(), "w:pPr" | "w:rPr")
}

fn atributo(elemento: &BytesStart<'_>, nome: &str) -> Result<Option<String>, ErroEstilo> {
    let atributo = elemento
        .try_get_attribute(nome)
        .map_err(quick_xml::Error::from)?;
    Ok(atributo.map(|atributo| atributo.value.into_owned()))
}

fn recusar_colisao(nome: &str, existentes: &[String]) -> Result<(), ErroEstilo> {
    let chave = chave_de_colisao(nome);
    match existentes
        .iter()
        .find(|existente| chave_de_colisao(existente) == chave)
    {
        Some(existente) => Err(ErroEstilo::NomeReservado {
            nome: nome.to_owned(),
            existente: existente.clone(),
        }),
        None => Ok(()),
    }
}

fn escrever_estilo_nomeado(
    escritor: &mut Writer<Vec<u8>>,
    nome: &str,
    paragrafo: &EstiloParagrafo,
) -> Result<(), ErroEstilo> {
    // O Pandoc casa `custom-style` pelo w:name; o id segue a convenção do
    // Word, que é o nome sem espaços.
    let id = nome.replace(' ', "");
    let mut abertura = BytesStart::new("w:style");
    abertura.push_attribute(("w:type", "paragraph"));
    abertura.push_attribute(("w:customStyle", "1"));
    abertura.push_attribute(("w:styleId", id.as_str()));
    escritor.write_event(Event::Start(abertura))?;
    vazio(escritor, "w:name", &[("w:val", nome)])?;
    // Baseado em Normal e não em Body Text: toda propriedade já vem explícita,
    // então não há nada do corpo a herdar por dentro do Word.
    vazio(escritor, "w:basedOn", &[("w:val", "Normal")])?;
    vazio(escritor, "w:qFormat", &[])?;
    escrever_formatacao(escritor, paragrafo)?;
    escritor.write_event(Event::End(BytesEnd::new("w:style")))?;
    Ok(())
}

/// Escreve `w:pPr` e `w:rPr` com todas as propriedades. A ordem dos elementos
/// é a do schema do OOXML (spacing, ind, jc; rFonts, b, i, sz), que o Word exige.
fn escrever_formatacao(
    escritor: &mut Writer<Vec<u8>>,
    paragrafo: &EstiloParagrafo,
) -> Result<(), ErroEstilo> {
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
    let antes = espaco_antes.twips().to_string();
    let depois = espaco_depois.twips().to_string();
    let linha = entrelinha.duzentos_e_quarenta_avos().to_string();
    let recuo = recuo_primeira_linha.twips().to_string();
    let alinhamento = match alinhamento {
        Alinhamento::Esquerda => "left",
        Alinhamento::Centro => "center",
        Alinhamento::Direita => "right",
        Alinhamento::Justificado => "both",
    };
    let fonte = fonte.nome();
    let meios_pontos = tamanho.meios_pontos().to_string();
    let ligado = |valor: bool| if valor { "1" } else { "0" };

    escritor.write_event(Event::Start(BytesStart::new("w:pPr")))?;
    vazio(
        escritor,
        "w:spacing",
        &[
            ("w:before", &antes),
            ("w:after", &depois),
            // lineRule auto: w:line em 240 avos da linha simples da fonte,
            // o "Múltiplos" da caixa de parágrafo do Word.
            ("w:line", &linha),
            ("w:lineRule", "auto"),
        ],
    )?;
    vazio(escritor, "w:ind", &[("w:firstLine", &recuo)])?;
    vazio(escritor, "w:jc", &[("w:val", alinhamento)])?;
    escritor.write_event(Event::End(BytesEnd::new("w:pPr")))?;

    escritor.write_event(Event::Start(BytesStart::new("w:rPr")))?;
    vazio(
        escritor,
        "w:rFonts",
        &[
            ("w:ascii", fonte),
            ("w:hAnsi", fonte),
            ("w:eastAsia", fonte),
            ("w:cs", fonte),
        ],
    )?;
    vazio(escritor, "w:b", &[("w:val", ligado(*negrito))])?;
    vazio(escritor, "w:bCs", &[("w:val", ligado(*negrito))])?;
    vazio(escritor, "w:i", &[("w:val", ligado(*italico))])?;
    vazio(escritor, "w:iCs", &[("w:val", ligado(*italico))])?;
    vazio(escritor, "w:sz", &[("w:val", &meios_pontos)])?;
    vazio(escritor, "w:szCs", &[("w:val", &meios_pontos)])?;
    escritor.write_event(Event::End(BytesEnd::new("w:rPr")))?;
    Ok(())
}

fn vazio(
    escritor: &mut Writer<Vec<u8>>,
    nome: &str,
    atributos: &[(&str, &str)],
) -> Result<(), ErroEstilo> {
    let mut elemento = BytesStart::new(nome);
    for atributo in atributos {
        // `push_attribute` escapa o valor.
        elemento.push_attribute(*atributo);
    }
    escritor.write_event(Event::Empty(elemento))?;
    Ok(())
}
