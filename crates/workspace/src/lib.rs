//! A pasta do livro. Toda gravação em disco do md Novel passa por aqui:
//! arquivo gerado (`SaidaPendente`) e, no app, capítulo salvo.
//!
//! Este crate não depende de tauri nem abre processo: o app e a linha de
//! comando são adaptadores em cima dele.

mod erro;
mod livro;
mod saida;
mod texto;

pub use erro::ErroWorkspace;
pub use livro::{Capitulo, Gravacao, Livro, ResumoDoCapitulo, criar_livro, nome_de_livro};
pub use saida::SaidaPendente;
pub use texto::{contar_palavras, titulo_do_texto};
