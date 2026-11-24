//! O estilo editado por um formulário (o painel "Estilo" do app, ADR 0003):
//! os campos de cada parágrafo viram um `Estilo` validado, e um `Estilo` vira
//! o texto do `estilo.yaml`. O piso é ida e volta sem perda.

use std::collections::BTreeMap;

use estilo::{Campos, CamposTrocados, Estilo};

const ROMANCE: &str = include_str!("fixtures/romance.yaml");
const EXEMPLO: &str = include_str!("../../../exemplos/estilo.yaml");

fn campos_de(estilo: &Estilo) -> (Campos, BTreeMap<String, CamposTrocados>) {
    (
        Campos::from(&estilo.corpo),
        estilo
            .estilos
            .iter()
            .map(|(nome, paragrafo)| {
                (
                    nome.clone(),
                    CamposTrocados::entre(paragrafo, &estilo.corpo),
                )
            })
            .collect(),
    )
}

// O estilo nomeado chega ao formulário só com o que troca: o resto é herdado,
// e mudar o corpo muda o estilo nomeado junto.
#[test]
fn o_estilo_nomeado_do_formulario_herda_do_corpo_o_que_nao_troca() {
    let estilo = Estilo::de_yaml(ROMANCE).expect("válido");
    let (mut corpo, estilos) = campos_de(&estilo);
    assert_eq!(
        estilos["Epígrafe"],
        CamposTrocados {
            alinhamento: Some("direita".to_owned()),
            recuo_primeira_linha: Some("0cm".to_owned()),
            espaco_depois: Some("24pt".to_owned()),
            italico: Some(true),
            ..CamposTrocados::default()
        }
    );
    corpo.tamanho = "14pt".to_owned();
    let mudado = Estilo::de_campos(corpo, estilos).expect("válido");
    assert_eq!(mudado.estilos["Epígrafe"].tamanho, mudado.corpo.tamanho);
    assert_eq!(mudado.estilos["Epígrafe"].tamanho.medida().pontos(), 14.0);
    // A carta trocou o tamanho: fica com o dela.
    assert_eq!(mudado.estilos["Carta"].tamanho.medida().pontos(), 11.0);
}

#[test]
fn o_yaml_escrito_le_de_volta_o_mesmo_estilo() {
    for texto in [ROMANCE, EXEMPLO] {
        let estilo = Estilo::de_yaml(texto).expect("estilo válido");
        let escrito = estilo.para_yaml().expect("escrever");
        assert_eq!(
            Estilo::de_yaml(&escrito).expect("o escrito é válido"),
            estilo,
            "{escrito}"
        );
        // Escrever de novo o que foi lido dá o mesmo texto.
        assert_eq!(
            Estilo::de_yaml(&escrito)
                .expect("válido")
                .para_yaml()
                .expect("escrever"),
            escrito
        );
    }
}

#[test]
fn o_yaml_escrito_comeca_com_a_explicacao_e_a_versao_e_o_corpo_vai_inteiro() {
    let escrito = Estilo::de_yaml(ROMANCE)
        .expect("válido")
        .para_yaml()
        .expect("escrever");
    assert!(escrito.starts_with("# "), "{escrito}");
    assert!(escrito.contains("\nversao: 1\ncorpo:\n"), "{escrito}");
    for chave in [
        "fonte:",
        "tamanho: 12pt",
        "entrelinha: 1.5",
        "alinhamento: justificado",
        "recuo-primeira-linha: 1.25cm",
        "espaco-antes: 0pt",
        "espaco-depois: 0pt",
        "italico: false",
        "negrito: false",
    ] {
        assert!(
            escrito.contains(&format!("\n  {chave}")),
            "{chave}: {escrito}"
        );
    }
}

#[test]
fn estilo_nomeado_so_leva_o_que_difere_do_corpo_e_continua_herdando_o_resto() {
    let estilo = Estilo::de_yaml(ROMANCE).expect("válido");
    let escrito = estilo.para_yaml().expect("escrever");
    let bloco: String = escrito
        .lines()
        .skip_while(|linha| *linha != "  Epígrafe:")
        .skip(1)
        .take_while(|linha| linha.starts_with("    "))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        bloco,
        "    alinhamento: direita\n    recuo-primeira-linha: 0cm\n    espaco-depois: 24pt\n    italico: true"
    );
    // Um estilo igual ao corpo em tudo fica como bloco vazio, e lê de volta.
    let mut com_igual = estilo.clone();
    com_igual
        .estilos
        .insert("Igual ao corpo".to_owned(), estilo.corpo.clone());
    let escrito = com_igual.para_yaml().expect("escrever");
    assert!(escrito.contains("  Igual ao corpo: {}\n"), "{escrito}");
    assert_eq!(Estilo::de_yaml(&escrito).expect("válido"), com_igual);
}

#[test]
fn os_campos_do_formulario_viram_o_mesmo_estilo() {
    for texto in [ROMANCE, EXEMPLO] {
        let estilo = Estilo::de_yaml(texto).expect("válido");
        let (corpo, estilos) = campos_de(&estilo);
        assert_eq!(
            Estilo::de_campos(corpo, estilos).expect("campos válidos"),
            estilo
        );
    }
}

/// O que estraga um campo válido, para o teste abaixo.
type Estrago = fn(&mut Campos);

#[test]
fn campo_invalido_e_recusado_com_o_valor_recebido() {
    let estilo = Estilo::de_yaml(ROMANCE).expect("válido");
    let (corpo, estilos) = campos_de(&estilo);
    let casos: [(&str, Estrago); 5] = [
        ("5000pt", |campos| campos.tamanho = "5000pt".to_owned()),
        ("torto", |campos| campos.alinhamento = "torto".to_owned()),
        ("12", |campos| campos.espaco_antes = "12".to_owned()),
        ("1000", |campos| campos.entrelinha = 1000.0),
        ("fonte", |campos| campos.fonte = String::new()),
    ];
    for (trecho, estragar) in casos {
        let mut ruim = corpo.clone();
        estragar(&mut ruim);
        let erro = Estilo::de_campos(ruim, estilos.clone()).expect_err(trecho);
        assert!(erro.to_string().contains(trecho), "{trecho}: {erro}");
    }
    // Nome de estilo que não cabe no CSS e no Word também.
    let mut com_nome_ruim = estilos.clone();
    com_nome_ruim.insert("a\"b".to_owned(), CamposTrocados::default());
    assert!(Estilo::de_campos(corpo, com_nome_ruim).is_err());
}

// Propriedade nova de estilo precisa aparecer no formulário: `Campos` tem as
// mesmas chaves de `EstiloParagrafo`, nem mais, nem menos.
#[test]
fn os_campos_do_formulario_sao_as_propriedades_do_estilo() {
    let estilo = Estilo::de_yaml(ROMANCE).expect("válido");
    let chaves = |yaml: String| -> Vec<String> {
        yaml.lines()
            .filter_map(|linha| linha.split_once(':').map(|(chave, _)| chave.to_owned()))
            .collect()
    };
    let do_estilo = chaves(serde_yaml_ng::to_string(&estilo.corpo).expect("serializar"));
    let do_formulario =
        chaves(serde_yaml_ng::to_string(&Campos::from(&estilo.corpo)).expect("serializar"));
    assert_eq!(do_formulario, do_estilo);
    assert_eq!(do_estilo.len(), 9, "{do_estilo:?}");
    // E o que um estilo nomeado pode trocar são essas mesmas chaves: um
    // estilo diferente do corpo em tudo as mostra todas.
    let outro = Estilo::de_yaml(
        "versao: 1\ncorpo:\n  fonte: Outra\n  tamanho: 20pt\n  entrelinha: 3\n  \
         alinhamento: centro\n  recuo-primeira-linha: 9cm\n  espaco-antes: 7pt\n  \
         espaco-depois: 8pt\n  italico: true\n  negrito: true\n",
    )
    .expect("válido");
    let trocados = CamposTrocados::entre(&outro.corpo, &estilo.corpo);
    assert_eq!(
        chaves(serde_yaml_ng::to_string(&trocados).expect("serializar")),
        do_estilo
    );
}
