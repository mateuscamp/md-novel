"""Roda o binário do spike, espera as medidas automáticas e captura a janela.

Uso: python3 -I medir.py <binário> <png de saída>

A captura pega só o retângulo da janela do spike, e só se ela estiver na área
de trabalho visível, para não fotografar outra coisa da tela.
"""

import json
import queue
import subprocess
import sys
import threading
import time

LIMITE_SEGUNDOS = 90
TITULO = "md-novel — spike da janela"


def hyprctl(*argumentos: str):
    return json.loads(subprocess.run(["hyprctl", *argumentos, "-j"], capture_output=True, text=True, check=True).stdout)


def janela_do_spike() -> dict | None:
    # Com mais de um monitor, cada um mostra a sua área de trabalho.
    monitores = hyprctl("monitors")
    visiveis = {monitor["activeWorkspace"]["id"] for monitor in monitores}
    for monitor in monitores:
        print(f"monitor {monitor['name']}: {monitor['width']}x{monitor['height']} a {monitor['refreshRate']:.1f} Hz")
    for cliente in hyprctl("clients"):
        if cliente.get("title") == TITULO and cliente.get("workspace", {}).get("id") in visiveis:
            return cliente
    return None


def capturar(destino: str) -> str:
    janela = janela_do_spike()
    if janela is None:
        return "captura: a janela não está na área de trabalho visível; nada foi fotografado"
    (x, y), (largura, altura) = janela["at"], janela["size"]
    subprocess.run(["grim", "-g", f"{x},{y} {largura}x{altura}", destino], check=True)
    return f"captura: {largura}x{altura} em {destino} (xwayland={janela.get('xwayland')})"


def ler_linhas(saida, fila: queue.Queue) -> None:
    for linha in saida:
        fila.put(linha)
    fila.put(None)


def principal() -> int:
    binario, destino = sys.argv[1], sys.argv[2]
    processo = subprocess.Popen([binario], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    # A leitura fica numa thread: um app que não imprime nada não pode
    # segurar o script além do limite.
    fila: queue.Queue = queue.Queue()
    threading.Thread(target=ler_linhas, args=(processo.stdout, fila), daemon=True).start()
    prazo = time.monotonic() + LIMITE_SEGUNDOS
    pronto = False
    try:
        while not pronto:
            try:
                linha = fila.get(timeout=max(0.0, prazo - time.monotonic()))
            except queue.Empty:
                print(f"limite de {LIMITE_SEGUNDOS}s sem a linha 'pronto:'; o app não terminou de medir")
                break
            if linha is None:
                print("o app fechou antes de terminar de medir")
                break
            print(linha.rstrip())
            pronto = "pronto:" in linha
        if pronto:
            time.sleep(1)
            print(capturar(destino))
    finally:
        processo.terminate()
        try:
            processo.wait(timeout=10)
        except subprocess.TimeoutExpired:
            processo.kill()
    return 0 if pronto else 1


if __name__ == "__main__":
    sys.exit(principal())
