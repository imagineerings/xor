use comfy_types::{CancellationError, CancellationToken};
use std::collections::BTreeSet;
use thiserror::Error;

const MAXIMUM_ELF_TABLE_BYTES: usize = 64 * 1024 * 1024;
const CANCELLATION_CHUNK_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeElfDynamicContract {
    pub(crate) symbols: BTreeSet<String>,
    pub(crate) needed: BTreeSet<String>,
    pub(crate) soname: Option<String>,
}

impl NativeElfDynamicContract {
    pub(crate) fn symbols(&self) -> &BTreeSet<String> {
        &self.symbols
    }

    pub(crate) fn needed(&self) -> &BTreeSet<String> {
        &self.needed
    }

    pub(crate) fn soname(&self) -> Option<&str> {
        self.soname.as_deref()
    }
}

#[derive(Debug, Error)]
pub(crate) enum NativeElfInspectionError {
    #[error("native ELF inspection was cancelled")]
    Cancelled(#[from] CancellationError),
    #[error("invalid native ELF object: {0}")]
    Invalid(String),
}

#[derive(Clone, Copy)]
struct ElfProgramHeader {
    kind: u32,
    file_offset: usize,
    virtual_address: u64,
    file_size: usize,
    memory_size: u64,
}

pub(crate) fn inspect_elf64_dynamic_contract(
    bytes: &[u8],
    expected_machine: u16,
    cancellation: &CancellationToken,
) -> Result<NativeElfDynamicContract, NativeElfInspectionError> {
    cancellation.check()?;
    inspect_elf64_dynamic_contract_inner(bytes, expected_machine, cancellation)
        .map_err(NativeElfInspectionError::Invalid)
}

fn inspect_elf64_dynamic_contract_inner(
    bytes: &[u8],
    expected_machine: u16,
    cancellation: &CancellationToken,
) -> Result<NativeElfDynamicContract, String> {
    check(cancellation)?;
    if bytes.get(0..4) != Some(b"\x7fELF")
        || bytes.get(4) != Some(&2)
        || bytes.get(5) != Some(&1)
        || bytes.get(6) != Some(&1)
    {
        return Err("expected a little-endian ELF64 object".to_owned());
    }
    if read_u16(bytes, 16)? != 3 || read_u16(bytes, 18)? != expected_machine {
        return Err(format!(
            "expected an ELF64 shared object for machine {expected_machine}"
        ));
    }
    let program_offset = usize::try_from(read_u64(bytes, 32)?)
        .map_err(|_| "program table offset exceeds address space".to_owned())?;
    let program_entry_size = usize::from(read_u16(bytes, 54)?);
    let program_count = usize::from(read_u16(bytes, 56)?);
    if program_entry_size < 56 || program_count == 0 {
        return Err("ELF program table is absent or malformed".to_owned());
    }
    let program_table_size = program_entry_size
        .checked_mul(program_count)
        .ok_or_else(|| "ELF program table size overflowed".to_owned())?;
    checked_range(bytes, program_offset, program_table_size)?;
    let mut programs = Vec::with_capacity(program_count);
    for index in 0..program_count {
        check(cancellation)?;
        let offset = program_offset
            .checked_add(
                index
                    .checked_mul(program_entry_size)
                    .ok_or_else(|| "ELF program offset overflowed".to_owned())?,
            )
            .ok_or_else(|| "ELF program offset overflowed".to_owned())?;
        let header = checked_slice(bytes, offset, program_entry_size)?;
        let file_offset = usize::try_from(read_u64(header, 8)?)
            .map_err(|_| "program file offset exceeds address space".to_owned())?;
        let file_size = usize::try_from(read_u64(header, 32)?)
            .map_err(|_| "program file size exceeds address space".to_owned())?;
        let memory_size = read_u64(header, 40)?;
        if u64::try_from(file_size).unwrap_or(u64::MAX) > memory_size {
            return Err("program file size exceeds its memory size".to_owned());
        }
        checked_range(bytes, file_offset, file_size)?;
        programs.push(ElfProgramHeader {
            kind: read_u32(header, 0)?,
            file_offset,
            virtual_address: read_u64(header, 16)?,
            file_size,
            memory_size,
        });
    }
    if !programs.iter().any(|program| program.kind == 1) {
        return Err("ELF object has no loadable segment".to_owned());
    }
    let dynamic_segments = programs
        .iter()
        .filter(|program| program.kind == 2)
        .copied()
        .collect::<Vec<_>>();
    if dynamic_segments.len() != 1 {
        return Err("ELF object must have exactly one PT_DYNAMIC segment".to_owned());
    }
    let dynamic_segment = dynamic_segments[0];
    if dynamic_segment.file_size == 0
        || dynamic_segment.file_size > MAXIMUM_ELF_TABLE_BYTES
        || dynamic_segment.file_size % 16 != 0
    {
        return Err("PT_DYNAMIC has an invalid bounded size".to_owned());
    }
    if !programs
        .iter()
        .filter(|program| program.kind == 1)
        .any(|program| {
            contains_virtual_range(
                *program,
                dynamic_segment.virtual_address,
                dynamic_segment.file_size,
            )
        })
    {
        return Err("PT_DYNAMIC is not contained in a loadable segment".to_owned());
    }
    let dynamic_table = checked_slice(
        bytes,
        dynamic_segment.file_offset,
        dynamic_segment.file_size,
    )?;
    let mut string_address = None;
    let mut string_size = None;
    let mut symbol_address = None;
    let mut symbol_entry_size = None;
    let mut needed_offsets = Vec::new();
    let mut soname_offset = None;
    let mut found_null = false;
    for entry in dynamic_table.chunks_exact(16) {
        check(cancellation)?;
        let tag = read_i64(entry, 0)?;
        let value = read_u64(entry, 8)?;
        match tag {
            0 => {
                found_null = true;
                break;
            }
            1 => needed_offsets.push(value),
            5 => assign_once(&mut string_address, value, "DT_STRTAB")?,
            6 => assign_once(&mut symbol_address, value, "DT_SYMTAB")?,
            10 => assign_once(&mut string_size, value, "DT_STRSZ")?,
            11 => assign_once(&mut symbol_entry_size, value, "DT_SYMENT")?,
            14 => assign_once(&mut soname_offset, value, "DT_SONAME")?,
            15 | 29 => {
                return Err(
                    "ELF RPATH and RUNPATH are forbidden for certified libraries".to_owned(),
                );
            }
            _ => {}
        }
    }
    if !found_null {
        return Err("PT_DYNAMIC has no terminating DT_NULL entry".to_owned());
    }
    let string_address = string_address.ok_or_else(|| "PT_DYNAMIC has no DT_STRTAB".to_owned())?;
    let string_size =
        usize::try_from(string_size.ok_or_else(|| "PT_DYNAMIC has no DT_STRSZ".to_owned())?)
            .map_err(|_| "dynamic string-table size exceeds address space".to_owned())?;
    if string_size == 0 || string_size > MAXIMUM_ELF_TABLE_BYTES {
        return Err("dynamic string table has an invalid bounded size".to_owned());
    }
    let symbol_address = symbol_address.ok_or_else(|| "PT_DYNAMIC has no DT_SYMTAB".to_owned())?;
    let symbol_entry_size =
        usize::try_from(symbol_entry_size.ok_or_else(|| "PT_DYNAMIC has no DT_SYMENT".to_owned())?)
            .map_err(|_| "dynamic-symbol entry size exceeds address space".to_owned())?;
    if symbol_entry_size != 24 {
        return Err("DT_SYMENT does not match the ELF64 symbol size".to_owned());
    }
    let string_offset = virtual_to_file_offset(&programs, string_address, string_size)?;
    let strings = checked_slice(bytes, string_offset, string_size)?;

    let section_offset = usize::try_from(read_u64(bytes, 40)?)
        .map_err(|_| "section table offset exceeds address space".to_owned())?;
    let section_entry_size = usize::from(read_u16(bytes, 58)?);
    let section_count = usize::from(read_u16(bytes, 60)?);
    if section_entry_size < 64 || section_count == 0 {
        return Err("ELF section table is absent or malformed".to_owned());
    }
    let section_table_size = section_entry_size
        .checked_mul(section_count)
        .ok_or_else(|| "ELF section table size overflowed".to_owned())?;
    checked_range(bytes, section_offset, section_table_size)?;
    let section = |index: usize| -> Result<&[u8], String> {
        if index >= section_count {
            return Err("ELF section link is out of bounds".to_owned());
        }
        let offset = section_offset
            .checked_add(
                index
                    .checked_mul(section_entry_size)
                    .ok_or_else(|| "ELF section offset overflowed".to_owned())?,
            )
            .ok_or_else(|| "ELF section offset overflowed".to_owned())?;
        checked_slice(bytes, offset, section_entry_size)
    };
    let mut symbols = BTreeSet::new();
    let mut matched_dynamic_symbols = false;
    for index in 0..section_count {
        check(cancellation)?;
        let header = section(index)?;
        if read_u32(header, 4)? != 11 {
            continue;
        }
        let symbol_offset = usize::try_from(read_u64(header, 24)?)
            .map_err(|_| "dynamic-symbol offset exceeds address space".to_owned())?;
        let symbol_size = usize::try_from(read_u64(header, 32)?)
            .map_err(|_| "dynamic-symbol size exceeds address space".to_owned())?;
        let string_index = usize::try_from(read_u32(header, 40)?)
            .map_err(|_| "string-table index exceeds address space".to_owned())?;
        let section_symbol_entry_size = usize::try_from(read_u64(header, 56)?)
            .map_err(|_| "dynamic-symbol entry size exceeds address space".to_owned())?;
        if section_symbol_entry_size != symbol_entry_size
            || symbol_size == 0
            || symbol_size > MAXIMUM_ELF_TABLE_BYTES
            || symbol_size % section_symbol_entry_size != 0
        {
            return Err("dynamic-symbol table has an invalid entry size".to_owned());
        }
        let symbol_virtual_address = read_u64(header, 16)?;
        let string_header = section(string_index)?;
        if read_u32(string_header, 4)? != 3 || read_u64(string_header, 16)? != string_address {
            return Err("dynamic-symbol table does not link to a string table".to_owned());
        }
        let section_string_offset = usize::try_from(read_u64(string_header, 24)?)
            .map_err(|_| "string-table offset exceeds address space".to_owned())?;
        let section_string_size = usize::try_from(read_u64(string_header, 32)?)
            .map_err(|_| "string-table size exceeds address space".to_owned())?;
        if symbol_virtual_address != symbol_address
            || symbol_offset != virtual_to_file_offset(&programs, symbol_address, symbol_size)?
            || section_string_offset != string_offset
            || section_string_size != string_size
            || read_u64(header, 8)? & 2 == 0
        {
            continue;
        }
        if matched_dynamic_symbols {
            return Err(
                "multiple sections claim the loader-consumed dynamic-symbol table".to_owned(),
            );
        }
        matched_dynamic_symbols = true;
        let table = checked_slice(bytes, symbol_offset, symbol_size)?;
        for entry in table.chunks_exact(section_symbol_entry_size) {
            check(cancellation)?;
            let name_offset = usize::try_from(read_u32(entry, 0)?)
                .map_err(|_| "symbol name offset exceeds address space".to_owned())?;
            let section_index = read_u16(entry, 6)?;
            if name_offset == 0 || section_index == 0 {
                continue;
            }
            let name = dynamic_string(strings, name_offset, cancellation)?;
            if !name.is_empty() {
                symbols.insert(name.to_owned());
            }
        }
    }
    if !matched_dynamic_symbols {
        return Err(
            "ELF object has no section matching the loader-consumed dynamic-symbol table"
                .to_owned(),
        );
    }
    let mut needed = BTreeSet::new();
    for offset in needed_offsets {
        let offset = usize::try_from(offset)
            .map_err(|_| "DT_NEEDED string offset exceeds address space".to_owned())?;
        let dependency = dynamic_string(strings, offset, cancellation)?.to_owned();
        if !needed.insert(dependency.clone()) {
            return Err(format!(
                "PT_DYNAMIC contains duplicate DT_NEEDED entry {dependency}"
            ));
        }
    }
    let soname = soname_offset
        .map(|offset| {
            usize::try_from(offset)
                .map_err(|_| "DT_SONAME string offset exceeds address space".to_owned())
                .and_then(|offset| {
                    dynamic_string(strings, offset, cancellation).map(ToOwned::to_owned)
                })
        })
        .transpose()?;
    check(cancellation)?;
    Ok(NativeElfDynamicContract {
        symbols,
        needed,
        soname,
    })
}

fn check(cancellation: &CancellationToken) -> Result<(), String> {
    cancellation.check().map_err(|error| error.to_string())
}

fn assign_once(slot: &mut Option<u64>, value: u64, name: &str) -> Result<(), String> {
    if slot.replace(value).is_some() {
        Err(format!("PT_DYNAMIC contains duplicate {name}"))
    } else {
        Ok(())
    }
}

fn contains_virtual_range(program: ElfProgramHeader, address: u64, length: usize) -> bool {
    let Ok(length) = u64::try_from(length) else {
        return false;
    };
    let Some(range_end) = address.checked_add(length) else {
        return false;
    };
    let Some(program_end) = program.virtual_address.checked_add(program.memory_size) else {
        return false;
    };
    address >= program.virtual_address && range_end <= program_end
}

fn virtual_to_file_offset(
    programs: &[ElfProgramHeader],
    address: u64,
    length: usize,
) -> Result<usize, String> {
    let mut resolved = None;
    for program in programs.iter().filter(|program| program.kind == 1) {
        if !contains_virtual_range(*program, address, length) {
            continue;
        }
        let delta = usize::try_from(address - program.virtual_address)
            .map_err(|_| "virtual-address delta exceeds address space".to_owned())?;
        let end = delta
            .checked_add(length)
            .ok_or_else(|| "virtual-address range overflowed".to_owned())?;
        if end > program.file_size {
            continue;
        }
        let offset = program
            .file_offset
            .checked_add(delta)
            .ok_or_else(|| "mapped file offset overflowed".to_owned())?;
        if resolved.is_some_and(|prior| prior != offset) {
            return Err("virtual address maps ambiguously to multiple file offsets".to_owned());
        }
        resolved = Some(offset);
    }
    resolved.ok_or_else(|| "dynamic virtual address is not file-backed by PT_LOAD".to_owned())
}

fn dynamic_string<'a>(
    strings: &'a [u8],
    offset: usize,
    cancellation: &CancellationToken,
) -> Result<&'a str, String> {
    let suffix = strings
        .get(offset..)
        .ok_or_else(|| "dynamic string is outside the string table".to_owned())?;
    for (chunk_index, chunk) in suffix.chunks(CANCELLATION_CHUNK_BYTES).enumerate() {
        check(cancellation)?;
        if let Some(position) = chunk.iter().position(|byte| *byte == 0) {
            let end = chunk_index
                .checked_mul(CANCELLATION_CHUNK_BYTES)
                .and_then(|start| start.checked_add(position))
                .ok_or_else(|| "dynamic string length overflowed".to_owned())?;
            return std::str::from_utf8(&suffix[..end])
                .map_err(|_| "dynamic string is not UTF-8".to_owned());
        }
    }
    Err("dynamic string is not NUL terminated".to_owned())
}

fn checked_range(bytes: &[u8], offset: usize, length: usize) -> Result<(), String> {
    offset
        .checked_add(length)
        .filter(|end| *end <= bytes.len())
        .map(|_| ())
        .ok_or_else(|| "ELF range is out of bounds".to_owned())
}

fn checked_slice(bytes: &[u8], offset: usize, length: usize) -> Result<&[u8], String> {
    checked_range(bytes, offset, length)?;
    Ok(&bytes[offset..offset + length])
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, String> {
    let value = checked_slice(bytes, offset, 2)?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let value = checked_slice(bytes, offset, 4)?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, String> {
    let value = checked_slice(bytes, offset, 8)?;
    Ok(u64::from_le_bytes([
        value[0], value[1], value[2], value[3], value[4], value[5], value[6], value[7],
    ]))
}

fn read_i64(bytes: &[u8], offset: usize) -> Result<i64, String> {
    let value = checked_slice(bytes, offset, 8)?;
    Ok(i64::from_le_bytes([
        value[0], value[1], value[2], value[3], value[4], value[5], value[6], value[7],
    ]))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    pub(crate) fn fixture(
        machine: u16,
        symbols: &BTreeSet<String>,
        needed: &[&str],
        runpath: Option<&str>,
        soname: &str,
    ) -> Vec<u8> {
        let mut strings = vec![0_u8];
        let mut names = Vec::new();
        for symbol in symbols {
            names.push(u32::try_from(strings.len()).unwrap_or_default());
            strings.extend_from_slice(symbol.as_bytes());
            strings.push(0);
        }
        let mut needed_offsets = Vec::new();
        for dependency in needed {
            needed_offsets.push(u64::try_from(strings.len()).unwrap_or_default());
            strings.extend_from_slice(dependency.as_bytes());
            strings.push(0);
        }
        let runpath_offset = runpath.map(|path| {
            let offset = u64::try_from(strings.len()).unwrap_or_default();
            strings.extend_from_slice(path.as_bytes());
            strings.push(0);
            offset
        });
        let soname_offset = u64::try_from(strings.len()).unwrap_or_default();
        strings.extend_from_slice(soname.as_bytes());
        strings.push(0);
        let program_offset = 64_usize;
        let program_entry_size = 56_usize;
        let program_count = 2_usize;
        let string_offset = 192_usize;
        let symbol_offset = (string_offset + strings.len() + 7) & !7;
        let symbol_size = (symbols.len() + 1) * 24;
        let dynamic_offset = (symbol_offset + symbol_size + 7) & !7;
        let dynamic_entries = 6 + needed_offsets.len() + usize::from(runpath_offset.is_some());
        let dynamic_size = dynamic_entries * 16;
        let section_offset = (dynamic_offset + dynamic_size + 7) & !7;
        let mut bytes = vec![0_u8; section_offset + 4 * 64];
        bytes[..4].copy_from_slice(b"\x7fELF");
        bytes[4] = 2;
        bytes[5] = 1;
        bytes[6] = 1;
        write_u16(&mut bytes, 16, 3);
        write_u16(&mut bytes, 18, machine);
        write_u64(
            &mut bytes,
            32,
            u64::try_from(program_offset).unwrap_or_default(),
        );
        write_u64(
            &mut bytes,
            40,
            u64::try_from(section_offset).unwrap_or_default(),
        );
        write_u16(
            &mut bytes,
            54,
            u16::try_from(program_entry_size).unwrap_or_default(),
        );
        write_u16(
            &mut bytes,
            56,
            u16::try_from(program_count).unwrap_or_default(),
        );
        write_u16(&mut bytes, 58, 64);
        write_u16(&mut bytes, 60, 4);
        let file_length = u64::try_from(bytes.len()).unwrap_or_default();
        write_u32(&mut bytes, program_offset, 1);
        write_u32(&mut bytes, program_offset + 4, 4);
        write_u64(&mut bytes, program_offset + 32, file_length);
        write_u64(&mut bytes, program_offset + 40, file_length);
        write_u64(&mut bytes, program_offset + 48, 8);
        let dynamic_program = program_offset + program_entry_size;
        write_u32(&mut bytes, dynamic_program, 2);
        write_u32(&mut bytes, dynamic_program + 4, 4);
        write_u64(
            &mut bytes,
            dynamic_program + 8,
            u64::try_from(dynamic_offset).unwrap_or_default(),
        );
        write_u64(
            &mut bytes,
            dynamic_program + 16,
            u64::try_from(dynamic_offset).unwrap_or_default(),
        );
        write_u64(
            &mut bytes,
            dynamic_program + 32,
            u64::try_from(dynamic_size).unwrap_or_default(),
        );
        write_u64(
            &mut bytes,
            dynamic_program + 40,
            u64::try_from(dynamic_size).unwrap_or_default(),
        );
        write_u64(&mut bytes, dynamic_program + 48, 8);
        bytes[string_offset..string_offset + strings.len()].copy_from_slice(&strings);
        for (index, name_offset) in names.into_iter().enumerate() {
            let entry = symbol_offset + (index + 1) * 24;
            write_u32(&mut bytes, entry, name_offset);
            bytes[entry + 4] = 0x12;
            write_u16(&mut bytes, entry + 6, 1);
        }
        let strings_header = section_offset + 64;
        write_u32(&mut bytes, strings_header + 4, 3);
        write_u64(
            &mut bytes,
            strings_header + 16,
            u64::try_from(string_offset).unwrap_or_default(),
        );
        write_u64(
            &mut bytes,
            strings_header + 24,
            u64::try_from(string_offset).unwrap_or_default(),
        );
        write_u64(
            &mut bytes,
            strings_header + 32,
            u64::try_from(strings.len()).unwrap_or_default(),
        );
        let symbols_header = section_offset + 128;
        write_u32(&mut bytes, symbols_header + 4, 11);
        write_u64(&mut bytes, symbols_header + 8, 2);
        write_u64(
            &mut bytes,
            symbols_header + 16,
            u64::try_from(symbol_offset).unwrap_or_default(),
        );
        write_u64(
            &mut bytes,
            symbols_header + 24,
            u64::try_from(symbol_offset).unwrap_or_default(),
        );
        write_u64(
            &mut bytes,
            symbols_header + 32,
            u64::try_from(symbol_size).unwrap_or_default(),
        );
        write_u32(&mut bytes, symbols_header + 40, 1);
        write_u64(&mut bytes, symbols_header + 56, 24);
        let mut dynamic_index = 0_usize;
        for (tag, value) in [
            (5, u64::try_from(string_offset).unwrap_or_default()),
            (10, u64::try_from(strings.len()).unwrap_or_default()),
            (6, u64::try_from(symbol_offset).unwrap_or_default()),
            (11, 24),
            (14, soname_offset),
        ] {
            let entry = dynamic_offset + dynamic_index * 16;
            write_u64(&mut bytes, entry, tag);
            write_u64(&mut bytes, entry + 8, value);
            dynamic_index += 1;
        }
        for offset in needed_offsets {
            let entry = dynamic_offset + dynamic_index * 16;
            write_u64(&mut bytes, entry, 1);
            write_u64(&mut bytes, entry + 8, offset);
            dynamic_index += 1;
        }
        if let Some(offset) = runpath_offset {
            let entry = dynamic_offset + dynamic_index * 16;
            write_u64(&mut bytes, entry, 29);
            write_u64(&mut bytes, entry + 8, offset);
            dynamic_index += 1;
        }
        write_u64(&mut bytes, dynamic_offset + dynamic_index * 16, 0);
        let dynamic_header = section_offset + 192;
        write_u32(&mut bytes, dynamic_header + 4, 6);
        write_u64(
            &mut bytes,
            dynamic_header + 16,
            u64::try_from(dynamic_offset).unwrap_or_default(),
        );
        write_u64(
            &mut bytes,
            dynamic_header + 24,
            u64::try_from(dynamic_offset).unwrap_or_default(),
        );
        write_u64(
            &mut bytes,
            dynamic_header + 32,
            u64::try_from(dynamic_size).unwrap_or_default(),
        );
        write_u32(&mut bytes, dynamic_header + 40, 1);
        write_u64(&mut bytes, dynamic_header + 56, 16);
        bytes
    }

    #[test]
    fn elf_inspection_binds_machine_soname_symbols_and_dependencies() {
        let symbols = BTreeSet::from(["avcodec_open2".to_owned(), "avcodec_send_frame".to_owned()]);
        let bytes = fixture(
            62,
            &symbols,
            &["libavutil.so.59", "libc.so.6"],
            None,
            "libavcodec.so.61",
        );
        let contract = inspect_elf64_dynamic_contract(&bytes, 62, &CancellationToken::default())
            .expect("synthetic ELF should be admitted");
        assert_eq!(contract.soname(), Some("libavcodec.so.61"));
        assert_eq!(contract.symbols(), &symbols);
        assert_eq!(
            contract.needed(),
            &BTreeSet::from(["libavutil.so.59".to_owned(), "libc.so.6".to_owned()])
        );
        assert!(
            inspect_elf64_dynamic_contract(&bytes, 183, &CancellationToken::default()).is_err()
        );
    }

    #[test]
    fn elf_inspection_rejects_duplicate_needed_entries() {
        let bytes = fixture(
            62,
            &BTreeSet::from(["avcodec_open2".to_owned()]),
            &["libc.so.6", "libc.so.6"],
            None,
            "libavcodec.so.61",
        );
        let error = inspect_elf64_dynamic_contract(&bytes, 62, &CancellationToken::default())
            .expect_err("duplicate DT_NEEDED entries must fail closed");
        assert!(error.to_string().contains("duplicate DT_NEEDED"));
    }

    #[test]
    fn elf_inspection_rejects_embedded_search_paths_and_cancellation() {
        let bytes = fixture(
            183,
            &BTreeSet::new(),
            &[],
            Some("/ambient"),
            "libavutil.so.59",
        );
        assert!(
            inspect_elf64_dynamic_contract(&bytes, 183, &CancellationToken::default()).is_err()
        );
        let cancellation = CancellationToken::default();
        cancellation.cancel();
        assert!(matches!(
            inspect_elf64_dynamic_contract(&bytes, 183, &cancellation),
            Err(NativeElfInspectionError::Cancelled(_))
        ));
    }
}
