//! Pseudo-random numbers using Galois-field types as an LFSR
//!
//! It turns out Galois-fields can mathematically describe linear-feedback
//! shift-registers (LFSR), which makes some sense when you think of polynomial
//! division as the feedback from an LFSR propogating through the xor gates:
//!
//! (0xcc * 0x02) % 0x11d as an LFSR:
//!
//!     .- 1 <-- 1 <-- 0 <-- 0 <x- 1 <x- 1 <x- 0 <-- 0 <-.
//!     '-----------------------'-----'-----'------------'
//!
//!        1     0     0     0     0     1     0     1 = 0x85
//!
//! What if the right-hand side of our multiplication != 2? The implementation
//! gets more complicated, but we can still create an LFSR, and it has the
//! the same properties:
//!
//! (0xcc * 0x06) % 0x11d as an LFSR:
//!
//!    .-x- 1 <--x- 1 <--x- 0 <x-x- 0 <xxx- 1 <xxx- 1 <-xx- 0 <x--- 0 <-.
//!    | '--|----|--'    '--|--|-|--'  ||'--|--|||--'   |'--|--|----'   |
//!    |    |    '----------'  | '-----||---'  ||'------|---'  |        |
//!    '----|------------------|-------|'------|'-------'------|--------'
//!         '------------------'-------'-------'---------------'
//!
//!         1       0       0       1       0       0       1       0 = 0x92
//!
//! As you can see it's a little bit more complicated
//!
//! You may recall that every Galois-field contains one or more generators, g,
//! that define a multiplicative group containing all non-zero elements in the
//! field. In less wordy terms, if you repeatedly multiply g to any non-zero
//! number, you eventually get all non-zero numbers in the field. This is
//! equivalent to an LFSR that generates all non-zero elements before looping.
//!
//! As you can see from the diagrams above, it's much simpler to implement
//! an LFSR when g = 2, or in polynomial terms, g(x) = x. You will see an
//! irreducible polynomial where g = 2 called a "primitive polynomial".
//!
//! It's beneficial to use a primitive polynomial over a non-primitive
//! irreducible polynomial if you're implementing the LFSR in hardware, but
//! with carry-less multiplication we don't really care as long as g is
//! defined correctly.
//!
//! The end result is an LFSR implemented using a single Galoid-field
//! multiplication, or 3 carry-less multiplication instructions if hardware
//! is available. Giving us a very efficient, though predictable, pseudo-random
//! number generator.
//!
//! ---
//!
//! Note! The type of randomness generated by these Galois-field LFSRs
//! is equivalent to same-size Xorshift generators, with the same limitations
//! and period. However Xorshift generators are much more efficient, using
//! only a handful of shifts and xors. So there's no real reason to use
//! a Galois-field LFS over an Xorshift generator.
//!

use rand::SeedableRng;
use rand::RngCore;
use rand::Rng;
use std::cmp::max;
use std::convert::TryFrom;
use ::gf256::*;
use ::gf256::macros::*;


/// A pretty terrible prng, with a period of only 255
#[derive(Debug, Clone)]
struct Gf256Rng(gf256);

impl SeedableRng for Gf256Rng {
    type Seed = [u8; 1];

    fn from_seed(mut seed: Self::Seed) -> Self {
        // make sure seed does not equal zero! otherwise our rng would only
        // ever output zero!
        if seed.iter().all(|&x| x == 0) {
            seed = [1];
        }

        Gf256Rng(gf256::from_le_bytes(seed))
    }

    fn from_rng<R: RngCore>(mut rng: R) -> Result<Self, rand::Error> {
        let mut seed = [0; 1];
        while seed.iter().all(|&x| x == 0) {
            rng.try_fill_bytes(&mut seed)?;
        }

        Ok(Gf256Rng::from_seed(seed))
    }
}

impl RngCore for Gf256Rng {
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        for i in 0..dest.len() {
            self.0 *= gf256::GENERATOR;
            dest[i] = u8::from(self.0);
        }
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        Ok(self.fill_bytes(dest))
    }

    fn next_u32(&mut self) -> u32 {
        rand_core::impls::next_u32_via_fill(self)
    }

    fn next_u64(&mut self) -> u64 {
        rand_core::impls::next_u64_via_fill(self)
    }
}


/// Fortunately we can make Galois-fields larger than 256 elements
///
/// Note these constants were chosen to not be too sparse or too dense
/// as this avoids some common patterns. For example, generator=2 is just
/// a left-shift by 1 until it triggers a division by the irreducible
/// polynomial
///
#[gf(polynomial=0x123456789abcdef6b, generator=0x123456789abcdef3)]
type gf2p64;

/// A better prng, with a period of 2^64, comparable to xorshift64
///
#[derive(Debug, Clone)]
struct Gf2p64Rng(gf2p64);

impl SeedableRng for Gf2p64Rng {
    type Seed = [u8; 8];

    fn from_seed(mut seed: Self::Seed) -> Self {
        // make sure seed does not equal zero! otherwise our rng would only
        // ever output zero!
        if seed.iter().all(|&x| x == 0) {
            seed = [1,2,3,4,5,6,7,8];
        }

        Gf2p64Rng(gf2p64::from_le_bytes(seed))
    }

    fn from_rng<R: RngCore>(mut rng: R) -> Result<Self, rand::Error> {
        let mut seed = [0; 8];
        while seed.iter().all(|&x| x == 0) {
            rng.try_fill_bytes(&mut seed)?;
        }

        Ok(Gf2p64Rng::from_seed(seed))
    }
}

impl RngCore for Gf2p64Rng {
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        rand_core::impls::fill_bytes_via_next(self, dest)
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        Ok(self.fill_bytes(dest))
    }

    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn next_u64(&mut self) -> u64 {
        self.0 *= gf2p64::GENERATOR;
        u64::from(self.0)
    }
}


fn main() {
    fn hex(xs: &[u8]) -> String {
        xs.iter()
            .map(|x| format!("{:02x}", x))
            .collect()
    }

    fn grid<'a>(width: usize, bs: &'a [u8]) -> impl Iterator<Item=String> + 'a {
        fn braille(b: u8) -> char {
            let b = '⠀' as u32
                + ((b as u32) & 0x87)
                + (((b as u32) & 0x08) << 3)
                + (((b as u32) & 0x70) >> 1);
            char::from_u32(b).unwrap()
        }

        (0 .. (bs.len()+width-1)/width)
            .step_by(4)
            .rev()
            .map(move |y| {
                let mut line = String::new();
                for x in (0..width).step_by(2) {
                    let mut b = 0;
                    for i in 0..4 {
                        for j in 0..2 {
                            if bs.get((y+i)*width + x+j).filter(|b| **b != 0).is_some() {
                                b |= 1 << (j*4+(3-i));
                            }
                        }
                    }
                    line.push(braille(b));
                }
                line
            })
    }

    println!();
    let mut gf256rng = Gf256Rng::from_seed([1]);
    let mut gf2p64rng = Gf2p64Rng::from_seed([1,2,3,4,5,6,7,8]);

    let mut buffer = [0u8; 32];
    gf256rng.fill_bytes(&mut buffer);
    println!("{:<9} => {}", "gf256rng", hex(&buffer));

    let mut buffer = [0u8; 32];
    gf2p64rng.fill_bytes(&mut buffer);
    println!("{:<9} => {}", "gf2p64rng", hex(&buffer));
    println!();


    // Uniform distributions are boring, lets show a rough triangle
    // distribution distribution, X = Y+Z where Y and Z are uniform (our prngs)

    const SAMPLES: usize = 2048;
    const WIDTH: usize = 128;
    const HEIGHT: usize = 64;

    println!("{} gf256rng samples:", SAMPLES);
    let mut buffer = [0u8; WIDTH*HEIGHT];
    for _ in 0..SAMPLES {
        let x = gf256rng.gen_range(0..WIDTH/2) + gf256rng.gen_range(0..WIDTH/2);
        let y = gf256rng.gen_range(0..HEIGHT/2) + gf256rng.gen_range(0..HEIGHT/2);
        buffer[x+y*WIDTH] += 1;
    }

    let mut x_dist = [0u8; 4*WIDTH];
    let mut x_max = 0;
    for x in 0..WIDTH {
        x_max = max(x_max, (0..HEIGHT).map(|y| u32::from(buffer[x+y*WIDTH])).sum());
    }
    for x in 0..WIDTH {
        let v: u32 = (0..HEIGHT).map(|y| u32::from(buffer[x+y*WIDTH])).sum();
        let v = (4*v+x_max-1) / x_max;
        for i in 0..usize::try_from(v).unwrap() {
            x_dist[x+i*WIDTH] = 1;
        }
    }

    let mut y_dist = [0u8; 4*HEIGHT];
    let mut y_max = 0;
    for y in 0..HEIGHT {
        y_max = max(y_max, (0..WIDTH).map(|x| u32::from(buffer[x+y*WIDTH])).sum());
    }
    for y in 0..HEIGHT {
        let v: u32 = (0..WIDTH).map(|x| u32::from(buffer[x+y*WIDTH])).sum();
        let v = (4*v+y_max-1) / y_max;
        for i in 0..usize::try_from(v).unwrap() {
            y_dist[(3-i)+y*4] = 1;
        }
    }

    for (line, y_dist_line) in grid(128, &buffer).zip(grid(4, &y_dist)) {
        println!("    {}  {}", line, y_dist_line);
    }
    println!();

    for x_dist_line in grid(128, &x_dist) {
        println!("    {}", x_dist_line);
    }
    println!();

    println!("{} gf2p64rng samples:", SAMPLES);
    let mut buffer = [0u8; WIDTH*HEIGHT];
    for _ in 0..SAMPLES {
        let x = gf2p64rng.gen_range(0..WIDTH/2) + gf2p64rng.gen_range(0..WIDTH/2);
        let y = gf2p64rng.gen_range(0..HEIGHT/2) + gf2p64rng.gen_range(0..HEIGHT/2);
        buffer[x+y*WIDTH] += 1;
    }

    let mut x_dist = [0u8; 4*WIDTH];
    let mut x_max = 0;
    for x in 0..WIDTH {
        x_max = max(x_max, (0..HEIGHT).map(|y| u32::from(buffer[x+y*WIDTH])).sum());
    }
    for x in 0..WIDTH {
        let v: u32 = (0..HEIGHT).map(|y| u32::from(buffer[x+y*WIDTH])).sum();
        let v = (4*v+x_max-1) / x_max;
        for i in 0..usize::try_from(v).unwrap() {
            x_dist[x+i*WIDTH] = 1;
        }
    }

    let mut y_dist = [0u8; 4*HEIGHT];
    let mut y_max = 0;
    for y in 0..HEIGHT {
        y_max = max(y_max, (0..WIDTH).map(|x| u32::from(buffer[x+y*WIDTH])).sum());
    }
    for y in 0..HEIGHT {
        let v: u32 = (0..WIDTH).map(|x| u32::from(buffer[x+y*WIDTH])).sum();
        let v = (4*v+y_max-1) / y_max;
        for i in 0..usize::try_from(v).unwrap() {
            y_dist[(3-i)+y*4] = 1;
        }
    }

    for (line, y_dist_line) in grid(128, &buffer).zip(grid(4, &y_dist)) {
        println!("    {}  {}", line, y_dist_line);
    }
    println!();

    for x_dist_line in grid(128, &x_dist) {
        println!("    {}", x_dist_line);
    }
    println!();
}
