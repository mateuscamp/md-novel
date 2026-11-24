//! Linha de comando fina do md Novel: lê os argumentos, chama `estilo` e
//! `exportar` e grava as saídas. Regra de estilo e chamada ao Pandoc não moram aqui.

use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use estilo::{Estilo, SeletorCss, gerar_css, gerar_reference_docx};
use exportar::{exportar_docx, modelo_reference_docx};
use workspace::SaidaPendente;

/// Um estilo.yaml escrito à mão tem menos de 10 kB. O teto evita ler um
/// arquivo errado inteiro para a memória.
const TAMANHO_MAXIMO_DO_ESTILO: u64 = 1024 * 1024;

#[derive(Parser)]
#[command(
    name = "md-novel",
    version,
    about = "Estilo único para o editor e o .docx de um livro em Markdown"
)]
struct Cli {
    #[command(subcommand)]
    comando: Comando,
}

#[derive(Subcommand)]
enum Comando {
    /// Gera o CSS do editor e o reference.docx a partir do estilo.yaml.
    Estilo(ArgumentosEstilo),
    /// Exporta os capítulos, na ordem dada, para um .docx com o estilo do livro.
    Exportar(ArgumentosExportar),
}

#[derive(Args)]
struct ArgumentosEstilo {
    /// O estilo.yaml do livro.
    estilo: PathBuf,
    /// Onde gravar o CSS do editor (termina em .css).
    #[arg(long)]
    css: Option<PathBuf>,
    /// Onde gravar o reference.docx do Pandoc (termina em .docx).
    #[arg(long)]
    docx: Option<PathBuf>,
    /// Seletor CSS dos parágrafos do corpo, para usar o CSS em outro editor.
    #[arg(long, default_value = ".md-novel-corpo")]
    seletor_corpo: String,
}

#[derive(Args)]
struct ArgumentosExportar {
    /// O estilo.yaml do livro.
    #[arg(long)]
    estilo: PathBuf,
    /// O .docx a gravar.
    #[arg(long)]
    saida: PathBuf,
    /// Os capítulos em Markdown, na ordem do livro.
    #[arg(required = true)]
    capitulos: Vec<PathBuf>,
}

fn main() -> ExitCode {
    let resultado = match Cli::parse().comando {
        Comando::Estilo(argumentos) => compilar_estilo(&argumentos),
        Comando::Exportar(argumentos) => exportar(&argumentos),
    };
    match resultado {
        Ok(()) => ExitCode::SUCCESS,
        Err(erro) => {
            eprintln!("erro: {erro}");
            ExitCode::FAILURE
        }
    }
}

fn compilar_estilo(argumentos: &ArgumentosEstilo) -> Result<(), Box<dyn Error>> {
    if argumentos.css.is_none() && argumentos.docx.is_none() {
        return Err("nada a gerar: passe --css, --docx ou os dois".into());
    }
    // As duas saídas são conferidas e escritas em temporários antes de
    // qualquer troca. Um erro até aqui (extensão, pasta, estilo inválido)
    // descarta os temporários e deixa as saídas antigas como estavam: CSS
    // novo ao lado de reference.docx antigo é o desencontro que esta
    // ferramenta existe para evitar.
    let mut css = preparar(argumentos.css.as_deref(), "css")?;
    let mut docx = preparar(argumentos.docx.as_deref(), "docx")?;
    let seletor = SeletorCss::novo(&argumentos.seletor_corpo)?;
    let estilo = ler_estilo(&argumentos.estilo)?;
    if let Some(saida) = &mut css {
        saida.escrever(gerar_css(&estilo, &seletor).as_bytes())?;
    }
    if let Some(saida) = &mut docx {
        saida.escrever(&gerar_reference_docx(&modelo_reference_docx()?, &estilo)?)?;
    }
    for saida in [css, docx].into_iter().flatten() {
        saida.publicar()?;
    }
    Ok(())
}

fn preparar(
    caminho: Option<&Path>,
    extensao: &str,
) -> Result<Option<SaidaPendente>, Box<dyn Error>> {
    match caminho {
        Some(caminho) => Ok(Some(SaidaPendente::preparar(caminho, extensao)?)),
        None => Ok(None),
    }
}

fn exportar(argumentos: &ArgumentosExportar) -> Result<(), Box<dyn Error>> {
    let estilo = ler_estilo(&argumentos.estilo)?;
    let reference = gerar_reference_docx(&modelo_reference_docx()?, &estilo)?;
    let avisos = exportar_docx(&argumentos.capitulos, &reference, &argumentos.saida)?;
    // O .docx saiu, mas imagem que não entrou ou nota repetida o autor precisa ver.
    for aviso in avisos {
        eprintln!("aviso do Pandoc: {aviso}");
    }
    Ok(())
}

fn ler_estilo(caminho: &Path) -> Result<Estilo, Box<dyn Error>> {
    let com_caminho = |erro: &dyn Error| format!("{}: {erro}", caminho.display());
    let mut texto = String::new();
    File::open(caminho)
        .map_err(|erro| com_caminho(&erro))?
        .take(TAMANHO_MAXIMO_DO_ESTILO + 1)
        .read_to_string(&mut texto)
        .map_err(|erro| com_caminho(&erro))?;
    if texto.len() as u64 > TAMANHO_MAXIMO_DO_ESTILO {
        return Err(format!(
            "{}: passa de 1 MiB; esperado um estilo.yaml",
            caminho.display()
        )
        .into());
    }
    Ok(Estilo::de_yaml(&texto).map_err(|erro| com_caminho(&erro))?)
}
