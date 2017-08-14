// Fourier Transforms

use num::{Zero, One};
use num::complex::Complex;

use util::nmat::*;

fn w(n: i64, exp: i64) -> Complex<f32>
{
    assert!(n > 0);

    use std::f32::consts::PI;
    let mut c = Complex::<f32>::one();

    // TODO wtf
    for _i in 0..exp {
        let mut tmp = Complex::<f32>::zero();

        let tpn = (PI * 2.0) / (n as f32);
        tmp.re = tpn.cos();
        tmp.im = -1.0 * tpn.sin();

        c = c * tmp; // TODO implement *= for complex in Num (any maybe pow?)
    }

    c
}

pub fn F<O: Ordering>(n: usize) -> Matrix<Complex<f32>, O> {
    let mut mat = Matrix::new_with_default((n, n), Complex::<f32>::zero());

    for r in 0..n {
        for c in 0..n {
            mat[(r,c)] = w(n as i64, (r*c) as i64);
        }
    }

    mat
}

pub fn reference_fourier<O: Ordering>(x: &Matrix<f32, O>)
    -> Matrix<Complex<f32>, RowMajor>
{
    let (n, m) = x.dim();
    assert!(m == 1);

    F::<RowMajor>(n) * x
}
