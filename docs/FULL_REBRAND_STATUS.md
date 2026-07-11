# 1BitShit CPU – vollständiger Rebrand-Vertrag

## Ziel

Das Projekt wird vollständig als **1BitShit CPU** ausgeliefert. Der Rebrand darf
keine Engine, API, Modellart, FFI-Funktion, UI-Funktion oder bestehende
Installation zerstören.

## Primäre Namen

| Bereich | Primärer Name |
|---|---|
| Produkt | `1BitShit CPU` |
| CLI | `bitshit` |
| Runtime-Verzeichnis | `~/.1bitshit` |
| Modelle | `~/.1bitshit/models` bzw. `BITSHIT_MODELS_DIR` |
| Shared-Crate | `bitshit-shared` |
| API-Crate | `bitshit-api` |
| Llama-Crate | `bitshit-llama` |
| ONNX-Crate | `bitshit-onnx` |
| Builder-Crate | `bitshit-builder` |
| Engine-Bibliothek | `bitshit-engine` |
| Llama-Bibliothek | `bitshit-llama` |
| ONNX-Bibliothek | `bitshit-onnx` |
| Native ABI | `bitshit_kernel_*` |
| Umgebungsvariablen | `BITSHIT_HOME`, `BITSHIT_ROOT`, `BITSHIT_MODELS_DIR` |

## Erhaltene Funktionen

- Llama/GGUF- und BitNet-Inferenz
- ONNX-Text-, Embedding-, Vision- und Audio-Infrastruktur
- Hugging-Face-Auswahl für GGUF und ONNX
- gesplittete GGUF-Dateien
- externe ONNX-Datendateien
- Tokenizer-, Vokabular-, Preprocessor- und Konfigurationsdateien
- sichtbarer gemeinsamer Modellordner
- Model Hub, Pull, Run, List und Remove
- Chat-API und Chat-Entitäten
- API-Daemon
- native Streaming-FFI
- KV-Cache laden und speichern
- Trigger-Interception und Abbruchsteuerung
- Hardware-Audit und Booster
- Skills, Plugins, MCP und Ingestion
- Dashboard und native Menüs
- installierte und lokale Development-Runtime

## Format-Autorität

Die Dateiendung bestimmt den Loader:

- `.gguf` und unterstützte `.bin`-Gewichte verwenden Llama/GGUF.
- `.onnx` verwendet ONNX.
- Ein GGUF-Modell darf niemals als ONNX geöffnet werden.
- Ein ONNX-Modell darf niemals als GGUF geöffnet werden.
- ONNX-Embedding-, Vision- und Audio-Modelle werden nicht als Chatmodell erzwungen.

## CPU-Standard

`bitshit-onnx` baut standardmäßig mit dem Feature `cpu`. CUDA wird nur durch
`--features cuda` zugeschaltet. Eine CPU-Binary meldet keine CUDA-Aktivierung,
wenn CUDA nicht einkompiliert wurde.

## Kompatibilitätsregel

Alte Namen dürfen nur noch in gekapselten Adaptern vorkommen:

- alte native Symbole `cluaiz_kernel_*` leiten auf `bitshit_kernel_*` weiter;
- alte Rust-Abhängigkeitsnamen können als lokale Cargo-Aliase auf neue Pakete
  zeigen;
- `~/.cluaiz` darf einmalig **gelesen und kopiert**, aber niemals gestartet oder
  automatisch gelöscht werden;
- alte nicht mehr kompilierte Referenzdateien bleiben während der Migration
  erhalten, bis alle Downstream-Nutzer auf die neuen Schnittstellen gewechselt
  sind.

Kompatibilitätsnamen dürfen nicht mehr als Menütext, Banner, Updatequelle,
Installationsziel, Modellpfad oder primärer Bibliotheksname erscheinen.

## Build-Abnahme

Der Rebrand ist nur veröffentlichbar, wenn folgendes Skript erfolgreich endet:

```bash
bash tools/verify-rebrand.sh
```

Es prüft und baut:

1. `bitshit-shared`
2. `bitshit-onnx` im CPU-Standard
3. `bitshit-llama`
4. `engines`
5. `bitshit-api`
6. die `bitshit`-CLI
7. den vollständigen CPU-Workspace
8. alle Release-Artefakte
9. erhaltene Feature-Bäume
10. aktive sichtbare Legacy-Bezeichnungen

## Installation und Migration

Der vollständige lokale Ablauf liegt in:

```bash
tools/apply-full-rebrand.sh
```

Das Skript sichert lokale Änderungen, lädt den Rebrand-Branch, führt die
vollständige Abnahme aus, installiert die neue Runtime, übernimmt bestehende
Modelle kopierend und startet anschließend `bitshit`.
