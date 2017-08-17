extern crate synth;

use synth::util::nmat::*;

#[inline(never)]
fn make_big_matrix<O: Ordering>() -> Matrix<f32, O>
{
    // I have an 8 meg cache, 512x512 is larger than the entire cache
    let mut m = Matrix::<f32, O>::new((512, 512));
    for i in 0..m.dim().0 {
        for j in 0..m.dim().1 {
            m[(i,j)] = (i + j) as f32;
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

    println!("m3 = {}", m3[(123,456)]);
}
