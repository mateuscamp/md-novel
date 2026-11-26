//! `app/macos/instalar.sh`, o que o `make instalar` e o `make desinstalar`
//! chamam no macOS: monta o pacote `md Novel.app` numa pasta (por padrão
//! `~/Applications`) e tira de lá. Os testes rodam o script de verdade numa
//! pasta temporária, com ferramentas de mentira no PATH para valer também no
//! Linux do CI, e com as ferramentas reais do macOS quando rodam no Mac.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

const PACOTE: &str = "md Novel.app";
const BINARIO: &str = "md Novel.app/Contents/MacOS/md-novel-app";
const PLIST: &str = "md Novel.app/Contents/Info.plist";
const ICONE: &str = "md Novel.app/Contents/Resources/icon.icns";

fn raiz() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn campo_tauri_conf(chave: &str) -> String {
    let configuracao =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json"))
            .expect("ler o tauri.conf.json");
    let busca = format!("\"{chave}\": \"");
    let inicio = configuracao
        .find(&busca)
        .unwrap_or_else(|| panic!("achar {chave} no tauri.conf.json"))
        + busca.len();
    let fim = configuracao[inicio..]
        .find('"')
        .unwrap_or_else(|| panic!("fim de {chave} no tauri.conf.json"));
    configuracao[inicio..inicio + fim].to_owned()
}

fn identificador() -> String {
    campo_tauri_conf("identifier")
}

fn versao() -> String {
    campo_tauri_conf("version")
}

fn rascunhos(apoio: &Path) -> PathBuf {
    apoio.join("rascunhos")
}

fn registro_codesign(apoio: &Path) -> PathBuf {
    apoio.join("codesign.log")
}

fn script() -> PathBuf {
    raiz().join("app/macos/instalar.sh")
}

fn escrever_executavel(caminho: &Path, conteudo: &str) {
    fs::write(caminho, conteudo).expect("escrever executável de teste");
    fs::set_permissions(caminho, fs::Permissions::from_mode(0o755)).expect("tornar executável");
}

/// Ferramentas de mentira na frente do PATH para que a lógica inteira do
/// instalador macOS rode no CI do Linux e no Mac sem depender de `sips`,
/// `iconutil` nem `codesign`.
struct Ferramentas<'a> {
    sistema: &'a str,
    com_sips: bool,
    com_iconutil: bool,
    iconutil_falha: bool,
}

impl Default for Ferramentas<'_> {
    fn default() -> Self {
        Self {
            sistema: "Darwin",
            com_sips: true,
            com_iconutil: true,
            iconutil_falha: false,
        }
    }
}

fn preparar_ferramentas(apoio: &Path, opcoes: &Ferramentas<'_>) -> PathBuf {
    let bin = apoio.join("ferramentas-de-mentira");
    let _ = fs::remove_dir_all(&bin);
    fs::create_dir_all(&bin).expect("criar a pasta das ferramentas de mentira");

    escrever_executavel(
        &bin.join("uname"),
        &format!("#!/bin/sh\necho {}\n", opcoes.sistema),
    );

    if opcoes.com_sips {
        // `sips -z <h> <w> <src> --out <dst>`: copia a origem para o destino.
        escrever_executavel(
            &bin.join("sips"),
            "#!/bin/sh\n\
             origem=\"\"\n\
             destino=\"\"\n\
             while [ $# -gt 0 ]; do\n\
               case \"$1\" in\n\
                 -z) shift 3 ;;\n\
                 --out) destino=$2; shift 2 ;;\n\
                 *) origem=$1; shift ;;\n\
               esac\n\
             done\n\
             cp \"$origem\" \"$destino\"\n",
        );
    }

    if opcoes.com_iconutil {
        let corpo = if opcoes.iconutil_falha {
            "#!/bin/sh\necho 'iconutil falhou de proposito' >&2\nexit 1\n".to_owned()
        } else {
            // `iconutil -c icns <iconset> -o <saida>`
            "#!/bin/sh\n\
             saida=\"\"\n\
             while [ $# -gt 0 ]; do\n\
               case \"$1\" in\n\
                 -o) saida=$2; shift 2 ;;\n\
                 *) shift ;;\n\
               esac\n\
             done\n\
             printf 'icns-simulado' > \"$saida\"\n"
                .to_owned()
        };
        escrever_executavel(&bin.join("iconutil"), &corpo);
    }

    escrever_executavel(
        &bin.join("codesign"),
        &format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\n",
            registro_codesign(apoio).to_str().expect("utf-8")
        ),
    );

    bin
}

fn rodar_o_script(
    script: &Path,
    ferramentas: &Ferramentas<'_>,
    apoio: &Path,
    argumentos: &[&str],
) -> Output {
    let bin = preparar_ferramentas(apoio, ferramentas);
    fs::create_dir_all(rascunhos(apoio)).expect("criar a pasta dos rascunhos");

    // Quando o teste quer simular ausência de `sips` ou `iconutil`, não pode
    // herdar `/usr/bin` no PATH (onde o macOS tem os reais): usa só `bin` e
    // copia ou aponta utilitários básicos. Em vez de filtrar `/usr/bin`, o
    // teste de ferramenta ausente põe um PATH mínimo só com os comandos POSIX
    // comuns via link, sem `sips`/`iconutil`.
    let path_valor = if !ferramentas.com_sips || !ferramentas.com_iconutil {
        for util in [
            "sh", "mkdir", "cp", "mv", "rm", "chmod", "dirname", "mktemp", "tr", "printf", "grep",
        ] {
            for pasta in ["/bin", "/usr/bin"] {
                let candidato = Path::new(pasta).join(util);
                if candidato.exists() {
                    let _ = symlink(&candidato, bin.join(util));
                    break;
                }
            }
        }
        bin.to_str().expect("utf-8").to_owned()
    } else {
        let caminho = std::env::var("PATH").expect("PATH");
        format!("{}:{caminho}", bin.to_str().expect("utf-8"))
    };

    Command::new("sh")
        .arg(script)
        .args(argumentos)
        .env("PATH", path_valor)
        .env("TMPDIR", rascunhos(apoio))
        .output()
        .expect("rodar o app/macos/instalar.sh")
}

fn rodar(apoio: &Path, argumentos: &[&str]) -> Output {
    rodar_o_script(&script(), &Ferramentas::default(), apoio, argumentos)
}

fn erro(saida: &Output) -> String {
    String::from_utf8_lossy(&saida.stderr).into_owned()
}

struct Bancada {
    apoio: TempDir,
    pasta: PathBuf,
    compilado: PathBuf,
}

impl Bancada {
    fn nova() -> Self {
        let apoio = tempfile::tempdir().expect("pasta temporária");
        let pasta = apoio.path().join("pasta do autor/Applications");
        let compilado = apoio.path().join("md-novel-app");
        fs::write(&compilado, b"programa, primeira compilacao").expect("escrever o programa");
        Self {
            apoio,
            pasta,
            compilado,
        }
    }

    fn instalar(&self) -> Output {
        rodar(
            self.apoio.path(),
            &[
                "instalar",
                self.pasta.to_str().expect("utf-8"),
                self.compilado.to_str().expect("utf-8"),
            ],
        )
    }

    fn desinstalar(&self) -> Output {
        rodar(
            self.apoio.path(),
            &["desinstalar", self.pasta.to_str().expect("utf-8")],
        )
    }

    fn arquivos(&self) -> Vec<String> {
        fn descer(pasta: &Path, base: &Path, achados: &mut Vec<String>) {
            let Ok(entradas) = fs::read_dir(pasta) else {
                return;
            };
            for entrada in entradas {
                let caminho = entrada.expect("ler a entrada").path();
                let meta = fs::symlink_metadata(&caminho).expect("metadados");
                if meta.is_dir() {
                    descer(&caminho, base, achados);
                } else {
                    let relativo = caminho.strip_prefix(base).expect("dentro da pasta");
                    achados.push(relativo.to_str().expect("utf-8").to_owned());
                }
            }
        }
        let mut achados = Vec::new();
        descer(&self.pasta, &self.pasta, &mut achados);
        achados.sort();
        achados
    }

    fn rascunhos_esquecidos(&self) -> Vec<PathBuf> {
        fs::read_dir(rascunhos(self.apoio.path()))
            .expect("ler a pasta dos rascunhos")
            .map(|entrada| entrada.expect("ler a entrada").path())
            .collect()
    }
}

#[test]
fn instala_o_pacote_app_com_binario_icone_e_info_plist_e_mais_nada() {
    let bancada = Bancada::nova();
    let saida = bancada.instalar();
    assert!(saida.status.success(), "{}", erro(&saida));

    let mut esperados = vec![BINARIO.to_owned(), ICONE.to_owned(), PLIST.to_owned()];
    esperados.sort();
    assert_eq!(bancada.arquivos(), esperados);
    assert_eq!(bancada.rascunhos_esquecidos(), Vec::<PathBuf>::new());

    let binario = bancada.pasta.join(BINARIO);
    assert_eq!(
        fs::read(&binario).expect("ler o binário instalado"),
        b"programa, primeira compilacao"
    );
    let modo = fs::metadata(&binario)
        .expect("metadados do binário")
        .permissions()
        .mode();
    assert_eq!(modo & 0o777, 0o755, "o binário instalado é executável");

    assert_eq!(
        fs::read(bancada.pasta.join(ICONE)).expect("ler o ícone"),
        b"icns-simulado"
    );

    let plist = fs::read_to_string(bancada.pasta.join(PLIST)).expect("ler o Info.plist");
    for trecho in [
        format!(
            "<key>CFBundleIdentifier</key>\n  <string>{}</string>",
            identificador()
        ),
        "<key>CFBundleName</key>\n  <string>md Novel</string>".to_owned(),
        "<key>CFBundleDisplayName</key>\n  <string>md Novel</string>".to_owned(),
        "<key>CFBundleExecutable</key>\n  <string>md-novel-app</string>".to_owned(),
        "<key>CFBundleIconFile</key>\n  <string>icon</string>".to_owned(),
        "<key>CFBundlePackageType</key>\n  <string>APPL</string>".to_owned(),
        format!(
            "<key>CFBundleShortVersionString</key>\n  <string>{}</string>",
            versao()
        ),
        format!(
            "<key>CFBundleVersion</key>\n  <string>{}</string>",
            versao()
        ),
        "<key>NSHighResolutionCapable</key>\n  <true/>".to_owned(),
    ] {
        assert!(plist.contains(&trecho), "falta {trecho:?} em:\n{plist}");
    }

    let assinatura =
        fs::read_to_string(registro_codesign(bancada.apoio.path())).expect("ler log do codesign");
    assert!(
        assinatura.contains("--force --deep --sign -"),
        "esperava assinatura ad-hoc, veio: {assinatura}"
    );
}

#[test]
fn instalar_de_novo_troca_o_programa_e_nao_deixa_temporario_nem_backup() {
    let bancada = Bancada::nova();
    assert!(bancada.instalar().status.success());

    fs::write(&bancada.compilado, b"programa, segunda compilacao").expect("recompilar");
    let saida = bancada.instalar();
    assert!(saida.status.success(), "{}", erro(&saida));

    assert_eq!(
        fs::read(bancada.pasta.join(BINARIO)).expect("ler o binário atualizado"),
        b"programa, segunda compilacao"
    );
    assert_eq!(bancada.arquivos().len(), 3, "{:?}", bancada.arquivos());
    // Só existe `md Novel.app` na raiz da pasta de aplicativos.
    let entradas_raiz: Vec<_> = fs::read_dir(&bancada.pasta)
        .expect("listar pasta")
        .map(|e| e.expect("entrada").file_name())
        .collect();
    assert_eq!(entradas_raiz, [PACOTE]);
}

#[test]
fn sem_o_programa_compilado_recusa_diz_como_compilar_e_nao_escreve_nada() {
    let bancada = Bancada::nova();
    fs::remove_file(&bancada.compilado).expect("apagar o programa");
    let saida = bancada.instalar();

    assert!(!saida.status.success());
    let mensagem = erro(&saida);
    assert!(
        mensagem.contains(bancada.compilado.to_str().expect("utf-8")),
        "{mensagem}"
    );
    assert!(
        mensagem.contains("cargo tauri build --no-bundle"),
        "{mensagem}"
    );
    assert!(bancada.arquivos().is_empty(), "{:?}", bancada.arquivos());
}

#[test]
fn sem_o_icone_do_projeto_recusa_antes_de_escrever() {
    let bancada = Bancada::nova();
    let copia = bancada.apoio.path().join("copia/app/macos/instalar.sh");
    fs::create_dir_all(copia.parent().expect("pasta da cópia")).expect("criar pasta");
    fs::copy(script(), &copia).expect("copiar o script");

    let saida = rodar_o_script(
        &copia,
        &Ferramentas::default(),
        bancada.apoio.path(),
        &[
            "instalar",
            bancada.pasta.to_str().expect("utf-8"),
            bancada.compilado.to_str().expect("utf-8"),
        ],
    );
    assert!(!saida.status.success());
    let mensagem = erro(&saida);
    assert!(
        mensagem.contains("não achei o ícone do projeto"),
        "{mensagem}"
    );
    assert!(
        mensagem.contains("src-tauri/icons/icon-source.png"),
        "{mensagem}"
    );
    assert!(!bancada.pasta.exists(), "{:?}", bancada.arquivos());
    assert_eq!(bancada.rascunhos_esquecidos(), Vec::<PathBuf>::new());
}

#[test]
fn falha_no_meio_nao_corrompe_instalacao_anterior_nem_deixa_temporario() {
    let bancada = Bancada::nova();
    assert!(bancada.instalar().status.success());

    fs::write(&bancada.compilado, b"programa, segunda compilacao").expect("recompilar");
    let saida = rodar_o_script(
        &script(),
        &Ferramentas {
            iconutil_falha: true,
            ..Ferramentas::default()
        },
        bancada.apoio.path(),
        &[
            "instalar",
            bancada.pasta.to_str().expect("utf-8"),
            bancada.compilado.to_str().expect("utf-8"),
        ],
    );
    assert!(!saida.status.success());
    assert_eq!(
        fs::read(bancada.pasta.join(BINARIO)).expect("instalação anterior preservada"),
        b"programa, primeira compilacao"
    );
    assert_eq!(bancada.arquivos().len(), 3, "{:?}", bancada.arquivos());
    assert_eq!(bancada.rascunhos_esquecidos(), Vec::<PathBuf>::new());
}

#[test]
fn recusa_destino_que_e_symlink_ou_arquivo_comum() {
    let bancada = Bancada::nova();
    fs::create_dir_all(&bancada.pasta).expect("criar pasta");

    let alvo_real = bancada.apoio.path().join("pasta-importante");
    fs::create_dir_all(&alvo_real).expect("criar alvo");
    fs::write(alvo_real.join("manuscrito.md"), b"sagrado").expect("escrever manuscrito");
    symlink(&alvo_real, bancada.pasta.join(PACOTE)).expect("criar symlink");

    for saida in [
        rodar(
            bancada.apoio.path(),
            &["conferir", bancada.pasta.to_str().expect("utf-8")],
        ),
        bancada.instalar(),
        bancada.desinstalar(),
    ] {
        assert!(!saida.status.success(), "aceitou symlink no destino");
        let mensagem = erro(&saida);
        assert!(mensagem.contains("link"), "{mensagem}");
        assert_eq!(
            fs::read(alvo_real.join("manuscrito.md")).expect("alvo intacto"),
            b"sagrado"
        );
    }

    fs::remove_file(bancada.pasta.join(PACOTE)).expect("remover symlink");
    fs::write(bancada.pasta.join(PACOTE), b"arquivo comum").expect("criar arquivo");
    for saida in [bancada.instalar(), bancada.desinstalar()] {
        assert!(!saida.status.success(), "aceitou arquivo comum no destino");
        assert_eq!(
            fs::read(bancada.pasta.join(PACOTE)).expect("arquivo intacto"),
            b"arquivo comum"
        );
    }
}

#[test]
fn prefixo_invalido_e_recusado_com_o_valor_recebido() {
    let bancada = Bancada::nova();
    let base = bancada.apoio.path().to_str().expect("utf-8").to_owned();
    let recusados = [
        "relativo/Applications".to_owned(),
        "/".to_owned(),
        format!("{base}/com\nquebra"),
        format!("{base}/com\ttabulacao"),
    ];
    for prefixo in recusados {
        let saida = rodar(
            bancada.apoio.path(),
            &[
                "instalar",
                &prefixo,
                bancada.compilado.to_str().expect("utf-8"),
            ],
        );
        assert!(!saida.status.success(), "aceitou o prefixo {prefixo:?}");
        let mensagem = erro(&saida);
        assert!(mensagem.contains("prefixo"), "{prefixo:?}: {mensagem}");
        assert!(
            mensagem.contains(prefixo.trim_end()),
            "{prefixo:?}: {mensagem}"
        );
    }
}

#[test]
fn fora_do_macos_recusa_dizendo_o_sistema_e_nao_escreve_nada() {
    let bancada = Bancada::nova();
    let pasta = bancada.pasta.to_str().expect("utf-8");
    let compilado = bancada.compilado.to_str().expect("utf-8");
    let chamadas: [&[&str]; 3] = [
        &["instalar", pasta, compilado],
        &["desinstalar", pasta],
        &["conferir", pasta],
    ];
    for argumentos in chamadas {
        let comando = argumentos[0];
        let saida = rodar_o_script(
            &script(),
            &Ferramentas {
                sistema: "Linux",
                ..Ferramentas::default()
            },
            bancada.apoio.path(),
            argumentos,
        );
        assert!(!saida.status.success(), "{comando} passou fora do macOS");
        let mensagem = erro(&saida);
        assert!(mensagem.contains("macOS"), "{comando}: {mensagem}");
        assert!(mensagem.contains("Linux"), "{comando}: {mensagem}");
        assert!(bancada.arquivos().is_empty(), "{:?}", bancada.arquivos());
    }
}

#[test]
fn conferir_aceita_o_valido_e_recusa_quando_falta_sips_ou_iconutil() {
    let bancada = Bancada::nova();
    let pasta = bancada.pasta.to_str().expect("utf-8");
    let ok = rodar(bancada.apoio.path(), &["conferir", pasta]);
    assert!(ok.status.success(), "{}", erro(&ok));
    assert!(!bancada.pasta.exists());

    let sem_sips = rodar_o_script(
        &script(),
        &Ferramentas {
            com_sips: false,
            ..Ferramentas::default()
        },
        bancada.apoio.path(),
        &["conferir", pasta],
    );
    assert!(!sem_sips.status.success());
    assert!(erro(&sem_sips).contains("sips"), "{}", erro(&sem_sips));

    let sem_iconutil = rodar_o_script(
        &script(),
        &Ferramentas {
            com_iconutil: false,
            ..Ferramentas::default()
        },
        bancada.apoio.path(),
        &["conferir", pasta],
    );
    assert!(!sem_iconutil.status.success());
    assert!(
        erro(&sem_iconutil).contains("iconutil"),
        "{}",
        erro(&sem_iconutil)
    );
}

#[test]
fn desinstalar_apaga_so_o_md_novel_e_preserva_vizinhos_e_outro_app() {
    let bancada = Bancada::nova();
    assert!(bancada.instalar().status.success());

    let vizinho = bancada.pasta.join("Outro.app/Contents/Info.plist");
    fs::create_dir_all(vizinho.parent().expect("pasta")).expect("criar vizinho");
    fs::write(&vizinho, b"outro app").expect("escrever vizinho");

    let saida = bancada.desinstalar();
    assert!(saida.status.success(), "{}", erro(&saida));
    assert_eq!(bancada.arquivos(), ["Outro.app/Contents/Info.plist"]);

    // Sem nada instalado, desinstalar de novo não é erro.
    let de_novo = bancada.desinstalar();
    assert!(de_novo.status.success(), "{}", erro(&de_novo));
    assert_eq!(bancada.arquivos(), ["Outro.app/Contents/Info.plist"]);

    // Se existir um `md Novel.app` que não pertence ao nosso identificador,
    // desinstalar recusa em vez de apagar cegamente.
    let estranho = bancada.pasta.join(PLIST);
    fs::create_dir_all(estranho.parent().expect("pasta")).expect("criar estranho");
    fs::write(&estranho, b"<string>com.outro.app</string>").expect("escrever estranho");

    let recusado = bancada.desinstalar();
    assert!(!recusado.status.success(), "apagou pacote de outro app");
    assert!(estranho.is_file(), "o pacote estranho deve continuar lá");
}

#[test]
fn comando_desconhecido_ou_incompleto_mostra_o_uso_com_o_que_recebeu() {
    let bancada = Bancada::nova();
    let pasta = bancada.pasta.to_str().expect("utf-8");
    let chamadas: [&[&str]; 4] = [
        &[],
        &["apagar", pasta],
        &["instalar", pasta],
        &["desinstalar"],
    ];
    for argumentos in chamadas {
        let saida = rodar(bancada.apoio.path(), argumentos);
        assert!(!saida.status.success(), "aceitou {argumentos:?}");
        let mensagem = erro(&saida);
        assert!(mensagem.contains("uso:"), "{argumentos:?}: {mensagem}");
        let recebido = argumentos.first().copied().unwrap_or("recebi: nada");
        assert!(mensagem.contains(recebido), "{argumentos:?}: {mensagem}");
        assert!(bancada.arquivos().is_empty(), "{:?}", bancada.arquivos());
    }
}

#[cfg(target_os = "macos")]
#[test]
fn no_mac_de_verdade_gera_icns_valido_plist_valido_e_assina_o_pacote() {
    let apoio = tempfile::tempdir().expect("pasta temporária");
    let pasta = apoio.path().join("Applications");
    let executavel = std::env::current_exe().expect("executável do próprio teste");

    let saida = Command::new("sh")
        .arg(script())
        .args([
            "instalar",
            pasta.to_str().expect("utf-8"),
            executavel.to_str().expect("utf-8"),
        ])
        .env("TMPDIR", apoio.path())
        .output()
        .expect("rodar app/macos/instalar.sh no macOS");
    assert!(saida.status.success(), "{}", erro(&saida));

    let icns = fs::read(pasta.join(ICONE)).expect("ler icon.icns gerado");
    assert!(icns.starts_with(b"icns"), "cabeçalho icns inválido");
    assert!(
        icns.len() > 1000,
        "icon.icns pequeno demais: {}",
        icns.len()
    );

    let plutil = Command::new("plutil")
        .args(["-lint", pasta.join(PLIST).to_str().expect("utf-8")])
        .output()
        .expect("rodar plutil");
    assert!(
        plutil.status.success(),
        "Info.plist inválido: {}",
        String::from_utf8_lossy(&plutil.stdout)
    );

    let codesign = Command::new("codesign")
        .args([
            "--verify",
            "--deep",
            "--strict",
            pasta.join(PACOTE).to_str().expect("utf-8"),
        ])
        .output()
        .expect("rodar codesign --verify");
    assert!(
        codesign.status.success(),
        "assinatura inválida: {}",
        String::from_utf8_lossy(&codesign.stderr)
    );
}
