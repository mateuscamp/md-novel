//! Compilador de estilo: um `estilo.yaml` vira o CSS do editor e o
//! `reference.docx` do Pandoc, de modo que a tela e o .docx recebem as mesmas
//! propriedades (ADR 0002).
//!
//! Regra do subconjunto: toda propriedade de [`EstiloParagrafo`] é emitida de
//! forma explícita nos dois lados. Propriedade nova entra no struct e os dois
//! geradores param de compilar até traduzi-la, porque ambos desestruturam o
//! struct sem `..`.
//!
//! Este crate não chama processo externo nem rede: o modelo do reference.docx
//! chega como bytes, entregue por `crates/exportar`.

mod css;
mod docx;
mod erro;
mod medida;
mod modelo;

pub use css::{SeletorCss, gerar_css};
pub use docx::gerar_reference_docx;
pub use erro::ErroEstilo;
pub use medida::{Medida, Unidade};
pub use modelo::{
    Alinhamento, Campos, CamposTrocados, Entrelinha, Estilo, EstiloParagrafo, Fonte, TamanhoFonte,
};
