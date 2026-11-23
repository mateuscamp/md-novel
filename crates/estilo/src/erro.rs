//! Erros do compilador de estilo. Toda mensagem traz o valor recebido e o
//! formato esperado, porque quem lê é o autor do livro, não um programador.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ErroEstilo {
    #[error("estilo.yaml inválido: {mensagem}")]
    Yaml { mensagem: String },

    #[error("versão {recebida} do estilo.yaml não é suportada; esperado: versao: 1")]
    VersaoNaoSuportada { recebida: u32 },

    #[error(
        "medida inválida {recebido:?}: {motivo}; esperado número com unidade pt, cm, mm ou in, como 1.25cm"
    )]
    MedidaInvalida { recebido: String, motivo: String },

    #[error("valor inválido para {chave}: {recebido:?}; esperado {esperado}")]
    ValorInvalido {
        chave: &'static str,
        recebido: String,
        esperado: &'static str,
    },

    #[error(
        "o corpo precisa definir {chave}: sem ela o editor e o Word usariam cada um o seu padrão"
    )]
    CorpoIncompleto { chave: &'static str },

    #[error(
        "nome de estilo inválido {recebido:?}; esperado 1 a 64 letras, números, espaços ou hifens"
    )]
    NomeDeEstiloInvalido { recebido: String },

    #[error(
        "o estilo {nome:?} bate com {existente:?}, que já existe no modelo do Pandoc; escolha outro nome"
    )]
    NomeReservado { nome: String, existente: String },

    #[error(
        "seletor CSS inválido {recebido:?}; esperado um seletor sem chaves, ponto e vírgula ou quebra de linha"
    )]
    SeletorInvalido { recebido: String },

    #[error("o modelo reference.docx não tem o formato esperado: {motivo}")]
    ModeloInesperado { motivo: String },

    #[error("não consegui ler o modelo reference.docx: {0}")]
    ModeloZip(#[from] zip::result::ZipError),

    #[error("não consegui ler o styles.xml do modelo: {0}")]
    ModeloXml(#[from] quick_xml::Error),

    #[error("erro de leitura ou escrita: {0}")]
    Io(#[from] std::io::Error),
}

impl From<serde_yaml_ng::Error> for ErroEstilo {
    fn from(erro: serde_yaml_ng::Error) -> Self {
        ErroEstilo::Yaml {
            mensagem: traduzir_erro_de_yaml(&erro.to_string()),
        }
    }
}

/// Frases fixas do serde e do leitor de YAML, na ordem em que são trocadas:
/// as mais específicas primeiro. O que não está aqui passa como veio.
const TRADUCOES: [(&str, &str); 19] = [
    // Erros de sintaxe do leitor de YAML que o autor mais provoca.
    (
        "found character that cannot start any token",
        "caractere que não pode começar um valor; se for um tab, troque por espaços",
    ),
    (
        "did not find expected key",
        "esperava uma chave aqui; confira a indentação",
    ),
    (
        "mapping values are not allowed in this context",
        "dois-pontos fora de lugar; confira a indentação e as aspas",
    ),
    (
        "could not find expected ':'",
        "faltou o dois-pontos depois da chave",
    ),
    (
        "found unexpected end of stream",
        "o arquivo terminou no meio de um valor; confira as aspas",
    ),
    (", expected a boolean", "; esperado true ou false"),
    ("invalid type: sequence", "tipo inválido: lista"),
    ("invalid type: map", "tipo inválido: mapa"),
    ("invalid type: string ", "tipo inválido: texto "),
    ("invalid type: integer ", "tipo inválido: número inteiro "),
    ("invalid type: floating point ", "tipo inválido: número "),
    ("invalid type: ", "tipo inválido: "),
    ("invalid value: ", "valor inválido: "),
    ("unknown field ", "campo desconhecido "),
    ("unknown variant ", "valor desconhecido "),
    ("missing field ", "falta o campo "),
    ("duplicate entry with key ", "chave repetida: "),
    (", expected one of ", "; esperado um de "),
    (", expected ", "; esperado "),
];

/// Passa para o português as mensagens que o autor mais vê ao errar o
/// estilo.yaml e troca o " at line L column C" por "(linha L, coluna C)".
/// As mensagens deste crate já chegam em português e passam intactas.
fn traduzir_erro_de_yaml(original: &str) -> String {
    let (mensagem, local) = separar_local(original);
    let mut traduzida = mensagem.to_owned();
    for (ingles, portugues) in TRADUCOES {
        traduzida = traduzida.replace(ingles, portugues);
    }
    match local {
        Some((linha, coluna)) => format!("{traduzida} (linha {linha}, coluna {coluna})"),
        None => traduzida,
    }
}

/// Separa a mensagem do local do problema. O leitor de YAML escreve
/// "problema at line L column C" e, nos erros de sintaxe, acrescenta
/// ", while parsing ..." depois; esse contexto é descartado, e vale o
/// primeiro local, que é o do problema.
fn separar_local(original: &str) -> (&str, Option<(u32, u32)>) {
    const MARCA: &str = " at line ";
    for (inicio, _) in original.match_indices(MARCA) {
        let resto = &original[inicio + MARCA.len()..];
        let local = resto
            .split_once(", while ")
            .map_or(resto, |(local, _)| local);
        let numeros = local
            .split_once(" column ")
            .and_then(|(linha, coluna)| Some((linha.parse().ok()?, coluna.parse().ok()?)));
        if let Some(numeros) = numeros {
            return (&original[..inicio], Some(numeros));
        }
    }
    (original, None)
}
