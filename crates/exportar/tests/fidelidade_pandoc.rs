//! Fitness function do driver principal (fidelidade editor ↔ .docx), com o
//! Pandoc de verdade: o que o estilo.yaml declara precisa estar no .docx que
//! o Pandoc escreve, aplicado aos parágrafos certos.
//!
//! Precisa do `pandoc` no PATH. Sem ele o teste falha; não é pulado.

use std::fs;
use std::io::{Cursor, Read};
use std::path::Path;

use estilo::{Estilo, gerar_reference_docx};
use exportar::{exportar_docx, modelo_reference_docx};
use zip::ZipArchive;

fn entrada(docx: &[u8], nome: &str) -> String {
    let mut zip = ZipArchive::new(Cursor::new(docx)).expect("o .docx exportado é um zip");
    let mut texto = String::new();
    zip.by_name(nome)
        .unwrap_or_else(|erro| panic!("falta {nome} no .docx: {erro}"))
        .read_to_string(&mut texto)
        .expect("a entrada é UTF-8");
    texto
}

/// O bloco `<w:style ...>` cujo `w:name` é o nome dado, e o `w:styleId` dele.
fn estilo_por_nome<'a>(styles: &'a str, nome: &str) -> (&'a str, String) {
    let bloco = styles
        .split("<w:style ")
        .skip(1)
        .map(|resto| {
            resto
                .split("</w:style>")
                .next()
                .expect("fechamento do estilo")
        })
        .find(|bloco| bloco.contains(&format!("<w:name w:val=\"{nome}\"")))
        .unwrap_or_else(|| panic!("o .docx não tem o estilo {nome:?}"));
    let id = bloco
        .split("w:styleId=\"")
        .nth(1)
        .and_then(|resto| resto.split('"').next())
        .expect("w:styleId do estilo");
    (bloco, id.to_owned())
}

/// O `w:pStyle` do parágrafo que contém o texto.
fn estilo_do_paragrafo(document: &str, texto: &str) -> String {
    let paragrafo = document
        .split("<w:p>")
        .find(|paragrafo| paragrafo.contains(texto))
        .unwrap_or_else(|| panic!("não achei o parágrafo com {texto:?}"));
    paragrafo
        .split("<w:pStyle w:val=\"")
        .nth(1)
        .and_then(|resto| resto.split('"').next())
        .unwrap_or_else(|| panic!("parágrafo com {texto:?} sem w:pStyle"))
        .to_owned()
}

fn exige(bloco: &str, marcas: &[&str]) {
    for marca in marcas {
        assert!(bloco.contains(marca), "falta {marca} em:\n{bloco}");
    }
}

#[test]
fn o_docx_exportado_tem_as_propriedades_do_estilo_yaml_nos_paragrafos_certos() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    // A mesma fixture do crate estilo: uma cópia aqui poderia divergir.
    let yaml = fs::read_to_string(fixtures.join("../../../estilo/tests/fixtures/romance.yaml"))
        .expect("ler romance.yaml");
    let estilo = Estilo::de_yaml(&yaml).expect("romance.yaml é válido");
    let modelo = modelo_reference_docx().expect("pedir o modelo ao pandoc");
    let reference = gerar_reference_docx(&modelo, &estilo).expect("gerar o reference.docx");

    let pasta = tempfile::tempdir().expect("pasta temporária");
    let saida = pasta.path().join("livro.docx");
    exportar_docx(&[fixtures.join("capitulo.md")], &reference, &saida).expect("exportar");

    let docx = fs::read(&saida).expect("ler o .docx exportado");
    let styles = entrada(&docx, "word/styles.xml");
    let document = entrada(&docx, "word/document.xml");

    let (corpo, id_corpo) = estilo_por_nome(&styles, "Body Text");
    exige(
        corpo,
        &[
            r#"w:before="0""#,
            r#"w:after="0""#,
            r#"w:line="360""#,
            r#"w:lineRule="auto""#,
            r#"<w:ind w:firstLine="709""#,
            r#"<w:jc w:val="both""#,
            r#"w:ascii="EB Garamond""#,
            r#"<w:b w:val="0""#,
            r#"<w:i w:val="0""#,
            r#"<w:sz w:val="24""#,
        ],
    );
    assert_eq!(
        estilo_do_paragrafo(&document, "Segundo parágrafo"),
        id_corpo
    );

    let (epigrafe, id_epigrafe) = estilo_por_nome(&styles, "Epígrafe");
    exige(
        epigrafe,
        &[
            r#"w:after="480""#,
            r#"<w:ind w:firstLine="0""#,
            r#"<w:jc w:val="right""#,
            r#"<w:i w:val="1""#,
        ],
    );
    assert_eq!(estilo_do_paragrafo(&document, "A pele guarda"), id_epigrafe);

    let (placa, id_placa) = estilo_por_nome(&styles, "Placa de rua");
    exige(
        placa,
        &[
            r#"<w:jc w:val="center""#,
            r#"<w:b w:val="1""#,
            r#"w:before="240""#,
        ],
    );
    assert_eq!(estilo_do_paragrafo(&document, "RUA DO SOL"), id_placa);

    // O parágrafo logo depois de um título usa "First Paragraph", que herda
    // do corpo no modelo do Pandoc. Se o Pandoc mudar essa herança, a
    // abertura de cada capítulo sai fora do estilo sem aviso.
    let (primeiro, id_primeiro) = estilo_por_nome(&styles, "First Paragraph");
    exige(primeiro, &[&format!("<w:basedOn w:val=\"{id_corpo}\"")]);
    assert_eq!(
        estilo_do_paragrafo(&document, "Abertura do segundo"),
        id_primeiro
    );
    // Depois de um bloco com estilo nomeado, o parágrafo volta ao corpo.
    assert_eq!(
        estilo_do_paragrafo(&document, "Primeiro parágrafo"),
        id_corpo
    );

    let sobras: Vec<_> = fs::read_dir(pasta.path())
        .expect("listar a pasta de saída")
        .map(|entrada| entrada.expect("entrada da pasta").file_name())
        .collect();
    assert_eq!(
        sobras,
        ["livro.docx"],
        "a exportação não pode deixar temporário na pasta"
    );
}
