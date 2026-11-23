"""Teste funcional do app de desktop: abre o programa de verdade numa pasta
de livro temporária, deixa a interface digitar uma frase e confere o que
chegou ao disco e o que o navegador aplicou.

Uso: python3 -I teste_funcional.py <binário md-novel-app> [png da janela]

Duas passadas:
1. salvamento automático: digita e espera o app salvar sozinho;
2. fechar a janela logo depois de digitar: o texto tem de chegar ao disco
   antes de a janela fechar.

Precisa de uma sessão gráfica; não roda no CI. A captura só acontece no
Hyprland e pega apenas o retângulo da janela do app.
"""

import json
import os
import pathlib
import queue
import shutil
import subprocess
import sys
import tempfile
import threading
import time

LIMITE_SEGUNDOS = 60
FRASE = " Frase digitada pelo teste funcional."
# O salvamento automático espera 1,5 s. Fechar tem de salvar antes disso,
# senão quem salvou foi o temporizador e a passada 2 não provou nada.
PRAZO_PARA_FECHAR_SEGUNDOS = 1.3
RAIZ = pathlib.Path(__file__).resolve().parent.parent
ABERTURA = (
    "# Capítulo um\r\n\r\n"
    '::: {custom-style="Epígrafe"}\r\n'
    "A pele guarda o que a memória solta.\r\n"
    ":::\r\n\r\n"
)
PARAGRAFO = (
    "A casa ficava no alto do morro e de lá se via o rio inteiro quando a "
    "neblina deixava. Ela abria as janelas antes do café e esperava o barulho "
    "dos barcos que desciam carregados de sal e de farinha.\r\n\r\n"
)
SEGUNDO_CAPITULO = "# Capítulo dois\n\nOutro capítulo.\n".encode()
TEM_HYPRLAND = shutil.which("hyprctl") is not None and shutil.which("grim") is not None


def montar_livro(pasta: pathlib.Path) -> bytes:
    shutil.copy(RAIZ / "exemplos" / "estilo.yaml", pasta / "estilo.yaml")
    # CR LF de propósito: salvar tem de manter o formato do arquivo.
    primeiro = (ABERTURA + PARAGRAFO * 60).encode()
    (pasta / "01-abertura.md").write_bytes(primeiro)
    (pasta / "02-a-casa.md").write_bytes(SEGUNDO_CAPITULO)
    return primeiro


def hyprctl(*argumentos: str):
    return json.loads(subprocess.run(["hyprctl", *argumentos, "-j"], capture_output=True, text=True, check=True).stdout)


def janela_do_app(pid: int) -> dict | None:
    return next((cliente for cliente in hyprctl("clients") if cliente.get("pid") == pid), None)


def capturar(pid: int, destino: str) -> str:
    janela = janela_do_app(pid)
    visiveis = {monitor["activeWorkspace"]["id"] for monitor in hyprctl("monitors")}
    if janela is None or janela.get("workspace", {}).get("id") not in visiveis:
        return "captura: a janela não está numa área de trabalho visível; nada foi fotografado"
    (x, y), (largura, altura) = janela["at"], janela["size"]
    subprocess.run(["grim", "-g", f"{x},{y} {largura}x{altura}", destino], check=True)
    return f"captura: {largura}x{altura} em {destino}"


class App:
    """O binário rodando numa pasta de livro, com a saída lida numa thread:
    um app que não imprime nada não segura o teste além do limite."""

    def __init__(self, binario: str, pasta: pathlib.Path, fechar: bool = False):
        ambiente = {**os.environ, "MD_NOVEL_TESTE_DIGITAR": FRASE}
        if fechar:
            ambiente["MD_NOVEL_TESTE_FECHAR"] = "1"
        self.processo = subprocess.Popen(
            [binario, str(pasta)], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, env=ambiente
        )
        self.linhas: list[str] = []
        self._fila: queue.Queue = queue.Queue()
        threading.Thread(target=self._ler, daemon=True).start()

    def _ler(self) -> None:
        assert self.processo.stdout is not None
        for linha in self.processo.stdout:
            self._fila.put(linha)
        self._fila.put(None)

    def esperar_linha(self, trecho: str) -> str | None:
        """Lê até uma linha da interface com o trecho. `None` no limite ou se o app fechar."""
        prazo = time.monotonic() + LIMITE_SEGUNDOS
        while True:
            try:
                linha = self._fila.get(timeout=max(0.0, prazo - time.monotonic()))
            except queue.Empty:
                return None
            if linha is None:
                return None
            self.linhas.append(linha.rstrip())
            print("  " + linha.rstrip())
            if linha.startswith("[interface]") and trecho in linha:
                return linha

    def linha_com(self, prefixo: str) -> str:
        return next((linha for linha in self.linhas if linha.startswith(f"[interface] {prefixo}")), "")

    def encerrar(self) -> None:
        self.processo.terminate()
        try:
            self.processo.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.processo.kill()


def conferir_disco(pasta: pathlib.Path, original: bytes, falhas: list[str], passada: str) -> None:
    gravado = (pasta / "01-abertura.md").read_bytes()
    if gravado != original + FRASE.encode():
        falhas.append(f"{passada}: o capítulo no disco não é o original mais a frase digitada")
    if b"\n" in gravado.replace(b"\r\n", b""):
        falhas.append(f"{passada}: o CR LF do arquivo não foi mantido")
    if (pasta / "02-a-casa.md").read_bytes() != SEGUNDO_CAPITULO:
        falhas.append(f"{passada}: o capítulo 2 mudou")
    sobras = sorted(entrada.name for entrada in pasta.iterdir())
    if sobras != ["01-abertura.md", "02-a-casa.md", "estilo.yaml"]:
        falhas.append(f"{passada}: arquivos na pasta do livro: {sobras}")


def passada_do_salvamento_automatico(binario: str, destino: str | None, falhas: list[str]) -> None:
    print("passada 1: salvamento automático")
    with tempfile.TemporaryDirectory(prefix="md-novel-teste-") as temporaria:
        pasta = pathlib.Path(temporaria)
        original = montar_livro(pasta)
        app = App(binario, pasta)
        try:
            if app.esperar_linha("salvo:") is None:
                falhas.append("passada 1: o app não chegou a salvar")
            elif destino and TEM_HYPRLAND:
                time.sleep(1)
                print("  " + capturar(app.processo.pid, destino))
        finally:
            app.encerrar()

        def exigir(condicao: bool, falha: str) -> None:
            if not condicao:
                falhas.append(f"passada 1: {falha}")

        exigir("2 capítulos, css=true" in app.linha_com("livro:"), f"livro: {app.linha_com('livro:')!r}")
        exigir("01-abertura.md" in app.linha_com("capítulo:"), f"capítulo: {app.linha_com('capítulo:')!r}")
        corpo, nomeado = app.linha_com("linha do corpo:"), app.linha_com("linha de estilo nomeado:")
        exigir("align=justify" in corpo and "indent=47.2" in corpo, f"corpo: {corpo!r}")
        exigir(
            "align=right" in nomeado and "style=italic" in nomeado and "indent=0px" in nomeado,
            f"nomeado: {nomeado!r}",
        )
        rolagem = app.linha_com("rolagem:").removeprefix("[interface] rolagem: área ").replace("px", "").split(" de ")
        exigir(len(rolagem) == 2 and int(rolagem[0]) < int(rolagem[1]), f"o editor não rola por dentro: {rolagem}")
        conferir_disco(pasta, original, falhas, "passada 1")


def passada_de_fechar_a_janela(binario: str, falhas: list[str]) -> None:
    print("passada 2: fechar a janela logo depois de digitar")
    with tempfile.TemporaryDirectory(prefix="md-novel-teste-") as temporaria:
        pasta = pathlib.Path(temporaria)
        original = montar_livro(pasta)
        # O app pede o próprio fechamento logo depois de digitar, pelo mesmo
        # evento que o gerenciador de janelas dispara.
        app = App(binario, pasta, fechar=True)
        try:
            if app.esperar_linha("digitado pelo roteiro de teste") is None:
                falhas.append("passada 2: a interface não digitou")
                return
            digitado = time.monotonic()
            try:
                app.processo.wait(timeout=10)
            except subprocess.TimeoutExpired:
                falhas.append("passada 2: a janela não fechou depois de salvar")
                return
            demora = time.monotonic() - digitado
            print(f"  fechou {demora:.2f}s depois de digitar")
            if demora >= PRAZO_PARA_FECHAR_SEGUNDOS:
                falhas.append(f"passada 2: fechou em {demora:.2f}s; o salvamento automático pode ter salvo antes")
        finally:
            app.encerrar()
        conferir_disco(pasta, original, falhas, "passada 2")


def principal() -> int:
    binario = sys.argv[1]
    destino = sys.argv[2] if len(sys.argv) > 2 else None
    falhas: list[str] = []
    passada_do_salvamento_automatico(binario, destino, falhas)
    passada_de_fechar_a_janela(binario, falhas)
    for falha in falhas:
        print(f"FALHOU: {falha}")
    print("teste funcional:", "reprovado" if falhas else "aprovado")
    return 1 if falhas else 0


if __name__ == "__main__":
    sys.exit(principal())
