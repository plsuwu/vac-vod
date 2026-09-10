use std::collections::HashMap;
use std::fs;
use std::io;

struct TensorMeta {
    dtype: String,
    shape: Vec<usize>,
    start: usize,
    end: usize,
}

pub struct SafeTensors {
    data: Vec<u8>,
    meta: HashMap<String, TensorMeta>,
}

impl SafeTensors {
    pub fn load(path: &str) -> io::Result<Self> {
        let data = fs::read(path)?;
        let n = u64::from_le_bytes(data[..8].try_into().unwrap()) as usize;
        let header: serde_json::Value = serde_json::from_slice(&data[8..8 + n])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let base = 8 + n;

        let mut meta = HashMap::new();
        for (name, v) in header.as_object().unwrap() {
            if name == "__metadata__" {
                continue;
            }

            let offs = v["data_offsets"].as_array().unwrap();
            meta.insert(
                name.clone(),
                TensorMeta {
                    dtype: v["dtype"].as_str().unwrap().to_string(),
                    shape: v["shape"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|x| x.as_u64().unwrap() as usize)
                        .collect(),
                    start: base + offs[0].as_u64().unwrap() as usize,
                    end: base + offs[1].as_u64().unwrap() as usize,
                },
            );
        }

        Ok(Self { data, meta })
    }

    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.meta.keys()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.meta.contains_key(name)
    }

    pub fn tensor(&self, name: &str) -> (Vec<usize>, Vec<f32>) {
        let m = self
            .meta
            .get(name)
            .unwrap_or_else(|| panic!("missing tensor '{name}'"));

        let bytes = &self.data[m.start..m.end];
        let vals: Vec<f32> = match m.dtype.as_str() {
            "F32" => bytes
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
                .collect(),
            "F16" => bytes
                .chunks_exact(2)
                .map(|b| f16_to_f32(u16::from_le_bytes([b[0], b[1]])))
                .collect(),
            "BF16" => bytes
                .chunks_exact(2)
                .map(|b| f32::from_bits((u16::from_le_bytes([b[0], b[1]]) as u32) << 16))
                .collect(),
            d => panic!("unsupported dtype {d} for '{name}'"),
        };
        (m.shape.clone(), vals)
    }
}

fn f16_to_f32(h: u16) -> f32 {
    let sign = ((h >> 15) & 1) as u32;
    let exp = ((h >> 10) & 0x1f) as u32;
    let mant = (h & 0x3ff) as u32;
    let bits = match exp {
        0 => {
            if mant == 0 {
                sign << 31
            } else {
                let mut e: i32 = 127 - 15 + 1;
                let mut m = mant;
                while m & 0x400 == 0 {
                    m <<= 1;
                    e -= 1;
                }
                (sign << 31) | ((e as u32) << 23) | ((m & 0x3ff) << 13)
            }
        }
        0x1f => (sign << 31) | 0x7f80_0000 | (mant << 13),
        _ => (sign << 31) | ((exp + 127 - 15) << 23) | (mant << 13),
    };

    f32::from_bits(bits)
}
