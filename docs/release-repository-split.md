# Release-Komponenten in eigene Repositories aufteilen

## Ziel

Aus dem aktuellen Monorepository werden vier eigenständige Repositories erzeugt. Jedes Zielrepository enthält ausschließlich den Quellcode, die Tests, die Build-Dateien und die Dokumentation der Komponente, deren Name außen draufsteht.

Die Grundlage sind die vier bereits in `releases/README.md` definierten Release-Artefakte:

```text
bitshit-cli-<version>-<platform>.<archive>
bitshit-engine-<version>-<platform>.<archive>
bitshit-kernel-cpu-<version>-<platform>.<archive>
bitshit-driver-cpu-<version>-<platform>.<archive>
```

## Verbindliche Ziel-Repositories

### `1bitshit-cli`

Enthält nur:

- CLI-Anwendung
- Terminal-Dashboard und Menüs
- Benutzerkonfiguration
- Modellwahl und lokale Engine-Ansteuerung
- CLI-spezifische Tests
- CLI-Build und Release-Workflow

Wird entfernt:

- Inference-Engine-Implementierung
- CPU-Kernel
- CPU-Driver
- Modell-Registry, soweit sie nicht als reine Client-Metadaten benötigt wird
- Server- und Backend-Code

Rebranding:

- Paketname: `1bitshit-cli`
- Binary: `bitshit`
- sichtbare Produktbezeichnung: `1BitShit CLI`
- keine verbleibenden `cluaiz`-Produktnamen in Banner, README, Paketmetadaten oder Release-Namen

Release-Artefakt:

```text
bitshit-cli-<version>-<platform>.<archive>
```

---

### `1bitshit-engine`

Enthält nur:

- Inference-Engine
- Router und Runtime-Ausführung
- Modell-Lifecycle
- Engine-API
- Engine-spezifische Tests
- Engine-Build und Release-Workflow

Wird entfernt:

- Terminal-CLI und Dashboard
- CPU-Kernel-Quellbaum, sofern er als separates Modul veröffentlicht wird
- CPU-Driver-Quellbaum
- nicht benötigte UI-Assets
- vollständige Modellgewichte

Rebranding:

- Paketname: `1bitshit-engine`
- Bibliotheks-/Servicename: `bitshit-engine`
- sichtbare Produktbezeichnung: `1BitShit Engine`
- alte `cluaiz`-Produktbezeichnungen werden aus öffentlichen Metadaten entfernt

Release-Artefakt:

```text
bitshit-engine-<version>-<platform>.<archive>
```

---

### `1bitshit-kernel-cpu`

Enthält nur:

- CPU-Kernel
- CPU-spezifische Low-Level-Inferenzpfade
- SIMD-/Quantisierungsimplementierungen
- Kernel-Benchmarks
- Kernel-Tests
- Kernel-Build und Release-Workflow

Wird entfernt:

- CLI
- Dashboard
- Engine-Orchestrierung
- Modell-Registry
- Netzwerk- und API-Code
- CPU-Driver, sofern dieser getrennt gebaut wird

Rebranding:

- Paketname: `1bitshit-kernel-cpu`
- Bibliotheksname: `bitshit-kernel-cpu`
- sichtbare Produktbezeichnung: `1BitShit CPU Kernel`

Release-Artefakt:

```text
bitshit-kernel-cpu-<version>-<platform>.<archive>
```

---

### `1bitshit-driver-cpu`

Enthält nur:

- CPU-Backend-Driver
- Engine-zu-Kernel-Adapter
- Hardware-Erkennung, soweit sie für den Driver erforderlich ist
- Driver-spezifische Tests
- Driver-Build und Release-Workflow

Wird entfernt:

- CLI
- Dashboard
- vollständige Engine
- eigentliche Kernel-Implementierung
- Modell-Registry
- nicht benötigte Assets und Dokumentation

Rebranding:

- Paketname: `1bitshit-driver-cpu`
- Bibliotheksname: `bitshit-driver-cpu`
- sichtbare Produktbezeichnung: `1BitShit CPU Driver`

Release-Artefakt:

```text
bitshit-driver-cpu-<version>-<platform>.<archive>
```

## Reihenfolge der Aufteilung

1. Komponentengrenzen im Monorepository festlegen.
2. Für jede Komponente alle fremden Verzeichnisse und Abhängigkeiten entfernen.
3. Cargo-Workspaces, Paketnamen, Imports und Buildskripte korrigieren.
4. Öffentliche Produktnamen vollständig auf `1BitShit` umstellen.
5. Pro Komponente eigenständige Tests und Builds ausführen.
6. Erst danach das jeweilige Zielrepository erstellen und den bereinigten Stand pushen.
7. Tags und GitHub-Releases aus dem jeweiligen Komponentenrepository erzeugen.

## Abhängigkeitsregel

Die Ziel-Repositories dürfen sich nur über klar versionierte Bibliotheks- oder Paketabhängigkeiten verbinden. Kein Repository darf zur Laufzeit Dateien direkt aus einem Schwesterrepository erwarten.

Beispiel:

```text
1bitshit-cli -> 1bitshit-engine
1bitshit-engine -> 1bitshit-driver-cpu
1bitshit-driver-cpu -> 1bitshit-kernel-cpu
```

## Release-Regel

Jedes Repository veröffentlicht nur sein eigenes Artefakt. Archive dürfen keine vollständigen Kopien des ursprünglichen Monorepositories enthalten.

Jedes Archiv enthält mindestens:

- Komponentenversion
- Zielplattform bzw. Target Triple
- Quell-Commit
- SHA-256-Prüfsumme
- Lizenzhinweise
- kurze Installations- oder Einbindungsanleitung

## Aktueller Status

- `1BitShit CPU v0.2.0-dev` wurde in das Hauptrepository gemergt.
- Die Zielstruktur ist mit diesem Dokument verbindlich festgelegt.
- Die eigentliche physische Extraktion und Erstellung der vier neuen Repositories erfolgt erst nach vollständiger Bereinigung und erfolgreichem Einzelbuild jeder Komponente.
