#[derive(Debug, Clone, Copy, PartialEq, Default)]

struct Celsius {
    deg: f64,
}
impl Celsius {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(8);
        __bytes.extend_from_slice(&self.deg.to_ne_bytes());
        __bytes
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Default)]

struct Fahrenheit {
    deg: f64,
}
impl Fahrenheit {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(8);
        __bytes.extend_from_slice(&self.deg.to_ne_bytes());
        __bytes
    }
}


impl From<Celsius> for Fahrenheit {
#[inline]
fn from(c: Celsius) -> Fahrenheit {
        Fahrenheit { deg: c.deg * 9.0_f64 / 5.0_f64 + 32.0_f64 }
}
}

fn main() {
    let f = Fahrenheit::from(Celsius { deg: 37.0_f64 });
    println!("{:.1}", f.deg);
    let n: i64 = 37_i64;
    let f2: f64 = n as f64;
    println!("{:.0}", f2);
}

