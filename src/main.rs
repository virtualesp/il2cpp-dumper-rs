use std::fs;
use std::io::{self, Write, BufRead};
use std::process;

use clap::Parser;

use il2cpp_dumper::config::Config;
use il2cpp_dumper::error::Result;
use il2cpp_dumper::il2cpp::metadata::Metadata;
use il2cpp_dumper::il2cpp::base::{Il2Cpp, VaSegment};
use il2cpp_dumper::executor::Il2CppExecutor;
use il2cpp_dumper::output::decompiler::Il2CppDecompiler;
use il2cpp_dumper::output::struct_generator::StructGenerator;
use il2cpp_dumper::formats::elf::Elf;
use il2cpp_dumper::formats::pe::Pe;
use il2cpp_dumper::formats::macho::MachO;
use il2cpp_dumper::formats::nso::Nso;

const MAGIC_METADATA: u32 = 0xFAB11BAF;
const MAGIC_ELF: u32 = 0x464C457F;
const MAGIC_PE: u16 = 0x5A4D;
const MAGIC_MACHO32: u32 = 0xFEEDFACE;
const MAGIC_MACHO64: u32 = 0xFEEDFACF;
const MAGIC_MACHOFAT: u32 = 0xBEBAFECA;
const MAGIC_NSO: u32 = 0x304F534E;

#[derive(Parser)]
#[command(name = "il2cpp_dumper", version, about = "IL2CPP Dumper - Rust Port")]
struct Cli {
    il2cpp_binary: String,
    metadata: String,
    #[arg(default_value = ".")]
    output_dir: String,
    #[arg(long)]
    config: Option<String>,
}

fn read_magic_u32(data: &[u8]) -> u32 {
    if data.len() < 4 { return 0; }
    u32::from_le_bytes([data[0], data[1], data[2], data[3]])
}

fn read_magic_u16(data: &[u8]) -> u16 {
    if data.len() < 2 { return 0; }
    u16::from_le_bytes([data[0], data[1]])
}

fn prompt_dump_address() -> Option<u64> {
    print!("Input il2cpp dump address or input 0 to force continue: ");
    io::stdout().flush().ok();
    let stdin = io::stdin();
    if let Some(Ok(line)) = stdin.lock().lines().next() {
        if let Ok(addr) = u64::from_str_radix(line.trim().trim_start_matches("0x"), 16) {
            if addr != 0 {
                return Some(addr);
            }
        }
    }
    None
}

fn prompt_manual_addresses() -> Result<(u64, u64)> {
    print!("Input CodeRegistration (hex): ");
    io::stdout().flush().ok();
    let stdin = io::stdin();
    let cr_line = stdin.lock().lines().next()
        .unwrap_or(Ok(String::new()))
        .unwrap_or_default();
    print!("Input MetadataRegistration (hex): ");
    io::stdout().flush().ok();
    let mr_line = stdin.lock().lines().next()
        .unwrap_or(Ok(String::new()))
        .unwrap_or_default();

    let cr = u64::from_str_radix(cr_line.trim().trim_start_matches("0x"), 16)
        .map_err(|_| il2cpp_dumper::error::Error::Other("Invalid code registration address".into()))?;
    let mr = u64::from_str_radix(mr_line.trim().trim_start_matches("0x"), 16)
        .map_err(|_| il2cpp_dumper::error::Error::Other("Invalid metadata registration address".into()))?;
    Ok((cr, mr))
}

fn init_elf(data: Vec<u8>, metadata: &Metadata, config: &Config) -> Result<Il2Cpp> {
    let is_64 = data.len() > 4 && data[4] == 2;
    println!("Detected ELF{} format", if is_64 { "64" } else { "32" });

    let mut elf = Elf::new(data, !is_64)?;

    let version = if config.force_il2cpp_version {
        config.force_version
    } else {
        metadata.version
    };

    elf.set_properties(version, metadata.metadata_usages_count as u64);
    println!("IL2CPP Version: {}", elf.stream.version);

    if config.force_dump || elf.check_dump() {
        println!("Detected this may be a dump file.");
        if let Some(addr) = prompt_dump_address() {
            elf.stream.image_base = addr;
            elf.is_dumped = true;
        }
    }

    println!("Searching...");
    let method_count = metadata.method_defs.iter().filter(|m| m.method_index >= 0).count();
    let type_count = metadata.type_defs.len();
    let image_count = metadata.image_defs.len();

    let mut helper = elf.get_section_helper(method_count, type_count, image_count);
    let code_reg = helper.find_code_registration();
    let metadata_reg = helper.find_metadata_registration();

    let mut found = elf.auto_plus_init(code_reg, metadata_reg)?;

    if !found {
        if let Ok(Some((cr, mr))) = elf.symbol_search() {
            println!("Detected Symbol!");
            println!("CodeRegistration : 0x{cr:x}");
            println!("MetadataRegistration : 0x{mr:x}");
            elf.init(cr, mr)?;
            found = true;
        }
    }

    if !found {
        println!("ERROR: Can't use auto mode to process file, try manual mode.");
        let (cr, mr) = prompt_manual_addresses()?;
        elf.init(cr, mr)?;
    }

    Ok(Il2Cpp::from_elf(&elf))
}

fn init_pe(data: Vec<u8>, metadata: &Metadata, config: &Config) -> Result<Il2Cpp> {
    let mut pe = Pe::new(data)?;
    println!("Detected PE{} format", if pe.is_32bit { "32" } else { "64" });

    let version = if config.force_il2cpp_version {
        config.force_version
    } else {
        metadata.version
    };

    pe.stream.version = version;
    pe.stream.is_32bit = pe.is_32bit;
    println!("IL2CPP Version: {version}");

    if config.force_dump || pe.check_dump() {
        println!("Detected this may be a dump file.");
        if let Some(addr) = prompt_dump_address() {
            pe.stream.image_base = addr;
        }
    }

    println!("Searching...");
    let method_count = metadata.method_defs.iter().filter(|m| m.method_index >= 0).count();
    let type_count = metadata.type_defs.len();
    let image_count = metadata.image_defs.len();
    let mu_count = metadata.metadata_usages_count;

    let mut cr_addr = 0u64;
    let mut mr_addr = 0u64;

    if let Ok(Some((cr, mr))) = pe.symbol_search() {
        println!("Detected Symbol!");
        println!("CodeRegistration : 0x{cr:x}");
        println!("MetadataRegistration : 0x{mr:x}");
        cr_addr = cr;
        mr_addr = mr;
    }

    if cr_addr == 0 || mr_addr == 0 {
        let mut helper = pe.get_section_helper(method_count, type_count, mu_count, image_count, version);
        let code_reg = helper.find_code_registration();
        let metadata_reg = helper.find_metadata_registration();
        if let (Some(cr), Some(mr)) = (code_reg, metadata_reg) {
            cr_addr = cr;
            mr_addr = mr;
        }
    }

    if cr_addr == 0 || mr_addr == 0 {
        println!("ERROR: Can't use auto mode to process file, try manual mode.");
        let (cr, mr) = prompt_manual_addresses()?;
        cr_addr = cr;
        mr_addr = mr;
    }

    let pe_image_base = pe.image_base();
    let va_segments: Vec<VaSegment> = pe.sections.iter().map(|s| {
        VaSegment {
            vaddr: s.virtual_address as u64 + pe_image_base,
            memsz: s.virtual_size as u64,
            offset: s.pointer_to_raw_data as u64,
        }
    }).collect();

    let mut il2cpp = Il2Cpp::new(pe.stream.clone(), version, pe.is_32bit);
    il2cpp.va_segments = va_segments;
    il2cpp.image_base = pe_image_base;
    il2cpp.init(cr_addr, mr_addr, &|addr| pe.map_vatr(addr))?;
    Ok(il2cpp)
}

fn init_macho(data: Vec<u8>, metadata: &Metadata, config: &Config) -> Result<Il2Cpp> {
    let magic = read_magic_u32(&data);
    let is_64 = magic == MAGIC_MACHO64;
    println!("Detected Mach-O{} format", if is_64 { " 64-bit" } else { " 32-bit" });

    let mut macho = MachO::new(data, !is_64)?;

    let version = if config.force_il2cpp_version {
        config.force_version
    } else {
        metadata.version
    };

    macho.stream.version = version;
    println!("IL2CPP Version: {version}");

    println!("Searching...");
    let method_count = metadata.method_defs.iter().filter(|m| m.method_index >= 0).count();
    let type_count = metadata.type_defs.len();
    let image_count = metadata.image_defs.len();
    let mu_count = metadata.metadata_usages_count;

    let mut cr_addr = 0u64;
    let mut mr_addr = 0u64;

    if let Some((cr, mr)) = macho.symbol_search() {
        println!("Detected Symbol!");
        println!("CodeRegistration : 0x{cr:x}");
        println!("MetadataRegistration : 0x{mr:x}");
        cr_addr = cr;
        mr_addr = mr;
    }

    if cr_addr == 0 || mr_addr == 0 {
        let mut helper = macho.get_section_helper(method_count, type_count, mu_count, image_count, version);
        let code_reg = helper.find_code_registration();
        let metadata_reg = helper.find_metadata_registration();
        if let (Some(cr), Some(mr)) = (code_reg, metadata_reg) {
            cr_addr = cr;
            mr_addr = mr;
        }
    }

    if cr_addr == 0 || mr_addr == 0 {
        println!("ERROR: Can't use auto mode to process file, try manual mode.");
        let (cr, mr) = prompt_manual_addresses()?;
        cr_addr = cr;
        mr_addr = mr;
    }

    let va_segments: Vec<VaSegment> = macho.segments.iter().map(|s| {
        VaSegment {
            vaddr: s.vmaddr,
            memsz: s.vmsize,
            offset: s.fileoff,
        }
    }).collect();

    let mut il2cpp = Il2Cpp::new(macho.stream.clone(), version, macho.is_32bit);
    il2cpp.va_segments = va_segments;
    il2cpp.init(cr_addr, mr_addr, &|addr| macho.map_vatr(addr))?;
    Ok(il2cpp)
}

fn init_nso(data: Vec<u8>, metadata: &Metadata, config: &Config) -> Result<Il2Cpp> {
    println!("Detected NSO format");

    let nso = Nso::new(data)?;

    let version = if config.force_il2cpp_version {
        config.force_version
    } else {
        metadata.version
    };

    println!("IL2CPP Version: {version}");

    println!("Searching...");
    let method_count = metadata.method_defs.iter().filter(|m| m.method_index >= 0).count();
    let type_count = metadata.type_defs.len();
    let image_count = metadata.image_defs.len();
    let mu_count = metadata.metadata_usages_count;

    let mut helper = nso.get_section_helper(method_count, type_count, mu_count, image_count, version);
    let code_reg = helper.find_code_registration();
    let metadata_reg = helper.find_metadata_registration();

    let (cr_addr, mr_addr) = if let (Some(cr), Some(mr)) = (code_reg, metadata_reg) {
        (cr, mr)
    } else {
        println!("ERROR: Can't use auto mode to process file, try manual mode.");
        prompt_manual_addresses()?
    };

    let stream_len = nso.stream.data().len() as u64;
    let mut il2cpp = Il2Cpp::new(nso.stream.clone(), version, nso.is_32bit);
    il2cpp.va_segments = vec![VaSegment { vaddr: 0, memsz: stream_len, offset: 0 }];
    il2cpp.init(cr_addr, mr_addr, &|addr| nso.map_vatr(addr))?;
    Ok(il2cpp)
}

fn detect_format(data: &[u8]) -> &'static str {
    let magic32 = read_magic_u32(data);
    let magic16 = read_magic_u16(data);
    match magic32 {
        MAGIC_ELF => "elf",
        MAGIC_MACHO32 | MAGIC_MACHO64 | MAGIC_MACHOFAT => "macho",
        MAGIC_NSO => "nso",
        _ if magic16 == MAGIC_PE => "pe",
        _ => "unknown",
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    let config = if let Some(ref cp) = cli.config {
        Config::load_from_file(cp).unwrap_or_else(|e| {
            eprintln!("Warning: Failed to load config: {e}. Using defaults.");
            Config::default()
        })
    } else if std::path::Path::new("config.json").exists() {
        Config::load_from_file("config.json").unwrap_or_default()
    } else {
        Config::default()
    };

    fs::create_dir_all(&cli.output_dir).ok();

    println!("Initializing metadata...");
    let metadata_bytes = fs::read(&cli.metadata)?;
    let metadata_magic = read_magic_u32(&metadata_bytes);
    if metadata_magic != MAGIC_METADATA {
        return Err(il2cpp_dumper::error::Error::Other(
            format!("Invalid metadata file (magic: 0x{metadata_magic:08X})")
        ));
    }
    let mut metadata = Metadata::new(metadata_bytes)?;
    println!("Metadata Version: {}", metadata.version);

    println!("Initializing IL2CPP binary...");
    let il2cpp_bytes = fs::read(&cli.il2cpp_binary)?;
    let format = detect_format(&il2cpp_bytes);

    let mut il2cpp = match format {
        "elf" => init_elf(il2cpp_bytes, &metadata, &config)?,
        "pe" => init_pe(il2cpp_bytes, &metadata, &config)?,
        "macho" => init_macho(il2cpp_bytes, &metadata, &config)?,
        "nso" => init_nso(il2cpp_bytes, &metadata, &config)?,
        _ => {
            let magic = read_magic_u32(&il2cpp_bytes);
            return Err(il2cpp_dumper::error::Error::Other(
                format!("Unsupported binary format (magic: 0x{magic:08X})")
            ));
        }
    };

    println!("Dumping...");
    let mut executor = Il2CppExecutor::new(&metadata, &mut il2cpp)?;

    Il2CppDecompiler::decompile(&mut executor, &mut metadata, &mut il2cpp, &config, &cli.output_dir)?;
    println!("dump.cs generated");

    if config.generate_struct {
        println!("Generating struct...");
        StructGenerator::write_all(&mut executor, &mut metadata, &mut il2cpp, &cli.output_dir)?;
        println!("script.json, il2cpp.h, stringliteral.json generated");
    }

    if config.generate_dummy_dll {
        println!("Generating dummy dll...");
        il2cpp_dumper::output::dummy_assembly_generator::generate_dummy_dlls(
            &mut executor, &mut metadata, &mut il2cpp, &config, &cli.output_dir,
        )?;
        println!("Dummy dll files generated");
    }

    println!("Done!");

    if config.require_any_key {
        print!("Press Enter to exit...");
        io::stdout().flush().ok();
        let _ = io::stdin().read_line(&mut String::new());
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("ERROR: {e}");
        process::exit(1);
    }
}
