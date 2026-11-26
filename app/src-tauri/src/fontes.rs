//! Leitura de fontes instaladas pelo usuário no macOS (`~/Library/Fonts` e
//! `/Library/Fonts`).
//!
//! O sandbox do `WKWebView` bloqueia fontes instaladas pelo usuário por
//! proteção contra fingerprinting (ADR 0003). Aqui lemos os arquivos `.ttf` e
//! `.otf` das pastas padrão do macOS, sem dependência nova nem `unsafe`, para
//! a página registrar via `FontFace` com `Uint8Array` sob o mesmo CSP estrito.

#[cfg(any(target_os = "macos", test))]
use std::fs::{self, File};
#[cfg(any(target_os = "macos", test))]
use std::io::Read;
#[cfg(any(target_os = "macos", test))]
use std::path::{Path, PathBuf};

use serde::Serialize;

/// Teto por arquivo de fonte lido em memória.
#[cfg(any(target_os = "macos", test))]
const TAMANHO_MAXIMO_FONTE: u64 = 32 * 1024 * 1024;
#[cfg(any(target_os = "macos", test))]
const MAXIMO_DE_FACES: usize = 32;
#[cfg(any(target_os = "macos", test))]
const MAXIMO_DE_ARQUIVOS: usize = 4096;
#[cfg(any(target_os = "macos", test))]
const PROFUNDIDADE_MAXIMA: usize = 4;
#[cfg(any(target_os = "macos", test))]
const MAXIMO_NOME_FAMILIA: usize = 200;

/// Uma face (peso + estilo) de uma família encontrada nas pastas de fontes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FaceDeFonte {
    pub peso: String,
    pub estilo: String,
    pub dados_base64: String,
}

#[cfg(any(target_os = "macos", test))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadadosFonte {
    pub familias: Vec<String>,
    pub peso: String,
    pub estilo: String,
}

/// Pastas padrão onde o macOS guarda fontes instaladas pelo usuário.
#[cfg(any(target_os = "macos", test))]
pub fn pastas_de_fontes_macos(home: Option<&Path>) -> Vec<PathBuf> {
    let mut pastas = Vec::with_capacity(2);
    if let Some(pasta) = home.filter(|pasta| pasta.is_absolute()) {
        pastas.push(pasta.join("Library/Fonts"));
    }
    pastas.push(PathBuf::from("/Library/Fonts"));
    pastas
}

#[cfg(any(target_os = "macos", test))]
fn u16_be(fatia: &[u8], offset: usize) -> Option<u16> {
    let bytes: [u8; 2] = fatia.get(offset..offset + 2)?.try_into().ok()?;
    Some(u16::from_be_bytes(bytes))
}

#[cfg(any(target_os = "macos", test))]
fn u32_be(fatia: &[u8], offset: usize) -> Option<u32> {
    let bytes: [u8; 4] = fatia.get(offset..offset + 4)?.try_into().ok()?;
    Some(u32::from_be_bytes(bytes))
}

#[cfg(any(target_os = "macos", test))]
fn i32_be(fatia: &[u8], offset: usize) -> Option<i32> {
    let bytes: [u8; 4] = fatia.get(offset..offset + 4)?.try_into().ok()?;
    Some(i32::from_be_bytes(bytes))
}

#[cfg(any(target_os = "macos", test))]
fn assinatura_sfnt_valida(assinatura: &[u8]) -> bool {
    assinatura == [0x00, 0x01, 0x00, 0x00] || assinatura == b"OTTO" || assinatura == b"true"
}

/// Lê um arquivo de fonte respeitando o teto de bytes no próprio descritor
/// aberto (sem TOCTOU) e descartando arquivos sem assinatura sfnt logo nos
/// primeiros 4 bytes.
#[cfg(any(target_os = "macos", test))]
fn ler_arquivo_sfnt_limitado(caminho: &Path, teto_bytes: u64) -> Option<Vec<u8>> {
    let mut arquivo = File::open(caminho).ok()?;
    let meta = arquivo.metadata().ok()?;
    if !meta.is_file() || meta.len() < 12 || meta.len() > teto_bytes {
        return None;
    }
    let mut assinatura = [0u8; 4];
    arquivo.read_exact(&mut assinatura).ok()?;
    if !assinatura_sfnt_valida(&assinatura) {
        return None;
    }
    let restante = teto_bytes.checked_sub(4)?;
    let mut bytes = Vec::with_capacity(usize::try_from(meta.len()).unwrap_or(0));
    bytes.extend_from_slice(&assinatura);
    arquivo
        .take(restante.saturating_add(1))
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > teto_bytes {
        return None;
    }
    Some(bytes)
}

/// Extrai famílias (Name ID 16 e 1), peso (`fvar` eixo `wght` ou `OS/2`) e
/// estilo (`OS/2` ou `head`) de um arquivo OpenType/TrueType (`.ttf`/`.otf`).
#[cfg(any(target_os = "macos", test))]
pub fn ler_metadados_sfnt(bytes: &[u8]) -> Option<MetadadosFonte> {
    let assinatura = bytes.get(0..4)?;
    if !assinatura_sfnt_valida(assinatura) {
        return None;
    }
    let num_tabelas = usize::from(u16_be(bytes, 4)?);
    let mut tabela_name: Option<&[u8]> = None;
    let mut tabela_os2: Option<&[u8]> = None;
    let mut tabela_head: Option<&[u8]> = None;
    let mut tabela_fvar: Option<&[u8]> = None;

    for indice in 0..num_tabelas {
        let inicio = 12usize.checked_add(indice.checked_mul(16)?)?;
        let tag = bytes.get(inicio..inicio + 4)?;
        let offset = usize::try_from(u32_be(bytes, inicio + 8)?).ok()?;
        let tamanho = usize::try_from(u32_be(bytes, inicio + 12)?).ok()?;
        let fim = offset.checked_add(tamanho)?;
        let corpo = bytes.get(offset..fim)?;
        match tag {
            b"name" => tabela_name = Some(corpo),
            b"OS/2" => tabela_os2 = Some(corpo),
            b"head" => tabela_head = Some(corpo),
            b"fvar" => tabela_fvar = Some(corpo),
            _ => {}
        }
    }

    let familias = ler_familias(tabela_name?)?;
    if familias.is_empty() {
        return None;
    }

    let peso = tabela_fvar
        .and_then(ler_faixa_wght)
        .or_else(|| tabela_os2.and_then(ler_peso_os2))
        .unwrap_or_else(|| "400".to_owned());

    let estilo = tabela_os2
        .and_then(ler_estilo_os2)
        .or_else(|| tabela_head.and_then(ler_estilo_head))
        .unwrap_or("normal")
        .to_owned();

    Some(MetadadosFonte {
        familias,
        peso,
        estilo,
    })
}

#[cfg(any(target_os = "macos", test))]
fn ler_familias(tabela: &[u8]) -> Option<Vec<String>> {
    let contagem = usize::from(u16_be(tabela, 2)?);
    let inicio_strings = usize::from(u16_be(tabela, 4)?);
    let mut achadas = Vec::new();
    for indice in 0..contagem {
        let pos = 6usize.checked_add(indice.checked_mul(12)?)?;
        let platform_id = u16_be(tabela, pos)?;
        let encoding_id = u16_be(tabela, pos + 2)?;
        let name_id = u16_be(tabela, pos + 6)?;
        if name_id != 1 && name_id != 16 {
            continue;
        }
        let tamanho = usize::from(u16_be(tabela, pos + 8)?);
        let offset = usize::from(u16_be(tabela, pos + 10)?);
        let inicio = inicio_strings.checked_add(offset)?;
        let fim = inicio.checked_add(tamanho)?;
        let bruto = tabela.get(inicio..fim)?;
        let texto = match (platform_id, encoding_id) {
            (0, _) | (3, 1 | 10) => {
                if bruto.len() % 2 != 0 {
                    continue;
                }
                let unidades = bruto
                    .chunks_exact(2)
                    .map(|par| u16::from_be_bytes([par[0], par[1]]));
                char::decode_utf16(unidades)
                    .map(|resultado| resultado.unwrap_or(char::REPLACEMENT_CHARACTER))
                    .collect::<String>()
            }
            (1, 0) => bruto
                .iter()
                .filter(|&&b| b < 128)
                .map(|&b| char::from(b))
                .collect::<String>(),
            _ => continue,
        };
        let limpo = texto.trim();
        if !limpo.is_empty() && !achadas.iter().any(|existente| existente == limpo) {
            achadas.push(limpo.to_owned());
        }
    }
    Some(achadas)
}

#[cfg(any(target_os = "macos", test))]
fn ler_faixa_wght(tabela: &[u8]) -> Option<String> {
    let inicio_eixos = usize::from(u16_be(tabela, 4)?);
    let contagem = usize::from(u16_be(tabela, 8)?);
    let tamanho_eixo = usize::from(u16_be(tabela, 10)?);
    if tamanho_eixo < 20 {
        return None;
    }
    for indice in 0..contagem {
        let pos = inicio_eixos.checked_add(indice.checked_mul(tamanho_eixo)?)?;
        if tabela.get(pos..pos + 4)? != b"wght" {
            continue;
        }
        // Valores Fixed 16.16 (parte inteira nos 16 bits mais altos).
        let minimo = i32_be(tabela, pos + 4)? >> 16;
        let maximo = i32_be(tabela, pos + 12)? >> 16;
        if (1..=1000).contains(&minimo) && (1..=1000).contains(&maximo) && minimo <= maximo {
            return Some(if minimo == maximo {
                minimo.to_string()
            } else {
                format!("{minimo} {maximo}")
            });
        }
    }
    None
}

#[cfg(any(target_os = "macos", test))]
fn ler_peso_os2(tabela: &[u8]) -> Option<String> {
    let peso = u16_be(tabela, 4)?;
    (1..=1000).contains(&peso).then(|| peso.to_string())
}

#[cfg(any(target_os = "macos", test))]
fn ler_estilo_os2(tabela: &[u8]) -> Option<&'static str> {
    let selecao = u16_be(tabela, 62)?;
    if selecao & 0x0001 != 0 {
        Some("italic")
    } else if selecao & 0x0200 != 0 {
        Some("oblique")
    } else {
        Some("normal")
    }
}

#[cfg(any(target_os = "macos", test))]
fn ler_estilo_head(tabela: &[u8]) -> Option<&'static str> {
    let mac_style = u16_be(tabela, 44)?;
    Some(if mac_style & 0x0002 != 0 {
        "italic"
    } else {
        "normal"
    })
}

#[cfg(any(target_os = "macos", test))]
fn extensao_de_fonte(caminho: &Path) -> bool {
    caminho
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("ttf") || ext.eq_ignore_ascii_case("otf"))
}

#[cfg(any(target_os = "macos", test))]
pub fn codificar_base64(bytes: &[u8]) -> String {
    const TABELA: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut saida = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for bloco in bytes.chunks(3) {
        let b0 = bloco[0];
        let b1 = bloco.get(1).copied().unwrap_or(0);
        let b2 = bloco.get(2).copied().unwrap_or(0);
        saida.push(char::from(TABELA[usize::from(b0 >> 2)]));
        saida.push(char::from(
            TABELA[usize::from(((b0 & 0x03) << 4) | (b1 >> 4))],
        ));
        if bloco.len() > 1 {
            saida.push(char::from(
                TABELA[usize::from(((b1 & 0x0f) << 2) | (b2 >> 6))],
            ));
        } else {
            saida.push('=');
        }
        if bloco.len() > 2 {
            saida.push(char::from(TABELA[usize::from(b2 & 0x3f)]));
        } else {
            saida.push('=');
        }
    }
    saida
}

#[cfg(any(target_os = "macos", test))]
fn carregar_com_teto(familia: &str, pastas: &[PathBuf], teto_bytes: u64) -> Vec<FaceDeFonte> {
    let alvo = familia.trim().to_lowercase();
    if alvo.is_empty() || alvo.chars().count() > MAXIMO_NOME_FAMILIA {
        return Vec::new();
    }

    let mut faces = Vec::new();
    let mut vistos = 0usize;

    fn percorrer(
        pasta: &Path,
        profundidade: usize,
        alvo: &str,
        teto_bytes: u64,
        vistos: &mut usize,
        faces: &mut Vec<FaceDeFonte>,
    ) {
        if profundidade > PROFUNDIDADE_MAXIMA
            || *vistos >= MAXIMO_DE_ARQUIVOS
            || faces.len() >= MAXIMO_DE_FACES
        {
            return;
        }
        let Ok(entradas) = fs::read_dir(pasta) else {
            return;
        };
        let mut caminhos: Vec<PathBuf> =
            entradas.filter_map(|e| e.ok().map(|e| e.path())).collect();
        caminhos.sort();
        for caminho in caminhos {
            if *vistos >= MAXIMO_DE_ARQUIVOS || faces.len() >= MAXIMO_DE_FACES {
                break;
            }
            let Ok(meta_link) = fs::symlink_metadata(&caminho) else {
                continue;
            };
            if meta_link.file_type().is_dir() {
                // `file_type().is_dir()` em `symlink_metadata` é falso para symlinks,
                // evitando sair da pasta de fontes ou entrar em laços.
                percorrer(&caminho, profundidade + 1, alvo, teto_bytes, vistos, faces);
                continue;
            }
            if !extensao_de_fonte(&caminho) {
                continue;
            }
            *vistos += 1;
            let Some(bytes) = ler_arquivo_sfnt_limitado(&caminho, teto_bytes) else {
                continue;
            };
            let Some(metadados) = ler_metadados_sfnt(&bytes) else {
                continue;
            };
            if metadados
                .familias
                .iter()
                .any(|nome| nome.to_lowercase() == alvo)
            {
                faces.push(FaceDeFonte {
                    peso: metadados.peso,
                    estilo: metadados.estilo,
                    dados_base64: codificar_base64(&bytes),
                });
            }
        }
    }

    for pasta in pastas {
        percorrer(pasta, 0, &alvo, teto_bytes, &mut vistos, &mut faces);
    }
    faces
}

#[cfg(any(target_os = "macos", test))]
pub fn carregar_das_pastas(familia: &str, pastas: &[PathBuf]) -> Vec<FaceDeFonte> {
    carregar_com_teto(familia, pastas, TAMANHO_MAXIMO_FONTE)
}

#[cfg(target_os = "macos")]
pub fn carregar_do_mac(familia: &str) -> Vec<FaceDeFonte> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    carregar_das_pastas(familia, &pastas_de_fontes_macos(home.as_deref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Monta um arquivo OpenType/TrueType mínimo válido para testar o parser
    /// sem depender de nenhuma fonte instalada na máquina do CI.
    fn montar_ttf(
        familia: &str,
        familia_tipografica: Option<&str>,
        peso_os2: u16,
        fs_selection: u16,
        faixa_wght: Option<(i32, i32)>,
    ) -> Vec<u8> {
        let mut registros_nome: Vec<(u16, Vec<u8>)> = Vec::new();
        let utf16be = |texto: &str| -> Vec<u8> {
            texto.encode_utf16().flat_map(|u| u.to_be_bytes()).collect()
        };
        registros_nome.push((1, utf16be(familia)));
        if let Some(tipo) = familia_tipografica {
            registros_nome.push((16, utf16be(tipo)));
        }

        let mut strings = Vec::new();
        let mut tabela_name = Vec::new();
        tabela_name.extend_from_slice(&0u16.to_be_bytes());
        tabela_name.extend_from_slice(&(registros_nome.len() as u16).to_be_bytes());
        let string_offset = 6u16 + (registros_nome.len() as u16) * 12;
        tabela_name.extend_from_slice(&string_offset.to_be_bytes());
        for (name_id, bytes) in &registros_nome {
            let offset = strings.len() as u16;
            strings.extend_from_slice(bytes);
            tabela_name.extend_from_slice(&3u16.to_be_bytes());
            tabela_name.extend_from_slice(&1u16.to_be_bytes());
            tabela_name.extend_from_slice(&0x0409u16.to_be_bytes());
            tabela_name.extend_from_slice(&name_id.to_be_bytes());
            tabela_name.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
            tabela_name.extend_from_slice(&offset.to_be_bytes());
        }
        tabela_name.extend_from_slice(&strings);

        let mut tabela_os2 = vec![0u8; 64];
        tabela_os2[4..6].copy_from_slice(&peso_os2.to_be_bytes());
        tabela_os2[62..64].copy_from_slice(&fs_selection.to_be_bytes());

        let mut tabelas: Vec<([u8; 4], Vec<u8>)> =
            vec![(*b"OS/2", tabela_os2), (*b"name", tabela_name)];

        if let Some((min_w, max_w)) = faixa_wght {
            let mut fvar = Vec::new();
            fvar.extend_from_slice(&1u16.to_be_bytes());
            fvar.extend_from_slice(&0u16.to_be_bytes());
            fvar.extend_from_slice(&16u16.to_be_bytes());
            fvar.extend_from_slice(&2u16.to_be_bytes());
            fvar.extend_from_slice(&1u16.to_be_bytes());
            fvar.extend_from_slice(&20u16.to_be_bytes());
            fvar.extend_from_slice(&0u16.to_be_bytes());
            fvar.extend_from_slice(&0u16.to_be_bytes());
            fvar.extend_from_slice(b"wght");
            fvar.extend_from_slice(&(min_w << 16).to_be_bytes());
            fvar.extend_from_slice(&(min_w << 16).to_be_bytes());
            fvar.extend_from_slice(&(max_w << 16).to_be_bytes());
            fvar.extend_from_slice(&0u16.to_be_bytes());
            fvar.extend_from_slice(&256u16.to_be_bytes());
            tabelas.push((*b"fvar", fvar));
        }

        let num_tabelas = tabelas.len() as u16;
        let mut saida = Vec::new();
        saida.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
        saida.extend_from_slice(&num_tabelas.to_be_bytes());
        saida.extend_from_slice(&0u16.to_be_bytes());
        saida.extend_from_slice(&0u16.to_be_bytes());
        saida.extend_from_slice(&0u16.to_be_bytes());

        let mut offset = 12 + tabelas.len() * 16;
        for (tag, corpo) in &tabelas {
            saida.extend_from_slice(tag);
            saida.extend_from_slice(&0u32.to_be_bytes());
            saida.extend_from_slice(&(offset as u32).to_be_bytes());
            saida.extend_from_slice(&(corpo.len() as u32).to_be_bytes());
            offset += (corpo.len() + 3) & !3;
        }
        for (_, corpo) in &tabelas {
            saida.extend_from_slice(corpo);
            while saida.len() % 4 != 0 {
                saida.push(0);
            }
        }
        saida
    }

    #[test]
    fn pastas_no_mac_ignoram_home_relativo() {
        assert_eq!(
            pastas_de_fontes_macos(Some(Path::new("/Users/autora"))),
            vec![
                PathBuf::from("/Users/autora/Library/Fonts"),
                PathBuf::from("/Library/Fonts"),
            ]
        );
        assert_eq!(
            pastas_de_fontes_macos(Some(Path::new("relativo"))),
            vec![PathBuf::from("/Library/Fonts")]
        );
        assert_eq!(
            pastas_de_fontes_macos(None),
            vec![PathBuf::from("/Library/Fonts")]
        );
    }

    #[test]
    fn parser_sfnt_le_fonte_estatica_e_fonte_variavel_com_eixo_wght() {
        let estatica = montar_ttf("Fonte Teste Bold", Some("Fonte Teste"), 700, 0x0001, None);
        let meta = ler_metadados_sfnt(&estatica).expect("ler fonte estática");
        assert!(meta.familias.iter().any(|f| f == "Fonte Teste"));
        assert_eq!(meta.peso, "700");
        assert_eq!(meta.estilo, "italic");

        let variavel = montar_ttf("EB Garamond", None, 400, 0x0000, Some((400, 800)));
        let meta_var = ler_metadados_sfnt(&variavel).expect("ler fonte variável");
        assert_eq!(meta_var.familias, ["EB Garamond"]);
        assert_eq!(meta_var.peso, "400 800");
        assert_eq!(meta_var.estilo, "normal");

        assert_eq!(ler_metadados_sfnt(b"nao e uma fonte"), None);
    }

    #[test]
    fn base64_codifica_os_casos_da_rfc() {
        assert_eq!(codificar_base64(b""), "");
        assert_eq!(codificar_base64(b"f"), "Zg==");
        assert_eq!(codificar_base64(b"fo"), "Zm8=");
        assert_eq!(codificar_base64(b"foo"), "Zm9v");
        assert_eq!(codificar_base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn carregar_das_pastas_acha_so_a_familia_pedida_e_ignora_lixo() {
        let pasta = tempfile::tempdir().expect("pasta temporária");
        let sub = pasta.path().join("eb");
        fs::create_dir(&sub).expect("subpasta");

        let regular = montar_ttf("EB Garamond", None, 400, 0x0000, Some((400, 800)));
        let italica = montar_ttf("EB Garamond", None, 400, 0x0001, Some((400, 800)));
        let outra = montar_ttf("Outra Fonte", None, 400, 0x0000, None);

        fs::write(sub.join("EBGaramond.ttf"), &regular).expect("gravar regular");
        fs::write(sub.join("EBGaramond-Italic.ttf"), &italica).expect("gravar itálica");
        fs::write(pasta.path().join("Outra.otf"), &outra).expect("gravar outra");
        fs::write(pasta.path().join("notas.txt"), b"ignorar").expect("gravar txt");
        fs::write(pasta.path().join("quebrada.ttf"), b"lixo").expect("gravar quebrada");

        let faces = carregar_das_pastas("eb garamond", &[pasta.path().to_path_buf()]);
        assert_eq!(faces.len(), 2);
        assert!(
            faces
                .iter()
                .any(|f| f.estilo == "normal" && f.peso == "400 800")
        );
        assert!(
            faces
                .iter()
                .any(|f| f.estilo == "italic" && f.peso == "400 800")
        );
        assert!(carregar_das_pastas("", &[pasta.path().to_path_buf()]).is_empty());
    }

    #[test]
    fn arquivo_acima_do_teto_e_ignorado() {
        let pasta = tempfile::tempdir().expect("pasta temporária");
        let fonte = montar_ttf("Fonte Grande", None, 400, 0x0000, None);
        fs::write(pasta.path().join("grande.ttf"), &fonte).expect("gravar fonte");

        let abaixo = carregar_com_teto(
            "Fonte Grande",
            &[pasta.path().to_path_buf()],
            fonte.len() as u64,
        );
        assert_eq!(abaixo.len(), 1);

        let acima = carregar_com_teto(
            "Fonte Grande",
            &[pasta.path().to_path_buf()],
            (fonte.len() - 1) as u64,
        );
        assert!(acima.is_empty(), "arquivo acima do teto deve ser ignorado");
    }

    #[test]
    fn guardas_de_nome_profundidade_e_maximo_de_faces_valem() {
        let pasta = tempfile::tempdir().expect("pasta temporária");
        let fonte = montar_ttf("Familia Limite", None, 400, 0x0000, None);

        // Nome acima do teto (200 chars) é recusado mesmo com fonte na pasta.
        let nome_longo = "a".repeat(MAXIMO_NOME_FAMILIA + 1);
        let fonte_longa = montar_ttf(&nome_longo, None, 400, 0x0000, None);
        fs::write(pasta.path().join("longa.ttf"), &fonte_longa).expect("gravar fonte longa");
        assert!(carregar_das_pastas(&nome_longo, &[pasta.path().to_path_buf()]).is_empty());

        // Profundidade acima de PROFUNDIDADE_MAXIMA não é alcançada.
        let mut funda = pasta.path().to_path_buf();
        for nivel in 0..=PROFUNDIDADE_MAXIMA {
            funda = funda.join(format!("n{nivel}"));
        }
        fs::create_dir_all(&funda).expect("criar pasta funda");
        let fonte_funda = montar_ttf("Familia Funda", None, 400, 0x0000, None);
        fs::write(funda.join("funda.ttf"), &fonte_funda).expect("gravar fonte funda");
        assert!(carregar_das_pastas("Familia Funda", &[pasta.path().to_path_buf()]).is_empty());

        // Mais que MAXIMO_DE_FACES na mesma família para no teto.
        for i in 0..(MAXIMO_DE_FACES + 3) {
            fs::write(pasta.path().join(format!("f{i:02}.ttf")), &fonte).expect("gravar face");
        }
        let faces = carregar_das_pastas("Familia Limite", &[pasta.path().to_path_buf()]);
        assert_eq!(faces.len(), MAXIMO_DE_FACES);
    }

    #[cfg(unix)]
    #[test]
    fn carregar_das_pastas_nao_segue_symlink_de_diretorio() {
        use std::os::unix::fs::symlink;

        let fora = tempfile::tempdir().expect("pasta externa");
        let fonte_fora = montar_ttf("Fonte Mista", None, 700, 0x0001, None);
        fs::write(fora.path().join("externa.ttf"), &fonte_fora).expect("gravar externa");

        let pasta_fontes = tempfile::tempdir().expect("pasta de fontes");
        let fonte_local = montar_ttf("Fonte Mista", None, 400, 0x0000, None);
        fs::write(pasta_fontes.path().join("local.ttf"), &fonte_local).expect("gravar local");
        symlink(fora.path(), pasta_fontes.path().join("atalho")).expect("criar symlink");

        // Deve achar a face local e ignorar a que está atrás do symlink de diretório.
        let faces = carregar_das_pastas("Fonte Mista", &[pasta_fontes.path().to_path_buf()]);
        assert_eq!(faces.len(), 1, "não deve descer em symlink de diretório");
        assert_eq!(faces[0].peso, "400");
    }
}
