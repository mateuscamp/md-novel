//! Erros da pasta do livro. Toda mensagem traz o caminho e o que fazer.

use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ErroWorkspace {
    #[error(
        "o caminho de saída {recebido:?} não serve: precisa ser um arquivo, ou um link para um arquivo que exista, e não uma pasta"
    )]
    SaidaInvalida { recebido: PathBuf },

    #[error(
        "a saída {recebido:?} precisa terminar em .{esperada}; assim nenhum comando sobrescreve um capítulo"
    )]
    ExtensaoErrada { recebido: PathBuf, esperada: String },

    #[error("a pasta {recebido:?} não existe; crie a pasta antes de gravar nela")]
    PastaInexistente { recebido: PathBuf },

    #[error("{recebido:?} não é uma pasta; escolha a pasta onde ficam os capítulos do livro")]
    NaoEPasta { recebido: PathBuf },

    #[error("{nome:?} não é um capítulo desta pasta; os capítulos são os arquivos .md dela")]
    CapituloDesconhecido { nome: String },

    #[error(
        "o capítulo {caminho:?} não está em UTF-8; converta o arquivo para UTF-8 antes de abrir"
    )]
    NaoEUtf8 { caminho: PathBuf },

    #[error(
        "o capítulo {caminho:?} mistura finais de linha do Windows e do Unix; uniformize o arquivo antes de editar, para o md Novel não trocar linha que você não mexeu"
    )]
    FimDeLinhaMisturado { caminho: PathBuf },

    #[error("{caminho:?} passa de 1 MiB; não parece um estilo.yaml")]
    EstiloGrandeDemais { caminho: PathBuf },

    #[error("o capítulo {caminho:?} passa de 16 MiB; não parece um capítulo de texto")]
    CapituloGrandeDemais { caminho: PathBuf },

    #[error(
        "o capítulo {caminho:?} mudou no disco depois que você abriu (outro programa ou a sincronização); nada foi gravado"
    )]
    MudouPorFora { caminho: PathBuf },

    #[error(
        "{recebido:?} já existe; nada foi gravado por cima. Para começar do modelo, renomeie ou apague o arquivo antes"
    )]
    JaExiste { recebido: PathBuf },

    #[error("erro de leitura ou escrita: {0}")]
    Io(#[from] std::io::Error),
}
