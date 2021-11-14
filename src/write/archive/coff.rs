//! Helper for writing COFF archives.
use crate::{
    archive, coff::CoffSymbol, endian, read::coff::CoffFile, write::string, write::WritableBuffer,
};
use alloc::string::ToString;
use std::{collections::{HashMap, HashSet}, vec::Vec, string::String};

/// Informations about .obj file as stored in archive file.
/// todo: improve doc.
#[derive(Debug)]
pub struct ArchiveCoffObjectFile<'a> {
    path: &'a str,
    mode: u32,
    contents: CoffFile<'a>,
}

/// A helper for writing COFF archives (.lib files).
///
/// Writing uses a two phase approach. The first phase prepares string tables and file storage in
/// the order that they will be written.
///
/// The second phase writes everything out in order. As duplicate symbols are known at this point,
/// special member headers ('/', '/' and '//) are also written out.
///
/// The expected use of this class is:
/// ```
/// let mut storage = vec![];
/// let mut writer = coff::Writer(&mut storage);
/// for obj in obj_files {
///     writer.reserve_file(&obj);
/// }
/// writer.flush();
/// ```
///
/// `Writer` also keeps track of duplicate symbols encountered during it's lifetime.
#[allow(missing_debug_implementations)]
pub struct Writer<'a> {
    buffer: &'a mut dyn WritableBuffer,
    object_files: Vec<ArchiveCoffObjectFile<'a>>,
    symbols: Vec<CoffSymbol<'a, 'a>>,
    seen_symbol_names: HashSet<&'a str>,
    string_table: string::StringTable<'a>,
}

impl<'a> Writer<'a> {
    /// Create a new `Writer`.
    pub fn new(buffer: &'a mut dyn WritableBuffer) -> Writer<'a> {
        Writer {
            buffer,
            object_files: vec![],
            symbols: vec![],
            seen_symbol_names: HashSet::new(),
            string_table: string::StringTable::default(),
        }
    }

    /// Returns a stringified offset into string table or `name` if `name`.len() does not exceed
    /// predefined threshold.
    fn get_name(table: &mut string::StringTable<'a>, name: &'a str) -> std::string::String {
        let should_use_string_table = archive::Header::default().name.len() < name.len();
        if should_use_string_table {
            let string::StringId(id) = table.add(&name.as_bytes());
            let ret = format!("/{}", id);
            debug_assert!(ret.len() <= archive::Header::default().name.len());
            ret
        } else {
            name.to_string()
        }
    }

    fn prepare_header(string_table: &mut string::StringTable<'a>, file: &ArchiveCoffObjectFile<'a>) -> archive::Header {
        let name = Self::get_name(string_table, file.path);
        let size = file.contents.data().len().to_string();
        let timestamp = file
            .contents
            .timestamp()
            .get(endian::LittleEndian)
            .to_string();
        let mode = format!("{:o}", file.mode);
        const DEFAULT_FILL: u8 = ' ' as u8;
        let mut ret = archive::Header {
            name: [DEFAULT_FILL; 16],
            date: [DEFAULT_FILL; 12],
            uid: [DEFAULT_FILL; 6],
            gid: [DEFAULT_FILL; 6],
            mode: [DEFAULT_FILL; 8],
            size: [DEFAULT_FILL; 10],
            terminator: archive::TERMINATOR,
        };
        ret.name[..name.len()].copy_from_slice(name.as_bytes());
        ret.date[..timestamp.len()].copy_from_slice(timestamp.as_bytes());
        ret.mode[..mode.len()].copy_from_slice(mode.as_bytes());
        ret.size[..size.len()].copy_from_slice(size.as_bytes());

        ret
    }

    /// Includes given `ArchiveCoffObjectFile` in all subsequent calls to `flush`.
    /// Updates string table & special link members.
    pub fn reserve_file(&mut self, file: ArchiveCoffObjectFile<'a>) {
        self.object_files.push(file);

    }

    /// List of duplicate symbols.
    pub fn duplicate_symbols(&'a self) -> &'a HashSet<&'a str> {
        &self.seen_symbol_names
    }

    fn prepare_first_member(&self) -> Vec<u8> {
        vec![]
    }
    
    fn prepare_second_member(&self) -> Vec<u8> {
        vec![]
    }

    fn prepare_string_table(&self) -> (Vec<u8>, HashMap<std::string::String, u32>)  {
        (vec![], HashMap::new())
    }


    /// Writes out the contents of archive file to buffer.
    pub fn flush(&'a mut self) {
        self.buffer.write_bytes(&archive::MAGIC);
        let first_linker_member = self.prepare_first_member();
        let second_linker_member = self.prepare_second_member();
        let string_table = self.prepare_string_table();
        for object in self.object_files.iter() {
            let header = Self::prepare_header(&mut self.string_table, object);
            self.buffer.write(&header);
            self.buffer.write_bytes(&object.contents.data());
        }


    }
}
