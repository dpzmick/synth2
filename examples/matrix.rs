extern crate synth;

use synth::util::nmat::*;
use synth::util::vector::FakeValue;

const SIZE: usize = 512;

type MT = FakeValue<i64>;

#[inline(never)]
fn make_big_matrix<O: Ordering>() -> Matrix<MT, O>
{
    // I have an 8 meg cache, 512x512 is larger than the entire cache
    let mut m = Matrix::<MT, O>::new((SIZE, SIZE));
    for i in 0..m.dim().0 {
        for j in 0..m.dim().1 {
            m[(i,j)] = ((i + j) as u16).into();
        }
    }

    m
}

fn main()
{
    // test to see if this gets vectorized
    let m1 = make_big_matrix::<RowMajor>();
    let m2 = make_big_matrix::<ColumnMajor>();

    let m3 = m1 * m2;
    println!("m3[0,0] = {:?}", m3[(0,0)]);
}
