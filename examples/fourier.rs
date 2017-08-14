extern crate synth;

use synth::util::ft;
use synth::util::nmat::*;

fn main()
{
    let xs: [f32; 4] = [0.0, 1.0, 0.0, -1.0];
    let mut inpt: Matrix<_, RowMajor> = Matrix::new((xs.len(), 1));

    for (i, item) in xs.iter().enumerate() {
        inpt[(i, 0)] = item.clone();
    }

    let out = ft::reference_fourier(&inpt);

    let (x, y) = out.dim();
    for i in 0..x {
        for j in 0..y {
            println!("out[{}, {}] = {}", i, j, out[(i,j)].norm())
        }
    }
}
