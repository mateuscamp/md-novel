//! A pasta do livro. Toda gravação em disco do md-novel passa por aqui:
//! arquivo gerado (`SaidaPendente`) e, no app, capítulo salvo.
//!
//! Este crate não depende de tauri nem abre processo: o app e a linha de
//! comando são adaptadores em cima dele.

mod erro;
mod livro;
mod saida;

pub use erro::ErroWorkspace;
pub use livro::{Capitulo, Gravacao, Livro};
pub use saida::SaidaPendente;
