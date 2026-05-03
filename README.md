<p align="center">
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" />
  <img src="https://img.shields.io/badge/IL2CPP-v16--v39-blueviolet?style=for-the-badge" />
  <img src="https://img.shields.io/badge/License-MIT-green?style=for-the-badge" />
</p>

<h1 align="center">🛡️ Rodroid Il2CppDumper V5</h1>

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
| **Forward Constant Propagation** | ❌ No | ❌ No | ✅ **Yes** |
| **Backward Slicing (Vtable Resolution)** | ❌ No | ⚠️ Partial (Cpp2IL) | ✅ **Yes** |
| **Init-Check Folding** | ❌ No | ❌ No | ✅ **Yes** |
| **String Literal Indirect Resolution (`il2cpp_string_new_wrapper`)** | ❌ No | ❌ No | ✅ **Yes** |
| **Generic Instantiation Tracking** | ❌ No | ❌ No | ✅ **Yes** |
| **Switch Table Reconstruction** | ❌ No | ❌ No | ✅ **Yes** |
| **Boxing/Unboxing Pattern Detection** | ❌ No | ❌ No | ✅ **Yes** |
| **Static Field Access Annotation** | ❌ No | ❌ No | ✅ **Yes** |
| **CODM (Call of Duty Mobile) Support** | ❌ No | ❌ No | ✅ **Yes (Android + iOS, 32/64-bit)** |
| **Fat Mach-O (Universal)** | ❌ No | ✅ Yes | ✅ Yes |
| **WASM (WebGL)** | ✅ Yes | ✅ Yes | ✅ Yes |
| **Dump file support** | ❌ No | ✅ Yes | ✅ Yes (+ ELF reload) |
| **v27+ ImageBase fix** | ❌ No | ✅ Yes | ✅ Yes |
| **Parallel I/O (`rayon`)** | ❌ No | ❌ No | ✅ **Yes** |
| **Auto-numbered output dirs** | ❌ No | ❌ No | ✅ **Dump0/, Dump1/...** |
| **Modern CLI UI (spinners, colors, prompts)** | ❌ No | ❌ No | ✅ **Yes** |
| **Cross-platform binary** | ⚠️ Needs Python | ⚠️ Needs .NET | ✅ **Standalone** |
| **C++ Scaffold (il2cpp-functions.h)** | ❌ No | ❌ No | ✅ **Yes** |
| **C++ Name Mangling (Itanium ABI)** | ❌ No | ❌ No | ✅ **Yes** |
| **Unity Header Auto-Detection** | ❌ No | ✅ Yes | ✅ **Yes (version-matched)** |
| **cpp_project/ Scaffolding** | ❌ No | ✅ Yes | ✅ **Yes** |
| **Topological Sort (type ordering)** | ❌ No | ❌ No | ✅ **Yes** |
| **Type Group Classification** | ❌ No | ❌ No | ✅ **Yes** |
| **Enhanced IDA Metadata** | ❌ No | ❌ No | ✅ **Yes** |
| **ELF Section Header Symbol Fallback** | ❌ No | ✅ Yes | ✅ **Yes** |
| **GUI** | ❌ No | ✅ WinForms | ✅ **Jetpack Compose + Tauri** |
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
- **il2cpp.h** — C struct definitions for native analysis with topological type ordering
- **stringliteral.json** — All string literal values and indices
- **DummyDLL** — Reconstructed .NET assemblies for dnSpy/ILSpy (parallelized)

### C++ Headers & Scaffolding
- **il2cpp-functions.h** — C++ scaffold with function pointer typedefs for hooking
- **Itanium ABI Name Mangling** — Correct C++ mangled names for all IL2CPP types
- **Unity Header Auto-Detection** — Version-matched `il2cpp-types.h` and `il2cpp-api.h` from embedded header database
- **cpp_project/** — Ready-to-compile C++ project scaffold with includes and CMake structure
- **Topological Sort** — Types emitted in dependency order; circular dependencies detected with fallback
- **Type Group Classification** — Types categorized into forward declarations, method types, generic types, usage types
- **Compiler Layout** — GCC (`__attribute__`) or MSVC (`__declspec`) layout attributes
- **Enhanced IDA Metadata** — Extra type info annotations for IDA Pro scripts

### Disassembly Engine
- **Multi-Architecture** — ARM64 (`yaxpeax-arm`), ARM32, x86/x64 (`iced-x86`)
- **Control Flow Graph (CFG)** — Basic block detection, branch targets, loop back-edges, `if (condition)` reconstruction
- **Metadata Annotations** — String literals, type info, method/field references, vtable resolution via `ADRP+LDR` patterns
- **Semantic Variable Tracking** — Maps registers to parameter names (`X0` → `this`, `X1` → `arg0`)
- **Forward Constant Propagation** — Tracks register values (MOVZ/MOVK/ADD/SUB/ORR/ADRP) across instructions to resolve register+register memory accesses like `LDR X0, [X22, X21, LSL #3]` into field names (e.g. `// this.<>2__current`). First IL2CPP tool to annotate indexed field accesses.
- **Backward Slicing for Vtable Resolution** — On `BLR Xn`, walks backward through `LDR X8, [X0]` (klass) → `LDR X9, [X8, #N]` (vtable slot) chains to resolve indirect calls into `// virtual call: TypeName.MethodName` instead of opaque `sub_XXXXXX`.
- **Initialization-Check Folding** — Detects and collapses `il2cpp_codegen_initialize_method`, `Il2CppCodeGenWriteBarrier`, and `TBZ/TBNZ`-on-bit-0 prologue patterns into a single `// [init check]` annotation, drastically reducing method-body noise.
- **Indirect String Literal Resolution** — Tracks the literal index in `W0/W1` at `il2cpp_string_new_wrapper` call sites and resolves through `metadata_string_literals` to annotate the actual string content (`// "Hello, world"`).
- **Generic Instantiation Tracking** — Resolves calls into `MethodInfo*` slots via `method_definition_method_specs` to annotate the concrete specialization (e.g. `// → List<int>.Add(this, item)`).
- **Switch Table Reconstruction** — Detects ARM64 jump-table prologues (`ADRP+ADD+LDR Xn, [Xn, Xidx, lsl #2]+BR Xn`) using reg+reg propagation and emits `switch (var)` blocks in the CFG.
- **Boxing / Unboxing Detection** — Annotates `il2cpp_codegen_box` / `il2cpp_unbox` call sites with the resolved boxed type from the first-arg type pointer.
- **Static Field Access Annotation** — Resolves the `ADRP+ADD → LDR X8, [Xklass, #static_fields_offset] → LDR Wd, [X8, #field_offset]` pattern into `// SomeClass.staticField` using the existing klass identification map.
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

### CODM (Call of Duty Mobile)
- Custom v23 metadata layout with two-slot `type_definitions_count` fingerprint anchor
- Android packed relocations (`DT_ANDROID_RELA` / `DT_ANDROID_REL`, APS2 + SLEB128) for 32-bit and 64-bit ELF
- iOS chained fixups (`LC_DYLD_CHAINED_FIXUPS`) and legacy rebase opcodes (`LC_DYLD_INFO_ONLY`) for 32-bit and 64-bit Mach-O
- Pointer formats: `DYLD_CHAINED_PTR_64`, `_64_OFFSET`, ARM64E variants
- Toggle via `--codm` flag or `Codm: true` in config — additive code path, leaves standard Unity games untouched

### Search Strategies
- **SectionHelper** — Format-aware section scanning
- **Symbol Search** — ELF/Mach-O symbol table lookup
- **ARM32 Search** — Dedicated binary pattern matching
- **\_\_mod\_init\_func** — Mach-O initializer analysis
- **Manual mode** — Enter addresses as fallback

---

## 📦 Installation

### From crates.io
```bash
cargo install il2cpp_dumper
```

This installs the latest release globally. Run `il2cpp_dumper` from anywhere.

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
    Version v0.4.1
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
  "maxDisassemblyInstructions": 512,
  "generateCppScaffold": true,
  "mangleNames": true,
  "enhancedIdaMetadata": true,
  "generateUnityHeaders": true,
  "compilerLayout": "GCC",
  "useTopologicalSort": true
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
    ├── struct_generator.rs           # script.json, il2cpp.h, type classification
    ├── dummy_assembly_generator.rs   # DummyDLL (parallel)
    ├── cpp_scaffolding.rs            # il2cpp-functions.h generation
    ├── cpp_ast.rs                    # C++ AST emission with group annotations
    ├── cpp_type_model.rs             # C++ type model from IL2CPP types
    ├── cpp_type_dependency_graph.rs  # Topological sort + cycle detection
    ├── name_mangler.rs               # Itanium ABI C++ name mangling
    ├── header_manager.rs             # Unity header version matching
    └── unity_version.rs              # Unity version parsing & ranges
```

---

## 📜 License

MIT

---

## 🙏 Credits

- [Perfare/Il2CppDumper](https://github.com/Perfare/Il2CppDumper) — Original C# implementation
- [SamboyCoding/Cpp2IL](https://github.com/SamboyCoding/Cpp2IL) — Advanced C# IL2CPP analysis tool
- [springmusk026/Il2CppDumper-Python](https://github.com/springmusk026/Il2CppDumper-Python) — Python port
- [LukeFZ/Il2CppInspectorRedux](https://github.com/LukeFZ/Il2CppInspectorRedux) — Thanks for the code i used in v4 of my il2cppdumper, but its more faster since the logic its on rust and not C#
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
