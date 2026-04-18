<p align="center">
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" />
  <img src="https://img.shields.io/badge/IL2CPP-v16--v39-blueviolet?style=for-the-badge" />
  <img src="https://img.shields.io/badge/License-MIT-green?style=for-the-badge" />
</p>

<h1 align="center">🛡️ Rodroid Il2CppDumper</h1>

<p align="center">
  <b>A blazing-fast, cross-platform IL2CPP binary dumper written in Rust.</b><br/>
  Full rewrite from the original C# <a href="https://github.com/Perfare/Il2CppDumper">Il2CppDumper</a> with significant performance improvements, modern CLI UI, and advanced features.
</p>

<p align="center">
  <a href="https://t.me/+WmudnO0-xoNhMDQ8">📢 Telegram Channel</a> &nbsp;·&nbsp;
  <a href="https://t.me/+QylrYL1GNsJiYjc0">💬 Telegram Group</a> &nbsp;·&nbsp;
  <b>Dev:</b> <a href="https://t.me/rodroidmods"><code>@rodroidmods</code></a>
</p>

---

## 🏆 Feature Comparison — Rust vs C# vs Python

| Feature | [Python](https://github.com/springmusk026/Il2CppDumper-Python) | [C#](https://github.com/Perfare/Il2CppDumper) | **Rust (This)** |
|---------|:---:|:---:|:---:|
| **dump.cs generation** | ⚠️ Basic (no properties) | ✅ Full | ✅ Full |
| **DummyDLL generation** | ❌ No | ✅ Yes | ✅ Yes (parallel) |
| **il2cpp.h struct gen** | ⚠️ Basic | ✅ Full | ✅ Full |
| **script.json** | ✅ Yes | ✅ Yes | ✅ Yes |
| **stringliteral.json** | ✅ Yes | ✅ Yes | ✅ Yes |
| **Split Dump Per Type (DiffableCS)** | ❌ No | ❌ No | ✅ **Yes (parallel)** |
| **Variable-width indices (v39/Unity 6)** | ❌ No | ❌ No | ✅ **Yes** |
| **Auto-XOR Metadata Decryption** | ❌ No | ❌ No | ✅ **Yes** |
| **Latest Unity Formats (v104, v106)** | ❌ No | ❌ No | ✅ **Yes** |
| **Assembly name in dump.cs** | ❌ No | ❌ No | ✅ **Yes** |
| **Unity version detection** | ❌ No | ❌ No | ✅ **Auto-detect** |
| **Inline Disassembly (ARM64/ARM32/x86/x64)** | ❌ No | ❌ No | ✅ **Yes** |
| **Control Flow Graph (CFG) Analysis** | ❌ No | ❌ No | ✅ **Yes** |
| **Metadata Annotations (strings, types, vtable)** | ❌ No | ❌ No | ✅ **Yes** |
| **Semantic Variable Tracking** | ❌ No | ❌ No | ✅ **Yes** |
| **Fat Mach-O (Universal)** | ❌ No | ✅ Yes | ✅ Yes |
| **WASM (WebGL)** | ✅ Yes | ✅ Yes | ✅ Yes |
| **Dump file support** | ❌ No | ✅ Yes | ✅ Yes (+ ELF reload) |
| **v27+ ImageBase fix** | ❌ No | ✅ Yes | ✅ Yes |
| **Parallel I/O (`rayon`)** | ❌ No | ❌ No | ✅ **Yes** |
| **Auto-numbered output dirs** | ❌ No | ❌ No | ✅ **Dump0/, Dump1/...** |
| **Modern CLI UI (spinners, colors, prompts)** | ❌ No | ❌ No | ✅ **Yes** |
| **Cross-platform binary** | ⚠️ Needs Python | ⚠️ Needs .NET | ✅ **Standalone** |
| **GUI** | ❌ No | ✅ WinForms | ✅ **Jetpack Compose (Android)** |
| **Embeddable as library** | ❌ No | ❌ No | ✅ **Rust crate / JNI** |

### ⚡ Performance

| Phase | Python | C# | **Rust** |
|-------|:---:|:---:|:---:|
| Metadata loading | ~3.7s | ~2s | **~0.5s** |
| Binary loading | ~5.2s | ~3s | **~0.8s** |
| Search & Init | ~5.4s | ~2s | **~0.3s** |
| dump.cs | ~14s | ~5s | **~2s** |
| Struct generation | ~6.4s | ~5s | **~3.5s** |
| DummyDLL | ❌ N/A | ~3s | **~1.5s** |
| **Total** | **~35s** | **~20s** | **~5–8s** |

> **4× faster than Python, 2.4× faster than C#** — on the same binary.

---

## ✨ Features

### Core Dumping
- **dump.cs** — Full C# class/method/field/property decompilation with RVA/VA/Offset and assembly names
- **Inline Disassembly** — Optional per-method native assembly embedded directly in dump.cs
- **DiffableCs** — Splits classes into individual `.cs` files by namespace, parallelized with `rayon`
- **script.json** — Method addresses/signatures for IDA/Ghidra scripting
- **il2cpp.h** — C struct definitions for native analysis
- **stringliteral.json** — All string literal values and indices
- **DummyDLL** — Reconstructed .NET assemblies for dnSpy/ILSpy (parallelized)

### Disassembly Engine
- **Multi-Architecture** — ARM64 (`yaxpeax-arm`), ARM32, x86/x64 (`iced-x86`)
- **Control Flow Graph (CFG)** — Basic block detection, branch targets, loop back-edges, `if (condition)` reconstruction
- **Metadata Annotations** — String literals, type info, method/field references, vtable resolution via `ADRP+LDR` patterns
- **Semantic Variable Tracking** — Maps registers to parameter names (`X0` → `this`, `X1` → `arg0`)
- **Configurable** — Toggle hex bytes, field names, annotations, CFG independently

### Supported Platforms

| Platform | Format | Status |
|----------|--------|:---:|
| Android | ELF32 / ELF64 | ✅ |
| iOS / macOS | Mach-O / Fat Mach-O | ✅ |
| Windows | PE32 / PE64 | ✅ |
| Nintendo Switch | NSO | ✅ |
| WebGL | WASM | ✅ |

### IL2CPP Versions
- **v16 – v39** (Unity 5.3 → Unity 6)
- Variable-width indices for v39/Unity 6
- Latest undocumented formats: `v104`, `v106`
- Auto XOR metadata decryption (1-byte, 4-byte, 8-byte, rolling, position-dependent, header-only)
- Manual version override via config

### Search Strategies
- **SectionHelper** — Format-aware section scanning
- **Symbol Search** — ELF/Mach-O symbol table lookup
- **ARM32 Search** — Dedicated binary pattern matching
- **\_\_mod\_init\_func** — Mach-O initializer analysis
- **Manual mode** — Enter addresses as fallback

---

## 📦 Installation

### From Source
```bash
git clone https://github.com/rodroidmods/il2cpp-dumper-rs.git
cd il2cpp-dumper-rs/il2cpp_dumper
cargo build --release
```

The binary will be at `target/release/il2cpp_dumper` (`.exe` on Windows).

### Cross-Compilation
```bash
# Linux
cargo build --release --target x86_64-unknown-linux-gnu

# macOS
cargo build --release --target aarch64-apple-darwin

# Android (via cross)
cross build --release --target aarch64-linux-android
```

---

## 🔧 Usage

```bash
il2cpp_dumper <il2cpp-binary> <global-metadata.dat> [output-dir]
```

### Examples
```bash
il2cpp_dumper libil2cpp.so global-metadata.dat
il2cpp_dumper GameAssembly.dll global-metadata.dat ./output
il2cpp_dumper UnityFramework global-metadata.dat
```

### Example Output
```
  Rodroid Il2CppDumper

    ╦╦  ╔═╗╔═╗╔═╗  ╔╦╗╦ ╦╔╦╗╔═╗╔═╗╦═╗
    ║║  ╠═╝║  ╠═╝   ║║║ ║║║║╠═╝║╣ ╠╦╝
    ╩╩═╝╚  ╚═╝╩    ═╩╝╚═╝╩ ╩╩  ╚═╝╩╚═
    Version v0.3.0
  ─────────────────────────────────────

  📂 Output .\Dump0

  ─── Binary Analysis ───────────────────

  ✓ Binary loaded: libil2cpp.so (52.43 MB)
  Unity Version: 2022.3.62f2
  ✓ Metadata loaded: global-metadata.dat (10.88 MB)
  Metadata Version: 31
  Type Definitions: 13815
  Method Definitions: 93772

  ─── Format Detection ──────────────────

  🔍 Detected ELF64 format
  IL2CPP Version: 31
  CodeRegistration: 0x44e5ff8
  MetadataRegistration: 0x465a328

  ─── Output Generation ─────────────────

  ✓ dump.cs generated
  ✓ script.json, il2cpp.h, stringliteral.json generated
  ✓ Dummy DLL files generated

  ═══════════════════════════════════════
  ✨ All tasks completed successfully!
  ═══════════════════════════════════════

  📂 Output Directory: .\Dump0
  📦 Generated Files: dump.cs, script.json, il2cpp.h, stringliteral.json, DummyDll/*.dll
  🚀 Elapsed: 8.35s
  ───────────────────────────────────────
```

---

## ⚙️ Configuration

Create a `config.json` in the working directory (or use `--config`):

```json
{
  "ForceIl2CppVersion": false,
  "ForceVersion": 29.0,
  "ForceDump": false,
  "NoRedirectedPointer": false,
  "GenerateStruct": true,
  "GenerateDummyDll": true,
  "DummyDllAddToken": true,
  "dumpDisassembly": false,
  "dumpDisassemblyHexBytes": true,
  "dumpDisassemblyFieldNames": true,
  "dumpDisassemblyAnnotations": true,
  "dumpDisassemblyCfg": true,
  "maxDisassemblyInstructions": 512
}
```

---

## 🏗️ Architecture

```
il2cpp_dumper/src/
├── main.rs                           # CLI, format detection, orchestration
├── config.rs                         # Configuration handling
├── formats/                          # Binary format parsers
│   ├── elf.rs                        # ELF32/64 (Android, Linux)
│   ├── pe.rs                         # PE32/64 (Windows)
│   ├── macho.rs                      # Mach-O + Fat Mach-O (iOS, macOS)
│   ├── nso.rs                        # NSO (Nintendo Switch)
│   └── wasm.rs                       # WebAssembly (WebGL)
├── il2cpp/                           # IL2CPP structures and metadata
│   ├── base.rs                       # Il2Cpp main struct
│   ├── metadata.rs                   # Metadata parser
│   └── structures.rs                 # IL2CPP type definitions
├── search/                           # Registration search algorithms
│   └── section_helper.rs
├── executor/                         # IL2CPP type resolution engine
├── disassembler/                     # Multi-arch disassembly engine
│   ├── mod.rs                        # CFG analysis, annotations, formatting
│   ├── arm.rs                        # ARM64/ARM32 decoder (yaxpeax)
│   └── x86.rs                        # x86/x64 decoder (iced-x86)
└── output/                           # Output generators
    ├── decompiler.rs                 # dump.cs + inline disassembly
    ├── struct_generator.rs           # script.json, il2cpp.h
    └── dummy_assembly_generator.rs   # DummyDLL (parallel)
```

---

## 📜 License

MIT

---

## 🙏 Credits

- [Perfare/Il2CppDumper](https://github.com/Perfare/Il2CppDumper) — Original C# implementation
- [SamboyCoding/Cpp2IL](https://github.com/SamboyCoding/Cpp2IL) — Advanced C# IL2CPP analysis tool
- [springmusk026/Il2CppDumper-Python](https://github.com/springmusk026/Il2CppDumper-Python) — Python port
- [dotnetdll](https://crates.io/crates/dotnetdll) — .NET DLL generation crate
- [rayon](https://crates.io/crates/rayon) — Parallel processing
- [console-rs](https://github.com/console-rs) — Terminal styling ecosystem (`console`, `indicatif`, `dialoguer`)

---

## 📬 Community

| | Link |
|---|---|
| 📢 **Telegram Channel** | [Join Channel](https://t.me/+WmudnO0-xoNhMDQ8) |
| 💬 **Telegram Group** | [Join Group](https://t.me/+QylrYL1GNsJiYjc0) |
| 👤 **Developer** | [`@rodroidmods`](https://t.me/rodroidmods) |

---

> **⚠️ Disclaimer**: This tool is for educational and research purposes only. Respect game developers' rights and terms of service.
