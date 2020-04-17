use std::{env, fs::File, io::{self, Read, Seek, SeekFrom, Write}, mem, slice};

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        panic!("ERROR: No map file supplied");
    }

    let mut input_file = File::open(args[1].clone())?;
    let mut output_file = File::create(args[1].replace(".map", "_decompressed.map"))?;

    let mut data = vec![0u8; 0x1000];

    input_file.seek(SeekFrom::Start(0))?;
    input_file.read_exact(&mut data[..])?;

    #[repr(C)]
    struct CompressedSection {
        pub size: u32,
        pub offset: u32
    }

    let mut sections = vec![];

    for _ in 0..1024 {
        let mut section = CompressedSection { size: 0, offset: 0 };
        
        input_file.read_exact(unsafe {
            slice::from_raw_parts_mut(&mut section as *mut _ as *mut u8, mem::size_of_val(&section))
        })?;

        if section.size == 0 || section.offset < 0x3000 {
            break;
        }

        sections.push(section);
    }

    for section in sections {
        if section.offset == 0 && section.size == 0 {
            break;
        }

        let mut section_data = vec![0u8; section.size as usize];
        input_file.seek(SeekFrom::Start(section.offset as u64))?;
        input_file.read_exact(&mut section_data[..])?;

        match inflate::inflate_bytes_zlib(&section_data[..]) {
            Ok(inflated_data) => data.extend_from_slice(&inflated_data[..]),
            Err(error_message) => return Err(io::Error::new(io::ErrorKind::InvalidData, error_message))
        }
    }

    output_file.write_all(&data[..])
}
