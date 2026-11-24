//! O livro como pasta: quais arquivos são capítulos, e a leitura e a
//! gravação de um capítulo sem mexer no que o autor não mexeu.

use std::fs::{self, File};
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::str;

use crate::erro::ErroWorkspace;
use crate::saida::{SaidaPendente, renomear_sem_sobrescrever};
use crate::texto::{contar_palavras, titulo_do_texto};

/// Marca de ordem de bytes do UTF-8, que alguns editores do Windows gravam.
const BOM: &[u8] = b"\xEF\xBB\xBF";
/// Um romance inteiro em texto não chega a 2 MiB. O teto evita carregar
/// para a memória e para a interface um arquivo que não é capítulo.
const TAMANHO_MAXIMO_DO_CAPITULO: u64 = 16 * 1024 * 1024;
/// O maior nome de capítulo, em caracteres, sem contar o `.md`.
const TAMANHO_MAXIMO_DO_NOME: usize = 100;
/// O que um nome de arquivo não pode ter em algum dos sistemas por onde a
/// pasta do livro passa: a barra daqui e os reservados do Windows de quem
/// recebe a pasta pela sincronização.
const PROIBIDOS_NO_NOME: [char; 9] = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

/// O nome serve de nome de arquivo ou de pasta em todo sistema por onde o
/// livro passa. Ponto no começo é arquivo escondido (e o `..` que sai da
/// pasta); ponto ou espaço no fim o Windows corta.
fn nome_serve(base: &str) -> bool {
    !base.is_empty()
        && !base.starts_with('.')
        && !base.ends_with('.')
        && !base.ends_with(' ')
        && base.chars().count() <= TAMANHO_MAXIMO_DO_NOME
        && !base
            .chars()
            .any(|letra| letra.is_control() || PROIBIDOS_NO_NOME.contains(&letra))
}

/// O nome da pasta de um livro: o que o autor digitou, sem espaços em volta.
/// Público para o app conferir antes de abrir o seletor de onde criar.
pub fn nome_de_livro(nome: &str) -> Result<String, ErroWorkspace> {
    let aparado = nome.trim();
    if !nome_serve(aparado) {
        return Err(ErroWorkspace::NomeDeLivroInvalido {
            recebido: nome.to_owned(),
        });
    }
    Ok(aparado.to_owned())
}

/// O nome do arquivo de um capítulo novo: o que o autor digitou, sem espaços
/// em volta e com `.md` no fim (digitado ou não).
fn nome_de_capitulo(nome: &str) -> Result<String, ErroWorkspace> {
    let aparado = nome.trim();
    let base = aparado
        .len()
        .checked_sub(".md".len())
        .filter(|corte| {
            aparado.is_char_boundary(*corte) && aparado[*corte..].eq_ignore_ascii_case(".md")
        })
        .map_or(aparado, |corte| &aparado[..corte]);
    if !nome_serve(base) {
        return Err(ErroWorkspace::NomeDeCapituloInvalido {
            recebido: nome.to_owned(),
        });
    }
    Ok(format!("{base}.md"))
}

/// Cria a pasta de um livro novo, vazia, dentro de `onde`, e abre. O nome é
/// o que o autor digitou, conferido como o de capítulo. Uma pasta ou arquivo
/// com esse nome nunca é reaproveitado: o livro novo nasce sem nada dentro.
pub fn criar_livro(onde: &Path, nome: &str) -> Result<Livro, ErroWorkspace> {
    let nome = nome_de_livro(nome)?;
    if !onde.is_dir() {
        return Err(ErroWorkspace::NaoEPasta {
            recebido: onde.to_owned(),
        });
    }
    let destino = onde.join(nome);
    // `create_dir` confere e cria numa operação só, e recusa até link quebrado.
    match fs::create_dir(&destino) {
        Ok(()) => Livro::abrir(&destino),
        Err(erro) if erro.kind() == ErrorKind::AlreadyExists => {
            Err(ErroWorkspace::JaExiste { recebido: destino })
        }
        Err(erro) => Err(erro.into()),
    }
}

/// Um estilo.yaml escrito à mão tem menos de 10 kB.
const TAMANHO_MAXIMO_DO_ESTILO: u64 = 1024 * 1024;

/// Uma pasta aberta como livro.
#[derive(Debug, Clone)]
pub struct Livro {
    pasta: PathBuf,
}

/// Como o arquivo do capítulo estava no disco, para devolvê-lo igual.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Formato {
    /// Começava com a marca de ordem de bytes do UTF-8.
    bom: bool,
    /// As linhas terminavam em CR LF (Windows) e não em LF.
    crlf: bool,
}

/// Um capítulo lido do disco. Guarda os bytes como estavam, para saber se
/// alguém (o Dropbox, outro editor) mudou o arquivo antes de salvar.
#[derive(Debug, Clone)]
pub struct Capitulo {
    caminho: PathBuf,
    bytes_lidos: Vec<u8>,
    formato: Formato,
    texto: String,
}

/// Um capítulo como aparece na lista do livro.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumoDoCapitulo {
    pub nome: String,
    /// O primeiro título Markdown do texto, se houver.
    pub titulo: Option<String>,
    /// `None` quando o arquivo não pôde ser lido como capítulo.
    pub palavras: Option<usize>,
}

/// O que `Capitulo::salvar` fez.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gravacao {
    /// O texto era o mesmo do disco: nada foi escrito.
    SemMudanca,
    Gravado,
}

impl Formato {
    /// Separa o formato do texto. `None` quando os finais de linha estão
    /// misturados: aí não há como devolver o arquivo igual depois de editar.
    fn separar(bytes: &[u8]) -> Option<(Formato, &[u8])> {
        let (bom, corpo) = match bytes.strip_prefix(BOM) {
            Some(corpo) => (true, corpo),
            None => (false, bytes),
        };
        let total_de_cr = corpo.iter().filter(|&&byte| byte == b'\r').count();
        let total_de_lf = corpo.iter().filter(|&&byte| byte == b'\n').count();
        let total_de_crlf = corpo.windows(2).filter(|par| par == b"\r\n").count();
        let crlf = match (total_de_cr, total_de_crlf) {
            (0, _) => false,
            // Todo CR e todo LF fazem parte de um CR LF.
            (cr, pares) if cr == pares && total_de_lf == pares => true,
            _ => return None,
        };
        Some((Formato { bom, crlf }, corpo))
    }

    /// O texto do editor de volta aos bytes do arquivo.
    fn codificar(self, texto: &str) -> Vec<u8> {
        // O editor trabalha em LF; um CR que venha de texto colado sai aqui.
        let em_lf = texto.replace("\r\n", "\n").replace('\r', "\n");
        let corpo = if self.crlf {
            em_lf.replace('\n', "\r\n")
        } else {
            em_lf
        };
        let mut bytes = Vec::with_capacity(BOM.len() + corpo.len());
        if self.bom {
            bytes.extend_from_slice(BOM);
        }
        bytes.extend_from_slice(corpo.as_bytes());
        bytes
    }
}

impl Livro {
    /// Abre uma pasta como livro. Não cria nem altera nada nela.
    pub fn abrir(pasta: &Path) -> Result<Livro, ErroWorkspace> {
        if !pasta.is_dir() {
            return Err(ErroWorkspace::NaoEPasta {
                recebido: pasta.to_owned(),
            });
        }
        Ok(Livro {
            pasta: pasta.to_owned(),
        })
    }

    pub fn pasta(&self) -> &Path {
        &self.pasta
    }

    /// Onde fica o estilo do livro. O arquivo pode não existir.
    pub fn caminho_do_estilo(&self) -> PathBuf {
        self.pasta.join("estilo.yaml")
    }

    /// Grava `texto` como o `estilo.yaml` do livro, só se ele ainda não
    /// existe. É a única gravação na pasta do livro que não é salvar um
    /// capítulo, e só acontece a pedido do autor (ADR 0003).
    ///
    /// Qualquer coisa com esse nome já conta como existir, inclusive um link
    /// (mesmo quebrado): um link é o estilo do autor guardado em outro lugar,
    /// e não é o md Novel que decide trocá-lo por um arquivo.
    pub fn criar_estilo(&self, texto: &str) -> Result<(), ErroWorkspace> {
        let destino = self.caminho_do_estilo();
        if fs::symlink_metadata(&destino).is_ok() {
            return Err(ErroWorkspace::JaExiste { recebido: destino });
        }
        let mut saida = SaidaPendente::preparar(&destino, "yaml")?;
        saida.escrever(texto.as_bytes())?;
        // Se o arquivo apareceu depois da conferência acima, é aqui que a
        // gravação é recusada.
        saida.publicar_sem_sobrescrever()
    }

    /// Grava o `estilo.yaml` que o painel de estilo montou. `lido` é o texto
    /// que o painel tinha em mãos (`None` se o arquivo não existia): se o que
    /// está no disco agora é outra coisa, o arquivo mudou por fora e nada é
    /// gravado, como num capítulo (ADR 0003).
    pub fn gravar_estilo(&self, novo: &str, lido: Option<&str>) -> Result<(), ErroWorkspace> {
        let caminho = self.caminho_do_estilo();
        let mudou = || ErroWorkspace::EstiloMudouPorFora {
            caminho: caminho.clone(),
        };
        let atual = self.ler_estilo()?;
        if atual.as_deref() != lido {
            return Err(mudou());
        }
        if atual.as_deref() == Some(novo) {
            return Ok(());
        }
        let mut saida = SaidaPendente::preparar(&caminho, "yaml")?;
        saida.escrever(novo.as_bytes())?;
        if atual.is_some() {
            return saida.publicar();
        }
        // Não existia quando o painel leu: se apareceu de lá para cá, a
        // troca que recusa sobrescrever é quem nota.
        saida.publicar_sem_sobrescrever().map_err(|erro| {
            if matches!(erro, ErroWorkspace::JaExiste { .. }) {
                mudou()
            } else {
                erro
            }
        })
    }

    /// Cria um capítulo vazio com o nome que o autor deu e devolve o nome do
    /// arquivo (com `.md`). O nome é o único dado que vem de fora da lista
    /// do livro, e por isso é conferido aqui: precisa ser um nome de arquivo
    /// simples, dentro da pasta. O que já existe nunca é trocado (ADR 0003).
    pub fn criar_capitulo(&self, nome: &str) -> Result<String, ErroWorkspace> {
        let arquivo = nome_de_capitulo(nome)?;
        let destino = self.pasta.join(&arquivo);
        // Como no estilo: qualquer coisa com esse nome, até um link quebrado,
        // já conta como existir.
        if fs::symlink_metadata(&destino).is_ok() {
            return Err(ErroWorkspace::JaExiste { recebido: destino });
        }
        SaidaPendente::preparar(&destino, "md")?.publicar_sem_sobrescrever()?;
        Ok(arquivo)
    }

    /// Troca o nome do arquivo de um capítulo da lista e devolve o nome novo
    /// (com `.md`). Os bytes não mudam: o título é a linha `#` do texto, que
    /// o autor muda escrevendo. Nunca passa por cima de outro arquivo.
    pub fn renomear_capitulo(&self, atual: &str, novo: &str) -> Result<String, ErroWorkspace> {
        // O nome atual vem da interface: só vale se estiver na lista.
        if !self.capitulos()?.iter().any(|capitulo| capitulo == atual) {
            return Err(ErroWorkspace::CapituloDesconhecido {
                nome: atual.to_owned(),
            });
        }
        let novo = nome_de_capitulo(novo)?;
        if novo != atual {
            renomear_sem_sobrescrever(&self.pasta.join(atual), &self.pasta.join(&novo))?;
        }
        Ok(novo)
    }

    /// Troca o nome da pasta do livro, na mesma pasta de cima, e devolve o
    /// livro no lugar novo. Nada dentro dela muda (ADR 0003, "Vários
    /// livros"). Nunca passa por cima de outra pasta, nem vazia.
    pub fn renomear(&self, nome: &str) -> Result<Livro, ErroWorkspace> {
        let nome = nome_de_livro(nome)?;
        let Some(de_cima) = self.pasta.parent() else {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "a pasta {:?} não está dentro de outra; não há onde renomeá-la",
                    self.pasta
                ),
            )
            .into());
        };
        let destino = de_cima.join(nome);
        if destino != self.pasta {
            renomear_sem_sobrescrever(&self.pasta, &destino)?;
        }
        Livro::abrir(&destino)
    }

    /// O texto do `estilo.yaml`, ou `None` se o livro ainda não tem um.
    pub fn ler_estilo(&self) -> Result<Option<String>, ErroWorkspace> {
        let caminho = self.caminho_do_estilo();
        let arquivo = match File::open(&caminho) {
            Ok(arquivo) => arquivo,
            Err(erro) if erro.kind() == ErrorKind::NotFound => return Ok(None),
            Err(erro) => return Err(erro.into()),
        };
        let mut bytes = Vec::new();
        arquivo
            .take(TAMANHO_MAXIMO_DO_ESTILO + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > TAMANHO_MAXIMO_DO_ESTILO {
            return Err(ErroWorkspace::EstiloGrandeDemais { caminho });
        }
        match String::from_utf8(bytes) {
            Ok(texto) => Ok(Some(texto)),
            Err(_) => Err(ErroWorkspace::NaoEUtf8 { caminho }),
        }
    }

    /// Os capítulos: arquivos `.md` comuns da pasta, sem os ocultos, em
    /// ordem de nome. É a ordem do livro enquanto não houver `livro.yaml`.
    pub fn capitulos(&self) -> Result<Vec<String>, ErroWorkspace> {
        let mut nomes = Vec::new();
        for entrada in fs::read_dir(&self.pasta)? {
            let entrada = entrada?;
            // Nome que não é UTF-8 não tem como ir e voltar pela interface.
            let Ok(nome) = entrada.file_name().into_string() else {
                continue;
            };
            let e_md = Path::new(&nome)
                .extension()
                .is_some_and(|extensao| extensao.eq_ignore_ascii_case("md"));
            // `metadata` segue link: um link para um .md comum é capítulo.
            let e_arquivo = fs::metadata(entrada.path()).is_ok_and(|metadados| metadados.is_file());
            if e_md && e_arquivo && !nome.starts_with('.') {
                nomes.push(nome);
            }
        }
        nomes.sort();
        Ok(nomes)
    }

    /// A lista do livro: cada capítulo com o título e a contagem de palavras.
    ///
    /// Lê todos os capítulos uma vez. Capítulo que não dá para ler (fora do
    /// UTF-8, finais de linha misturados, grande demais) continua na lista,
    /// sem título e sem contagem: o erro, com o motivo, aparece quando o
    /// autor tenta abri-lo.
    pub fn resumos(&self) -> Result<Vec<ResumoDoCapitulo>, ErroWorkspace> {
        let resumos = self
            .capitulos()?
            .into_iter()
            .map(|nome| {
                let lido = self.ler_da_lista(&nome).ok();
                let texto = lido.as_ref().map(Capitulo::texto);
                ResumoDoCapitulo {
                    titulo: texto.and_then(titulo_do_texto),
                    palavras: texto.map(contar_palavras),
                    nome,
                }
            })
            .collect();
        Ok(resumos)
    }

    /// Lê um capítulo pelo nome que `capitulos` devolveu.
    pub fn ler_capitulo(&self, nome: &str) -> Result<Capitulo, ErroWorkspace> {
        // O nome vem da interface. Só vale se estiver na lista: nada de
        // caminho, `..` ou arquivo que não é capítulo.
        if !self.capitulos()?.iter().any(|capitulo| capitulo == nome) {
            return Err(ErroWorkspace::CapituloDesconhecido {
                nome: nome.to_owned(),
            });
        }
        self.ler_da_lista(nome)
    }

    /// Lê um arquivo que já se sabe estar na lista de capítulos.
    fn ler_da_lista(&self, nome: &str) -> Result<Capitulo, ErroWorkspace> {
        let caminho = self.pasta.join(nome);
        let mut bytes = Vec::new();
        File::open(&caminho)?
            .take(TAMANHO_MAXIMO_DO_CAPITULO + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > TAMANHO_MAXIMO_DO_CAPITULO {
            return Err(ErroWorkspace::CapituloGrandeDemais { caminho });
        }
        let Some((formato, corpo)) = Formato::separar(&bytes) else {
            return Err(ErroWorkspace::FimDeLinhaMisturado { caminho });
        };
        let Ok(corpo) = str::from_utf8(corpo) else {
            return Err(ErroWorkspace::NaoEUtf8 { caminho });
        };
        let texto = corpo.replace("\r\n", "\n");
        Ok(Capitulo {
            caminho,
            bytes_lidos: bytes,
            formato,
            texto,
        })
    }
}

impl Capitulo {
    /// O texto para o editor: sem a marca de ordem de bytes e com as linhas
    /// terminadas em LF.
    pub fn texto(&self) -> &str {
        &self.texto
    }

    /// Grava o texto do editor no formato em que o arquivo estava.
    ///
    /// Recusa se o arquivo mudou no disco desde a leitura, e não escreve
    /// nada se o texto é o mesmo que foi lido.
    pub fn salvar(&mut self, texto: &str) -> Result<Gravacao, ErroWorkspace> {
        let mudou_por_fora = || ErroWorkspace::MudouPorFora {
            caminho: self.caminho.clone(),
        };
        // Sync seguro: o que está no disco agora precisa ser o que foi lido.
        // Um arquivo apagado ou trocado por outro programa (ou pelo Dropbox,
        // vindo de outro computador) nunca é sobrescrito. Entre esta leitura
        // e a troca ainda cabe uma mudança externa; a janela é de
        // milissegundos e fica registrada no ADR 0003.
        let no_disco = match fs::read(&self.caminho) {
            Ok(bytes) => bytes,
            Err(erro) if erro.kind() == ErrorKind::NotFound => return Err(mudou_por_fora()),
            Err(erro) => return Err(erro.into()),
        };
        if no_disco != self.bytes_lidos {
            return Err(mudou_por_fora());
        }
        let novos = self.formato.codificar(texto);
        if novos == self.bytes_lidos {
            return Ok(Gravacao::SemMudanca);
        }
        let mut saida = SaidaPendente::preparar(&self.caminho, "md")?;
        saida.escrever(&novos)?;
        saida.publicar()?;
        self.texto = texto.replace("\r\n", "\n").replace('\r', "\n");
        self.bytes_lidos = novos;
        Ok(Gravacao::Gravado)
    }
}
