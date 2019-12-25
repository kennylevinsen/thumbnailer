/**
    This file is part of Thumbnailer.

    Thumbnailer is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License.

    Thumbnailer is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with Thumbnailer.  If not, see <http://www.gnu.org/licenses/>.
*/

struct Png {}

struct Chunk {
    kind: String,
    length: u32,
    data: Vec<u8>,
    crc: u32
}

impl Chunk {
    fn new_text(keyword: &str,  text: &str) -> Result<Chunk, ()> {
        if keyword.is_empty() || keyword.len() > 79 || keyword.contains('\0') {
            return Err(());
        }

        if text.contains('\0') {
            return Err(());
        }

        let text = text.to_owned().replace("\r\n", "\n");

        if text.is_empty() {
            return Err(());
        }

        let mut r = Chunk {
            kind: "tEXt".to_owned(),
            length: 0,
            data: vec![],
            crc: 0
        };
        r.data.extend_from_slice(keyword.as_bytes());
        r.data.push(0);
        r.data.extend_from_slice(text.as_bytes());

        r.length = r.data.len() as u32;

        let mut crc = CRC::new();
        crc.update(r.kind.as_ref());
        if !r.data.is_empty() {
            crc.update(&r.data)
        }

        r.crc = crc.value();

        Ok(r)
    }
}

impl Png {
    const PNG_SIGNATURE: [u8; 8] =  [137, 80, 78, 71, 13, 10, 26, 10];

    fn decode(input: &mut dyn std::io::Read) -> Result<Vec<Chunk>, ()> {
        Png::decode_signature(input)?;
        Png::decode_all_chunks(input)
    }

    fn decode_signature(input: &mut dyn std::io::Read) -> Result<(), ()> {
        let mut buf: [u8; 8] = Default::default();
        input.read_exact(&mut buf).map_err(|_|())?;
        if buf == Png::PNG_SIGNATURE { Ok(()) } else { Err(()) }
    }

    fn decode_all_chunks(input: &mut dyn std::io::Read) -> Result<Vec<Chunk>, ()>{
        let mut result = vec!();
        loop {
            let chunk = Png::decode_chunk(input).map_err(|_|())?;
            result.push(chunk);
            if result.last().unwrap().kind == "IEND" {
                return Ok(result)
            }
        }
    }

    fn decode_chunk(input: &mut dyn std::io::Read) -> std::result::Result<Chunk, ()> {
        let mut crc = CRC::new();
        let mut chunk_length: [u8; 4] = Default::default();
        let mut chunk_kind: [u8; 4] = Default::default();
        let mut chunk_crc: [u8; 4] = Default::default();
        let mut chunk_data: Vec<u8> = vec![];

        input.read_exact(&mut chunk_length).map_err(|_e| ())?;
        let chunk_length = unsafe { std::mem::transmute::<[u8; 4], u32>(chunk_length).to_be()};

        input.read_exact(&mut chunk_kind).map_err(|_| ())?;
        crc.update(&chunk_kind);
        let chunk_kind = String::from_utf8(chunk_kind.to_vec()).map_err(|_|())?;

        if chunk_length > 0 {
            chunk_data.resize(chunk_length as usize, 0);
            input.read_exact(&mut chunk_data).map_err(|_|())?;
            crc.update(&chunk_data)
        }

        input.read_exact(&mut chunk_crc).map_err(|_|())?;
        let chunk_crc = unsafe { std::mem::transmute::<[u8; 4], u32>(chunk_crc).to_be()};

        if chunk_crc == crc.value() {
            Ok(Chunk {
                length: chunk_length,
                data: chunk_data,
                kind: chunk_kind,
                crc: chunk_crc
            })
        } else {
            Err(())
        }
    }

    fn encode(output: &mut dyn std::io::Write, chunks: &Vec<Chunk>) -> Result<(), ()> {
        output.write_all(Png::PNG_SIGNATURE.as_ref()).map_err(|_e|())?;
        for c in chunks {
            let mut crc = CRC::new();

            let length: [u8; 4] = unsafe { std::mem::transmute((c.data.len() as u32).to_be()) };
            output.write_all(&length).map_err(|_e|())?;

            let kind: &[u8] = c.kind.as_ref();
            if kind.len() != 4 {
                return Err(());
            }
            crc.update(&kind);
            output.write_all(kind).map_err(|_e|())?;

            if !c.data.is_empty() {
                crc.update(&c.data);
                output.write_all(&c.data).map_err(|_e|())?;
            }

            let length: [u8; 4] = unsafe { std::mem::transmute(crc.value().to_be()) };
            output.write_all(&length).map_err(|_e|())?;
        }

        Ok(())
    }
}

struct CRC {
    c: u32
}

impl CRC {
    const CRC_TABLE: [u32; 256] = [
        0x00000000, 0x77073096, 0xee0e612c, 0x990951ba, 0x076dc419,
        0x706af48f, 0xe963a535, 0x9e6495a3, 0x0edb8832, 0x79dcb8a4,
        0xe0d5e91e, 0x97d2d988, 0x09b64c2b, 0x7eb17cbd, 0xe7b82d07,
        0x90bf1d91, 0x1db71064, 0x6ab020f2, 0xf3b97148, 0x84be41de,
        0x1adad47d, 0x6ddde4eb, 0xf4d4b551, 0x83d385c7, 0x136c9856,
        0x646ba8c0, 0xfd62f97a, 0x8a65c9ec, 0x14015c4f, 0x63066cd9,
        0xfa0f3d63, 0x8d080df5, 0x3b6e20c8, 0x4c69105e, 0xd56041e4,
        0xa2677172, 0x3c03e4d1, 0x4b04d447, 0xd20d85fd, 0xa50ab56b,
        0x35b5a8fa, 0x42b2986c, 0xdbbbc9d6, 0xacbcf940, 0x32d86ce3,
        0x45df5c75, 0xdcd60dcf, 0xabd13d59, 0x26d930ac, 0x51de003a,
        0xc8d75180, 0xbfd06116, 0x21b4f4b5, 0x56b3c423, 0xcfba9599,
        0xb8bda50f, 0x2802b89e, 0x5f058808, 0xc60cd9b2, 0xb10be924,
        0x2f6f7c87, 0x58684c11, 0xc1611dab, 0xb6662d3d, 0x76dc4190,
        0x01db7106, 0x98d220bc, 0xefd5102a, 0x71b18589, 0x06b6b51f,
        0x9fbfe4a5, 0xe8b8d433, 0x7807c9a2, 0x0f00f934, 0x9609a88e,
        0xe10e9818, 0x7f6a0dbb, 0x086d3d2d, 0x91646c97, 0xe6635c01,
        0x6b6b51f4, 0x1c6c6162, 0x856530d8, 0xf262004e, 0x6c0695ed,
        0x1b01a57b, 0x8208f4c1, 0xf50fc457, 0x65b0d9c6, 0x12b7e950,
        0x8bbeb8ea, 0xfcb9887c, 0x62dd1ddf, 0x15da2d49, 0x8cd37cf3,
        0xfbd44c65, 0x4db26158, 0x3ab551ce, 0xa3bc0074, 0xd4bb30e2,
        0x4adfa541, 0x3dd895d7, 0xa4d1c46d, 0xd3d6f4fb, 0x4369e96a,
        0x346ed9fc, 0xad678846, 0xda60b8d0, 0x44042d73, 0x33031de5,
        0xaa0a4c5f, 0xdd0d7cc9, 0x5005713c, 0x270241aa, 0xbe0b1010,
        0xc90c2086, 0x5768b525, 0x206f85b3, 0xb966d409, 0xce61e49f,
        0x5edef90e, 0x29d9c998, 0xb0d09822, 0xc7d7a8b4, 0x59b33d17,
        0x2eb40d81, 0xb7bd5c3b, 0xc0ba6cad, 0xedb88320, 0x9abfb3b6,
        0x03b6e20c, 0x74b1d29a, 0xead54739, 0x9dd277af, 0x04db2615,
        0x73dc1683, 0xe3630b12, 0x94643b84, 0x0d6d6a3e, 0x7a6a5aa8,
        0xe40ecf0b, 0x9309ff9d, 0x0a00ae27, 0x7d079eb1, 0xf00f9344,
        0x8708a3d2, 0x1e01f268, 0x6906c2fe, 0xf762575d, 0x806567cb,
        0x196c3671, 0x6e6b06e7, 0xfed41b76, 0x89d32be0, 0x10da7a5a,
        0x67dd4acc, 0xf9b9df6f, 0x8ebeeff9, 0x17b7be43, 0x60b08ed5,
        0xd6d6a3e8, 0xa1d1937e, 0x38d8c2c4, 0x4fdff252, 0xd1bb67f1,
        0xa6bc5767, 0x3fb506dd, 0x48b2364b, 0xd80d2bda, 0xaf0a1b4c,
        0x36034af6, 0x41047a60, 0xdf60efc3, 0xa867df55, 0x316e8eef,
        0x4669be79, 0xcb61b38c, 0xbc66831a, 0x256fd2a0, 0x5268e236,
        0xcc0c7795, 0xbb0b4703, 0x220216b9, 0x5505262f, 0xc5ba3bbe,
        0xb2bd0b28, 0x2bb45a92, 0x5cb36a04, 0xc2d7ffa7, 0xb5d0cf31,
        0x2cd99e8b, 0x5bdeae1d, 0x9b64c2b0, 0xec63f226, 0x756aa39c,
        0x026d930a, 0x9c0906a9, 0xeb0e363f, 0x72076785, 0x05005713,
        0x95bf4a82, 0xe2b87a14, 0x7bb12bae, 0x0cb61b38, 0x92d28e9b,
        0xe5d5be0d, 0x7cdcefb7, 0x0bdbdf21, 0x86d3d2d4, 0xf1d4e242,
        0x68ddb3f8, 0x1fda836e, 0x81be16cd, 0xf6b9265b, 0x6fb077e1,
        0x18b74777, 0x88085ae6, 0xff0f6a70, 0x66063bca, 0x11010b5c,
        0x8f659eff, 0xf862ae69, 0x616bffd3, 0x166ccf45, 0xa00ae278,
        0xd70dd2ee, 0x4e048354, 0x3903b3c2, 0xa7672661, 0xd06016f7,
        0x4969474d, 0x3e6e77db, 0xaed16a4a, 0xd9d65adc, 0x40df0b66,
        0x37d83bf0, 0xa9bcae53, 0xdebb9ec5, 0x47b2cf7f, 0x30b5ffe9,
        0xbdbdf21c, 0xcabac28a, 0x53b39330, 0x24b4a3a6, 0xbad03605,
        0xcdd70693, 0x54de5729, 0x23d967bf, 0xb3667a2e, 0xc4614ab8,
        0x5d681b02, 0x2a6f2b94, 0xb40bbe37, 0xc30c8ea1, 0x5a05df1b,
        0x2d02ef8d
    ];

    fn new() -> CRC {
        CRC { c: 0xffffffffu32}
    }

    fn update(&mut self, buf: &[u8]) {
        for &b in buf {
            let k = (self.c ^ (b as u32)) & 0xff;
            self.c = CRC::CRC_TABLE[k as usize] ^ (self.c >> 8);
        }
    }

    fn value(&self) -> u32 {
        self.c ^ 0xffffffffu32
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;
    use std::path::PathBuf;
    use crate::png::Png;

    #[test]
    fn test_png_decoder() {
        let path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("test_resources")
            .join("image.png");
        assert!(path.exists());
        assert!(path.is_file());
        let mut file = std::fs::File::open(path).unwrap();
        let chunks = Png::decode(&mut file);
        assert!(chunks.is_ok());
        let chunks = chunks.unwrap();
        assert_eq!(chunks.len(), 15);
    }

    #[test]
    fn test_png_encoder() {
        let input_path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("test_resources")
            .join("image.png");

        // Read test image
        let chunks = {
            assert!(input_path.exists());
            let mut file = std::fs::File::open(&input_path).unwrap();
            Png::decode(&mut file).unwrap()
        };

        // Encode to another file
        let output_path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("test_resources")
            .join("image_copy.png");

        if output_path.exists() {
            std::fs::remove_file(&output_path).unwrap();
        }

        {
            let mut file = std::fs::File::create(&output_path).unwrap();
            Png::encode(&mut file, &chunks).unwrap();
        }

	{

	    let input_data = {
		let mut data: Vec<u8> = Default::default();
		let mut file = std::fs::File::open(input_path).unwrap();
		file.read_to_end(&mut data).unwrap();
		data
	    };

	    let output_data = {
		let mut data: Vec<u8> = Default::default();
		let mut file = std::fs::File::open(output_path).unwrap();
		file.read_to_end(&mut data).unwrap();
		data
	    };
	    assert_eq!(input_data, output_data);
	}
    }
}

