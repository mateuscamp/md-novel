//! Testes do compilador pela API pública: o que o `estilo.yaml` declara
//! precisa sair igual no CSS e no styles.xml do reference.docx.

use std::collections::BTreeSet;
use std::io::{Cursor, Read, Write};

use estilo::{ErroEstilo, Estilo, SeletorCss, gerar_css, gerar_reference_docx};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

const ROMANCE: &str = include_str!("fixtures/romance.yaml");

/// styles.xml reduzido, no formato do modelo do Pandoc 3.x. O modelo real é
/// exercitado em `crates/exportar/tests/fidelidade_pandoc.rs`.
const STYLES_DO_MODELO: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:docDefaults>
    <w:pPrDefault>
      <w:pPr>
        <w:spacing w:after="200" />
      </w:pPr>
    </w:pPrDefault>
  </w:docDefaults>
  <w:style w:type="paragraph" w:default="1" w:styleId="Normal">
    <w:name w:val="Normal" />
    <w:qFormat />
  </w:style>
  <w:style w:type="paragraph" w:styleId="BodyText">
    <w:name w:val="Body Text" />
    <w:basedOn w:val="Normal" />
    <w:link w:val="BodyTextChar" />
    <w:qFormat />
    <w:pPr>
      <w:spacing w:before="180" w:after="180" />
    </w:pPr>
    <w:rPr>
      <w:color w:val="FF0000" />
    </w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:customStyle="1" w:styleId="FirstParagraph">
    <w:name w:val="First Paragraph" />
    <w:basedOn w:val="BodyText" />
    <w:next w:val="BodyText" />
    <w:qFormat />
  </w:style>
  <w:style w:type="paragraph" w:styleId="Title">
    <w:name w:val="Title" />
    <w:basedOn w:val="Normal" />
    <w:pPr>
      <w:jc w:val="center" />
    </w:pPr>
  </w:style>
</w:styles>
"#;

const DOCUMENT_DO_MODELO: &str = "<w:document>conteúdo que não pode mudar</w:document>";

fn romance() -> Estilo {
    Estilo::de_yaml(ROMANCE).expect("a fixture romance.yaml é válida")
}

fn modelo_com(styles: &str) -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let opcoes = SimpleFileOptions::default();
    for (nome, conteudo) in [
        ("word/document.xml", DOCUMENT_DO_MODELO),
        ("word/styles.xml", styles),
    ] {
        zip.start_file(nome, opcoes)
            .expect("abrir entrada do zip de teste");
        zip.write_all(conteudo.as_bytes())
            .expect("escrever entrada do zip de teste");
    }
    zip.finish().expect("fechar zip de teste").into_inner()
}

fn modelo() -> Vec<u8> {
    modelo_com(STYLES_DO_MODELO)
}

fn entrada(docx: &[u8], nome: &str) -> String {
    let mut zip = ZipArchive::new(Cursor::new(docx)).expect("o docx gerado é um zip");
    let mut texto = String::new();
    zip.by_name(nome)
        .unwrap_or_else(|erro| panic!("falta {nome} no docx: {erro}"))
        .read_to_string(&mut texto)
        .expect("a entrada é UTF-8");
    texto
}

fn styles_gerado() -> String {
    let docx = gerar_reference_docx(&modelo(), &romance()).expect("gerar o reference.docx");
    entrada(&docx, "word/styles.xml")
}

/// O bloco `<w:style ...>` com o `w:styleId` dado.
fn bloco_do_estilo<'a>(styles: &'a str, id: &str) -> &'a str {
    let marca = format!("w:styleId=\"{id}\"");
    let posicao = styles
        .find(&marca)
        .unwrap_or_else(|| panic!("styles.xml sem o estilo {id}:\n{styles}"));
    let inicio = styles[..posicao]
        .rfind("<w:style ")
        .expect("abertura do estilo");
    let fim = posicao
        + styles[posicao..]
            .find("</w:style>")
            .expect("fechamento do estilo");
    &styles[inicio..fim + "</w:style>".len()]
}

#[test]
fn css_do_romance_bate_com_o_golden() {
    let css = gerar_css(&romance(), &SeletorCss::default());
    assert_eq!(css, include_str!("golden/romance.css"));
}

#[test]
fn css_usa_o_seletor_de_corpo_pedido() {
    let seletor = SeletorCss::novo(".cm-content > .cm-line").expect("seletor válido");
    let css = gerar_css(&romance(), &seletor);
    assert!(css.contains("\n.cm-content > .cm-line {\n"), "{css}");
    assert!(!css.contains(".md-novel-corpo"), "{css}");
}

/// Toda propriedade é explícita nas duas regras, então a do estilo nomeado só
/// vale se for mais específica que a do corpo, seja qual for o seletor dele.
#[test]
fn regra_do_estilo_nomeado_e_mais_especifica_que_a_do_corpo() {
    for corpo in [".md-novel-corpo", ".cm-content > .cm-line", ".a, #b .c"] {
        let seletor = SeletorCss::novo(corpo).expect("seletor válido");
        let css = gerar_css(&romance(), &seletor);
        let esperado = format!("\n:is({corpo})[data-md-novel-estilo=\"Epígrafe\"] {{\n");
        assert!(css.contains(&esperado), "faltou {esperado:?} em:\n{css}");
        assert!(
            !css.contains("\n[data-md-novel-estilo="),
            "regra nomeada sem o seletor do corpo perde na cascata:\n{css}"
        );
    }
}

#[test]
fn css_nao_usa_margem_vertical_que_se_fundiria_entre_paragrafos() {
    let css = gerar_css(&romance(), &SeletorCss::default());
    assert!(!css.contains("margin"), "{css}");
    assert!(
        css.contains("  padding-top: 12pt;\n  padding-bottom: 12pt;\n"),
        "{css}"
    );
}

#[test]
fn seletor_que_fecharia_a_regra_e_rejeitado() {
    for texto in [
        "", "  ", "a { } b", "a; b", "a\nb", ".a /* x", ".a */", "<style>", ".a\\)", ".a[b",
        ".a(b", ".a)", ".a]", "[x=\"y]", "[x='y]",
    ] {
        assert!(
            SeletorCss::novo(texto).is_err(),
            "{texto:?} deveria ser rejeitado"
        );
    }
}

#[test]
fn seletor_legitimo_de_outro_editor_e_aceito() {
    for texto in [
        ".cm-content > .cm-line",
        ".a, .b",
        "[data-x=\"y (z)\"] ~ p:not(.q)",
        "#editor .linha",
    ] {
        assert!(
            SeletorCss::novo(texto).is_ok(),
            "{texto:?} deveria ser aceito"
        );
    }
}

#[test]
fn corpo_vira_as_propriedades_do_body_text() {
    let styles = styles_gerado();
    let corpo = bloco_do_estilo(&styles, "BodyText");
    let golden = include_str!("golden/romance-corpo-propriedades.xml").trim_end();
    assert!(
        corpo.contains(golden),
        "esperado:\n{golden}\nveio:\n{corpo}"
    );
}

#[test]
fn body_text_guarda_a_identidade_e_perde_a_formatacao_antiga() {
    let styles = styles_gerado();
    let corpo = bloco_do_estilo(&styles, "BodyText");
    assert!(corpo.contains(r#"<w:name w:val="Body Text" />"#), "{corpo}");
    assert!(
        corpo.contains(r#"<w:link w:val="BodyTextChar" />"#),
        "{corpo}"
    );
    assert!(!corpo.contains(r#"w:before="180""#), "{corpo}");
    assert!(!corpo.contains("FF0000"), "{corpo}");
    assert_eq!(corpo.matches("<w:pPr>").count(), 1, "{corpo}");
    assert_eq!(corpo.matches("<w:rPr>").count(), 1, "{corpo}");
}

#[test]
fn estilos_nomeados_viram_estilos_de_paragrafo_com_o_mesmo_nome() {
    let styles = styles_gerado();
    assert_eq!(
        bloco_do_estilo(&styles, "Epígrafe"),
        include_str!("golden/romance-epigrafe.xml").trim_end()
    );
    assert_eq!(
        bloco_do_estilo(&styles, "Placaderua"),
        include_str!("golden/romance-placa.xml").trim_end()
    );
    assert!(
        styles.trim_end().ends_with("</w:style></w:styles>"),
        "os estilos nomeados entram no fim de w:styles"
    );
}

#[test]
fn o_resto_do_modelo_fica_como_estava() {
    let docx = gerar_reference_docx(&modelo(), &romance()).expect("gerar o reference.docx");
    assert_eq!(entrada(&docx, "word/document.xml"), DOCUMENT_DO_MODELO);
    let styles = entrada(&docx, "word/styles.xml");
    for id in ["Normal", "FirstParagraph", "Title"] {
        assert_eq!(
            bloco_do_estilo(&styles, id),
            bloco_do_estilo(STYLES_DO_MODELO, id),
            "o estilo {id} não podia mudar"
        );
    }
    assert!(
        styles.contains(r#"<w:spacing w:after="200" />"#),
        "{styles}"
    );
}

#[test]
fn gerar_duas_vezes_devolve_os_mesmos_bytes() {
    let primeira = gerar_reference_docx(&modelo(), &romance()).expect("primeira geração");
    let segunda = gerar_reference_docx(&modelo(), &romance()).expect("segunda geração");
    assert_eq!(primeira, segunda);
    assert_ne!(
        primeira,
        modelo(),
        "o docx gerado não pode ser o modelo intacto"
    );
}

/// Fitness function do subconjunto (ADR 0002): toda chave do schema existe
/// nos dois lados, em toda regra CSS e em todo estilo gerado.
#[test]
fn toda_chave_do_schema_sai_no_css_e_no_docx() {
    let tabela = [
        ("fonte", "font-family:", "<w:rFonts "),
        ("tamanho", "font-size:", "<w:sz "),
        ("entrelinha", "line-height:", " w:line=\""),
        ("alinhamento", "text-align:", "<w:jc "),
        (
            "recuo-primeira-linha",
            "text-indent:",
            "<w:ind w:firstLine=\"",
        ),
        // padding e não margin: margens de blocos vizinhos se fundem no CSS,
        // e o Word soma o espaço depois de um com o espaço antes do outro.
        ("espaco-antes", "padding-top:", " w:before=\""),
        ("espaco-depois", "padding-bottom:", " w:after=\""),
        ("italico", "font-style:", "<w:i "),
        ("negrito", "font-weight:", "<w:b "),
    ];
    let estilo = romance();
    let valor = serde_yaml_ng::to_value(&estilo.corpo).expect("serializar o corpo");
    let schema: BTreeSet<String> = valor
        .as_mapping()
        .expect("o corpo é um mapa")
        .keys()
        .map(|chave| chave.as_str().expect("chave é texto").to_owned())
        .collect();
    let cobertas: BTreeSet<String> = tabela
        .iter()
        .map(|(chave, _, _)| (*chave).to_owned())
        .collect();
    assert_eq!(
        schema, cobertas,
        "chave do schema sem linha nesta tabela, ou o contrário"
    );

    let css = gerar_css(&estilo, &SeletorCss::default());
    let styles = styles_gerado();
    let regras = 1 + estilo.estilos.len();
    let gerados = ["BodyText", "Carta", "Epígrafe", "Placaderua"];
    assert_eq!(gerados.len(), regras);
    for (chave, propriedade_css, marca_docx) in tabela {
        assert_eq!(
            css.matches(propriedade_css).count(),
            regras,
            "{chave}: {propriedade_css} deveria aparecer uma vez em cada regra CSS"
        );
        for id in gerados {
            let bloco = bloco_do_estilo(&styles, id);
            assert_eq!(
                bloco.matches(marca_docx).count(),
                1,
                "{chave}: {marca_docx} deveria aparecer uma vez no estilo {id}:\n{bloco}"
            );
        }
    }
}

#[test]
fn styles_do_modelo_acima_de_4_mib_e_rejeitado() {
    let enchimento = format!("<!--{}-->\n</w:styles>", "x".repeat(4 * 1024 * 1024));
    let enorme = STYLES_DO_MODELO.replace("</w:styles>", &enchimento);
    let erro = gerar_reference_docx(&modelo_com(&enorme), &romance()).expect_err("styles enorme");
    assert!(
        matches!(erro, ErroEstilo::ModeloInesperado { .. }),
        "veio {erro}"
    );
    assert!(erro.to_string().contains("4 MiB"), "{erro}");
}

#[test]
fn styles_gerado_tem_data_fixa_no_zip() {
    let docx = gerar_reference_docx(&modelo(), &romance()).expect("gerar o reference.docx");
    let mut zip = ZipArchive::new(Cursor::new(docx)).expect("zip");
    let data = zip
        .by_name("word/styles.xml")
        .expect("styles.xml")
        .last_modified();
    assert_eq!(
        data,
        Some(zip::DateTime::default()),
        "a data fixa é 1980-01-01"
    );
}

#[test]
fn nome_de_estilo_que_ja_existe_no_modelo_e_rejeitado() {
    for nome in ["Title", "title", "Body Text", "First Paragraph", "BodyText"] {
        let yaml = format!("{ROMANCE}  {nome}:\n    italico: true\n");
        let estilo = Estilo::de_yaml(&yaml).expect("o yaml em si é válido");
        let erro = gerar_reference_docx(&modelo(), &estilo).expect_err("nome reservado");
        assert!(
            matches!(erro, ErroEstilo::NomeReservado { .. }),
            "{nome}: veio {erro}"
        );
    }
}

#[test]
fn dois_estilos_que_dariam_o_mesmo_id_no_word_sao_rejeitados() {
    let yaml = format!("{ROMANCE}  Placaderua:\n    italico: true\n");
    let erro = Estilo::de_yaml(&yaml).expect_err("ids iguais");
    assert!(
        matches!(erro, ErroEstilo::NomeReservado { .. }),
        "veio {erro}"
    );
}

#[test]
fn modelo_fora_do_formato_e_rejeitado_em_vez_de_gerar_docx_torto() {
    let sem_body_text = STYLES_DO_MODELO.replace("BodyText", "OutroNome");
    let erro =
        gerar_reference_docx(&modelo_com(&sem_body_text), &romance()).expect_err("sem BodyText");
    assert!(
        matches!(erro, ErroEstilo::ModeloInesperado { .. }),
        "veio {erro}"
    );

    let outra_raiz = "<?xml version=\"1.0\"?><w:outro></w:outro>";
    let erro = gerar_reference_docx(&modelo_com(outra_raiz), &romance()).expect_err("raiz errada");
    assert!(
        matches!(erro, ErroEstilo::ModeloInesperado { .. }),
        "veio {erro}"
    );

    let erro = gerar_reference_docx(b"isto nao e um zip", &romance()).expect_err("não é zip");
    assert!(matches!(erro, ErroEstilo::ModeloZip(_)), "veio {erro}");
}

/// O exemplo do README é o primeiro estilo.yaml que o autor copia: ele não
/// pode deixar de valer quando o schema mudar.
#[test]
fn exemplo_do_readme_e_valido_e_usa_todas_as_chaves() {
    let caminho =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../exemplos/estilo.yaml");
    let yaml = std::fs::read_to_string(&caminho)
        .unwrap_or_else(|erro| panic!("ler {}: {erro}", caminho.display()));
    let estilo = Estilo::de_yaml(&yaml).unwrap_or_else(|erro| panic!("exemplo inválido: {erro}"));
    // Cada chave do schema aparece escrita no corpo do exemplo, para o autor
    // ver todas as opções num lugar só. As chaves vêm do próprio schema: uma
    // propriedade nova reprova aqui até entrar no exemplo.
    let valor = serde_yaml_ng::to_value(&estilo.corpo).expect("serializar o corpo");
    let chaves: Vec<&str> = valor
        .as_mapping()
        .expect("o corpo é um mapa")
        .keys()
        .map(|chave| chave.as_str().expect("chave é texto"))
        .collect();
    assert!(
        chaves.len() >= 9,
        "esperava as chaves do schema, veio {chaves:?}"
    );
    for chave in chaves {
        assert!(
            yaml.contains(&format!("\n  {chave}:")),
            "o corpo do exemplo não mostra {chave}"
        );
    }
    assert!(
        !estilo.estilos.is_empty(),
        "o exemplo mostra ao menos um estilo nomeado"
    );
    let docx = gerar_reference_docx(&modelo(), &estilo).expect("o exemplo gera o reference.docx");
    assert!(!docx.is_empty());
}
