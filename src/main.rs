use std::{env, fs::File, io::{self, Read, Seek, SeekFrom, Write}, mem, slice};

fn print_usage() {
	println!("Usage: h2cachetool <compress|decompress|pack> <input> <output>");
	std::process::exit(-1);
}

fn unpack(mut input_file: &File, mut output_file: &File) -> io::Result<()> {
	// read header
    let mut data = vec![0u8; 0x1000];

    input_file.seek(SeekFrom::Start(0))?;
    input_file.read_exact(&mut data[..])?;

    #[repr(C)]
    struct CompressedSection {
        pub size: i32,
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
		
		// negative size is used to mark chunk as uncompressed
		let size = section.size.abs() as usize;

        let mut section_data = vec![0u8; size];
        input_file.seek(SeekFrom::Start(section.offset as u64))?;
        input_file.read_exact(&mut section_data[..])?;
		
		if section.size < 0 {
			println!("Uncompressed chunk@ {}, size {}", section.offset, size);
			data.extend_from_slice(&section_data[..])
		} else {
			match inflate::inflate_bytes_zlib(&section_data[..]) {
				Ok(inflated_data) => { 
					println!("Compressed chunk@ {}, size {}", section.offset, inflated_data.len());
					data.extend_from_slice(&inflated_data[..])
					},
				Err(error_message) => return Err(io::Error::new(io::ErrorKind::InvalidData, error_message))
			}
		}
    }

    output_file.write_all(&data[..])
}

fn pack(mut input_file: &File, mut output_file: &File, should_compress: bool) -> io::Result<()> {
	// read header
    let mut data = vec![0u8; 0x1000];

    input_file.seek(SeekFrom::Start(0))?;
    input_file.read_exact(&mut data[..])?;
	
	// reverse data for compression info
	data.resize(0x3000, 0);

    #[repr(C)]
    struct CompressedSection {
        pub size: i32,
        pub offset: u32
    }
	
	println!("Processing sections...");

	let mut sections = vec![];
	let mut section_index = 0;
	loop
	{
		if section_index >= 1024 {
			panic!("File too large, need more than 1024 sections!");
		}
		
		let mut section = CompressedSection { size: 0, offset: 0 };
		section.offset = data.len() as u32;
		
		let mut section_data = vec![0u8; 0x40000];
		let len = input_file.read(&mut section_data[..])?;
		
		if len == 0 {
			break;
		}
		
		if should_compress {
			let compressed_data = deflate::deflate_bytes_zlib(&section_data[..len]);
			data.extend_from_slice(&compressed_data[..]);
			section.size = compressed_data.len() as i32;
		} else {
			data.extend_from_slice(&section_data[..len]);
			section.size = -(len as i32); // size is negative for uncompressed sections
		}
		
		sections.push(section);

		section_index += 1;
	}
	
	println!("Writing map data...");

    output_file.write_all(&data[..])?;
	
	println!("Writing section info...");
	// seek to end of header
	output_file.seek(SeekFrom::Start(0x1000))?;
	
	for section in sections {
		output_file.write_all(unsafe {
            slice::from_raw_parts(&section as *const _ as *const u8, mem::size_of_val(&section))
        })?;
	};
	Ok(())
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 4 {
		print_usage();
    }

    let input_file = File::open(args[2].clone())?;
    let output_file = File::create(args[3].clone())?;
	
	if args[1].starts_with('d') {
		unpack(&input_file, &output_file)
	} else if args[1].starts_with('c') {
		pack(&input_file, &output_file, true)
	} else if args[1].starts_with('p') {
		pack(&input_file, &output_file, false)
	} else {
		Ok(print_usage())
	}
}
