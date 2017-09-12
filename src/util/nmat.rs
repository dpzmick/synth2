use std::marker::PhantomData;
use std::ops::{Index, IndexMut, Mul, Add};
use std::fmt;
use std::fmt::Debug;

use util::vector::Vectorizable;

type Dimension = (usize, usize);
type Coordinate = (usize, usize);

pub trait Ordering {
    fn idx(dimensions: Dimension, coord: Coordinate) -> usize;

    // for Debug
    fn human_name() -> String;
}

pub struct RowMajor {}
impl Ordering for RowMajor {
    #[inline]
    fn idx((_, column_dimension): Dimension, (x, y): Coordinate) -> usize
    {
        y + x*column_dimension
    }

    fn human_name() -> String { "RowMajor".to_owned() }
}

pub struct ColumnMajor {}
impl Ordering for ColumnMajor {
    #[inline]
    fn idx((row_dimension, _): Dimension, (x, y): Coordinate) -> usize
    {
        x + y*row_dimension
    }

    fn human_name() -> String { "ColumnMajor".to_owned() }
}

// NxM matrix
pub struct Matrix<T, O: Ordering> {
    dim: Dimension,
    values: Vec<T>,

    // needed so that we can add type bound to struct
    ordering: PhantomData<O>
}

impl<T: Default + Clone> Matrix<T, RowMajor> {
    pub fn new_row_major(dim: Dimension) -> Self
    {
        Matrix::<T, RowMajor>::new(dim)
    }
}

impl<T: Default + Clone> Matrix<T, ColumnMajor> {
    pub fn new_column_major(dim: Dimension) -> Self
    {
        Matrix::<T, ColumnMajor>::new(dim)
    }
}

impl<T: Default + Clone, O: Ordering> Matrix<T, O> {
    pub fn new(dim: Dimension) -> Self
    {
        Self {
            dim,
            values: vec![Default::default(); dim.0*dim.1],
            ordering: PhantomData,
        }
    }
}

impl<T: Clone, O: Ordering> Matrix<T, O> {
    pub fn new_with_default(dim: Dimension, default: T) -> Self
    {
        Self {
            dim,
            values: vec![default; dim.0*dim.1],
            ordering: PhantomData,
        }
    }
}

impl<T, O: Ordering> Matrix<T, O> {
    pub fn dim(&self) -> (usize, usize)
    {
        self.dim
    }

    pub unsafe fn get_unchecked(&self, c: Coordinate) -> &T
    {
        let idx = O::idx(self.dim(), c);
        self.values.get_unchecked(idx)
    }

    pub unsafe fn get_unchecked_mut(&mut self, c: Coordinate) -> &mut T
    {
        let idx = O::idx(self.dim(), c);
        self.values.get_unchecked_mut(idx)
    }
}

impl<T: Vectorizable> Matrix<T, RowMajor> {
    fn vector_mult(&self, rhs: &Matrix<T, ColumnMajor>) -> Matrix<T, RowMajor>
    {
        let (n, m1) = self.dim();
        let (m2, p) = rhs.dim();
        assert!(m1 == m2);

        //println!("doing it the {}x vector way", T::vector_size());

        let mut output: Matrix<T, RowMajor> = Matrix::new((n, p));

        for i in 0..n {
            for j in 0..p {
                let end = if m1 > T::vector_size() { m1 } else { 0 };
                for k in (0..end).step_by(T::vector_size()) {
                    unsafe {
                        let curr = output.get_unchecked((i,j)).clone();

                        let vector1 = T::load(
                            &self.values, RowMajor::idx(output.dim(), (i,k)));

                        let vector2 = T::load(
                            &rhs.values, ColumnMajor::idx(output.dim(), (k,j)));

                        let products = vector1 * vector2;
                        let mut sum = T::default();

                        // TODO there should be an intrinsic for this
                        for i in 0..T::vector_size() {
                            // don't use AddAssign, adds another trait bound
                            sum = sum + T::extract(&products, i as u32);
                        }

                        *output.get_unchecked_mut((i,j)) = curr + sum;
                    }
                }

                // fill in whatever doesn't fit in the vector
                let end = (0..m1).step_by(T::vector_size()).last().unwrap_or(0);
                for k in end..m1 {
                    unsafe {
                        let curr = output.get_unchecked((i,j)).clone();
                        let product = self.get_unchecked((i,k)).clone()
                            * rhs.get_unchecked((k,j)).clone();
                        let sum = curr + product;
                        *output.get_unchecked_mut((i, j)) = sum;
                    }
                }
            }
        }

        output
    }
}

// TODO make mat[][] work?
// TODO assert/panic range check against dim(), don't depend on vector being the
// right size
impl<T, O: Ordering> Index<Coordinate> for Matrix<T, O> {
    type Output = T;

    fn index(&self, index: Coordinate) -> &Self::Output
    {
        let idx = O::idx(self.dim(), index);
        &self.values[idx]
    }
}

impl<T, O: Ordering> IndexMut<Coordinate> for Matrix<T, O> {
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output
    {
        let idx = O::idx(self.dim(), index);
        &mut self.values[idx]
    }
}

impl<T: Debug, O: Ordering> Debug for Matrix<T, O>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error>
    {
        write!(f, "Matrix<{}> {{ [ ", O::human_name())?;

        // always prints in RowMajor ordering
        for i in 0..self.dim().0 {
            write!(f, "[")?;
            for j in 0..self.dim().1 {
                write!(f, "{:?}", self[(i,j)])?;
                if j != self.dim().1 - 1 {
                    write!(f, ", ")?;
                }
            }
            write!(f, "]")?;

            if i != self.dim().0 - 1 {
                write!(f, ", ")?;
            }
        }

        write!(f, " ] }}")?;

        Ok(())
    }
}

impl<T1, T2, O1: Ordering, O2: Ordering>
    PartialEq<Matrix<T2, O2>> for Matrix<T1, O1>
where
    T1: PartialEq<T2>,
    // no bounds needed on T2
{
    default fn eq(&self, other: &Matrix<T2, O2>) -> bool
    {
        if self.dim() != other.dim() { return false; }

        // TODO iterators!
        let (r, c) = self.dim();

        for i in 0..r {
            for j in 0..c {
                if self[(i,j)] != other[(i,j)] { return false; }
            }
        }

        return true;
    }
}

// specialized PartialEq for ColumMajor vs ColumMajor
// requires nightly only #[feature(specialization)]
// else fallback to the default implementation (row major)
impl <T1, T2> PartialEq<Matrix<T2, ColumnMajor>> for Matrix<T1, ColumnMajor>
where T1: PartialEq<T2>
{
    fn eq(&self, other: &Matrix<T2, ColumnMajor>) -> bool
    {
        //println!("used the specialization");
        if self.dim() != other.dim() { return false; }

        let (r, c) = self.dim();

        // go in column major order instead
        for j in 0..c {
            for i in 0..r {
                if self[(i,j)] != other[(i,j)] { return false; }
            }
        }

        return true;
    }
}

// TODO vectorized eq?
// TODO eq method for doubles that includes some epsilon?
// TODO iterators might save the day on some of these funkier methods
// TODO faster multiplies for other ordering pairs

impl<'a, 'b, T1, T2, O1: Ordering, O2: Ordering>
    Mul<&'b Matrix<T2, O2>> for &'a Matrix<T1, O1>
where
    T1: Mul<T2> + Clone,
    T2: Clone,
    <T1 as Mul<T2>>::Output: Add<Output = <T1 as Mul<T2>>::Output> + Clone + Default,
{
    // always output row major because we compute in row major order
    type Output = Matrix<
        <T1 as Mul<T2>>::Output
        , RowMajor>;

    // self is a &'a
    default fn mul(self, rhs: &'b Matrix<T2, O2>) -> Self::Output
    {
        let (n, m1) = self.dim();
        let (m2, p) = rhs.dim();

        // TODO panic better
        assert!(m1 == m2);

        // TODO evaluate removing the Default requirement
        let mut output: Self::Output = Matrix::new((n, p));

        // TODO iterators?
        for i in 0..n {
            for j in 0..p {
                for k in 0..m1 {
                    // don't use AddAssign, not all types implement it
                    // use the get_unchecked to ensure that there are no bounds
                    // checks performed. This does appear to actually make a
                    // noticeable difference

                    unsafe {
                        let curr = output.get_unchecked((i,j)).clone();

                        let product =
                            self.get_unchecked((i,k)).clone()
                            * rhs.get_unchecked((k,j)).clone();

                        let sum = curr + product;

                        *output.get_unchecked_mut((i,j)) = sum;
                    }
                }
            }
        }

        output
    }
}

impl<'a, 'b, T: Vectorizable> Mul<&'b Matrix<T, ColumnMajor>> for &'a Matrix<T, RowMajor>
{
    fn mul(self, rhs: &'b Matrix<T, ColumnMajor>) -> Self::Output
    {
        self.vector_mult(rhs)
    }
}

impl<'a, T1, T2, O1: Ordering, O2: Ordering>
    Mul<&'a Matrix<T2, O2>> for Matrix<T1, O1>
where
    T1: Mul<T2> + Clone,
    T2: Clone,
    <T1 as Mul<T2>>::Output: Add<Output = <T1 as Mul<T2>>::Output> + Clone + Default,
{
    type Output = Matrix<
        <T1 as Mul<T2>>::Output
        , RowMajor>;

    fn mul(self, rhs: &'a Matrix<T2, O2>) -> Self::Output
    {
        &self * rhs
    }
}

impl<T1, T2, O1: Ordering, O2: Ordering> Mul<Matrix<T2, O2>> for Matrix<T1, O1>
where
    T1: Mul<T2> + Clone,
    T2: Clone,
    <T1 as Mul<T2>>::Output: Add<Output = <T1 as Mul<T2>>::Output> + Clone + Default,
{
    type Output = Matrix<
        <T1 as Mul<T2>>::Output
        , RowMajor>;

    fn mul(self, rhs: Matrix<T2, O2>) -> Self::Output
    {
        &self * &rhs
    }
}

// TODO move all of the logic out of the trait impls, then just call the better
// named functions (eg. naive_multiply, vector_eq, naive_eq, etc)

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert;

    pub fn make_big_matrix<T: convert::From<u16> + Default + Clone, O: Ordering>() -> Matrix<T, O>
    {
        // I have an 8 meg cache, 512x512 is larger than the entire cache
        let mut m = Matrix::<T, O>::new((512, 512));
        for i in 0..m.dim().0 {
            for j in 0..m.dim().1 {
                // largest value is m.dim().0 + m.dim().1 (512 + 512) = 1024
                // need 10 bits
                m[(i,j)] = ((i + j) as u16).into();
            }
        }

        m
    }

    pub fn test_simple_matrix_impl<O: Ordering>()
    {
        let mut mat: Matrix<i64, O> = Matrix::new((4, 4));
        mat[(0,0)] = 1;

        assert!(mat[(0,0)] == 1);
        for i in 0..4 {
            for j in 0..4 {
                if i == 0 && j == 0 { continue; }
                assert!(mat[(i, j)] == 0);
            }
        }
    }

    #[test]
    fn test_simple_matrix()
    {
        test_simple_matrix_impl::<RowMajor>();
        test_simple_matrix_impl::<ColumnMajor>();
    }

    pub fn test_eq_impl<O1: Ordering, O2: Ordering>()
    {
        let mut m1: Matrix<i64, O1> = Matrix::new((2, 2));
        let mut m2: Matrix<i64, O2> = Matrix::new((2, 2));

        for i in 0..2 {
            for j in 0..2 {
                m1[(i,j)] = 2;
                m2[(i,j)] = 2;
            }
        }

        assert_eq!(m1, m2);
    }

    #[test]
    fn test_eq()
    {
        test_eq_impl::<RowMajor,    RowMajor>();
        test_eq_impl::<RowMajor,    ColumnMajor>();
        test_eq_impl::<ColumnMajor, RowMajor>();
        test_eq_impl::<ColumnMajor, ColumnMajor>();
    }

    pub fn test_square_mul_impl<O1: Ordering, O2: Ordering>()
    {
        let mut m1: Matrix<i64, O1> = Matrix::new((2, 2));
        let mut m2: Matrix<i64, O2> = Matrix::new((2, 2));

        for i in 0..2 {
            for j in 0..2 {
                m1[(i,j)] = 2;
                m2[(i,j)] = (i + j) as i64;
            }
        }

        let m3 = m1 * m2;

        assert_eq!(m3.dim(), (2,2));

        let mut expected: Matrix<_, RowMajor> = Matrix::new((2, 2));
        expected[(0,0)] = 2;
        expected[(0,1)] = 6;
        expected[(1,0)] = 2;
        expected[(1,1)] = 6;

        assert_eq!(m3, expected);
    }

    #[test]
    fn test_square_mul()
    {
        test_square_mul_impl::<RowMajor,    RowMajor>();
        test_square_mul_impl::<RowMajor,    ColumnMajor>();
        test_square_mul_impl::<ColumnMajor, RowMajor>();
        test_square_mul_impl::<ColumnMajor, ColumnMajor>();
    }

    pub fn test_vector_mul1_impl<O1: Ordering, O2: Ordering>()
    {
        let (a, b) = (2, 1);
        let (x, y) = (0, 1);

        let mut m1: Matrix<i64, O1> = Matrix::new((1, 2));
        let mut m2: Matrix<i64, O2> = Matrix::new((2, 1));

        m1[(0,0)] = a;
        m1[(0,1)] = b;

        m2[(0,0)] = x;
        m2[(1,0)] = y;

        let m3 = m1 * m2;

        assert_eq!(m3.dim(), (1,1));
        assert_eq!(m3[(0, 0)], a*x + b*y);
    }

    #[test]
    fn test_vector_mut1()
    {
        test_vector_mul1_impl::<RowMajor,    RowMajor>();
        test_vector_mul1_impl::<RowMajor,    ColumnMajor>();
        test_vector_mul1_impl::<ColumnMajor, RowMajor>();
        test_vector_mul1_impl::<ColumnMajor, ColumnMajor>();
    }

    pub fn test_vector_mul2_impl<O1: Ordering, O2: Ordering>()
    {
        let (a, b) = (2, 1);
        let (x, y) = (0, 1);

        let mut m1: Matrix<i64, O1> = Matrix::new((2, 1));
        let mut m2: Matrix<i64, O2> = Matrix::new((1, 2));

        m1[(0,0)] = a;
        m1[(1,0)] = b;

        m2[(0,0)] = x;
        m2[(0,1)] = y;

        let m3 = m1 * m2;

        assert_eq!(m3.dim(), (2, 2));

        let mut expected = Matrix::new_row_major(m3.dim());
        expected[(0,0)] = a*x;
        expected[(0,1)] = a*y;
        expected[(1,0)] = b*x;
        expected[(1,1)] = b*y;

        assert_eq!(m3, expected);
    }

    #[test]
    fn test_vector_mut2()
    {
        test_vector_mul2_impl::<RowMajor,    RowMajor>();
        test_vector_mul2_impl::<RowMajor,    ColumnMajor>();
        test_vector_mul2_impl::<ColumnMajor, RowMajor>();
        test_vector_mul2_impl::<ColumnMajor, ColumnMajor>();
    }

    pub fn add_all_rm<O: Ordering>(m: &Matrix<i64, O>) -> i64
    {
        // iterate in RowMajor order
        let mut sum = 0;
        for i in 0..m.dim().0 {
            for j in 0..m.dim().1 {
                sum += m[(i,j)];
            }
        }

        sum
    }

    pub fn add_all_cm<O: Ordering>(m: &Matrix<i64, O>) -> i64
    {
        // iterate in ColumnMajor order
        let mut sum = 0;
        for j in 0..m.dim().1 {
            for i in 0..m.dim().0 {
                sum += m[(i,j)];
            }
        }

        sum
    }

}

#[cfg(all(feature = "benchmarks", test))]
mod benchmarks {
    use super::*;
    use super::tests::*;

    use test;
    use test::Bencher;
    use std::convert;

    use util::vector::FakeValue;

    #[bench]
    fn fast_add_all1(bench: &mut Bencher) -> ()
    {
        let m = make_big_matrix::<i64, RowMajor>();
        bench.iter(|| test::black_box(add_all_rm(&m)));
    }

    #[bench]
    fn fast_add_all2(bench: &mut Bencher) -> ()
    {
        let m = make_big_matrix::<i64, ColumnMajor>();
        bench.iter(|| test::black_box(add_all_cm(&m)));
    }

    #[bench]
    fn slow_add_all1(bench: &mut Bencher) -> ()
    {
        let m = make_big_matrix::<i64, RowMajor>();
        bench.iter(|| test::black_box(add_all_cm(&m)));
    }

    #[bench]
    fn slow_add_all2(bench: &mut Bencher) -> ()
    {
        let m = make_big_matrix::<i64, ColumnMajor>();
        bench.iter(|| test::black_box(add_all_rm(&m)));
    }

    // this bench should always run slower than fast_multiply
    #[bench]
    fn slow_multiply(bench: &mut Bencher) -> ()
    {
        let a = make_big_matrix::<i64, ColumnMajor>();
        let b = make_big_matrix::<i64, RowMajor>();

        bench.iter(|| &a * &b);
    }

    #[bench]
    fn fast_multiply(bench: &mut Bencher) -> ()
    {
        let a = make_big_matrix::<i64, RowMajor>();
        let b = make_big_matrix::<i64, ColumnMajor>();

        bench.iter(|| &a * &b);
    }

    #[bench]
    fn check_slowfast(bencher: &mut Bencher) -> ()
    {
        // // doesn't actually run any benchmarks, but we mark this as a benchmark so that it is run
        // // with optimized build
        // use std::time::Instant;

        // fn timeit<F: FnMut()>(mut f: F) -> f64 {
        //     let mut times = [0.0; 50];
        //     for t in times.iter_mut() {
        //         let now = Instant::now();
        //         f();
        //         let e = now.elapsed();
        //         *t = e.as_secs() as f64 + e.subsec_nanos() as f64 * 1e-9;
        //     }

        //     let l = times.len() as f64;
        //     times.iter().fold(0.0, |sum, val| sum + val) / l
        // }

        // let slow = timeit(|| {
        //     let a = make_big_matrix::<i64, ColumnMajor>();
        //     let b = make_big_matrix::<i64, RowMajor>();

        //     test::black_box(&a * &b);
        // });

        // let fast = timeit(|| {
        //     let a = make_big_matrix::<i64, RowMajor>();
        //     let b = make_big_matrix::<i64, ColumnMajor>();

        //     test::black_box(&a * &b);
        // });

        use test::bench;
        let bs = bench::benchmark(|b: &mut Bencher| b.iter(|| {
            let a = make_big_matrix::<i64, RowMajor>();
            let b = make_big_matrix::<i64, ColumnMajor>();

            test::black_box(&a * &b);
        }));

        use test::stats;
        struct BenchSamplesHack {
            ns_iter_summ: stats::Summary,
            mb_s: usize,
        }

        use std::mem;
        let tricked_ya: BenchSamplesHack = unsafe { mem::transmute(bs) };

        println!("{:?}", tricked_ya.ns_iter_summ.median);

        // assert!(slow > 3.0 * fast);
        // TODO there has to be a better way to express these performance tests
        // - use a different benchmarks lib perhaps. This one doesn't give
        //   nearly enough insight into what's going on
        // TODO write a better multiply so this test is no longer true
    }

    fn vector_multiply_impl<T>(bench: &mut Bencher) -> ()
        where T: Mul<Output=T> + Add<Output=T> + convert::From<u16> + Default + Clone
    {
        let a = make_big_matrix::<T, RowMajor>();
        let b = make_big_matrix::<T, ColumnMajor>();

        bench.iter(|| &a * &b);
    }

    // lets us test the performance of the unvectorized versions of the same
    // types we tested w/ vectorization enabled
    fn unvector_multiply_impl<T>(bench: &mut Bencher) -> ()
        where T: Mul<Output=T> + Add<Output=T> + convert::From<u16> + Default + Clone
    {
        let a = make_big_matrix::<FakeValue<T>, RowMajor>();
        let b = make_big_matrix::<FakeValue<T>, ColumnMajor>();

        bench.iter(|| &a * &b);
    }

    // can't generate function names in a macro...........
    macro_rules! vector_mult_bench {
        ($n:ident, $t:ty) => {
            #[bench]
            fn $n(bench: &mut Bencher) -> ()
            {
                vector_multiply_impl::<$t>(bench)
            }

        }
    }

    macro_rules! unvector_mult_bench {
        ($n:ident, $t:ty) => {
            #[bench]
            fn $n(bench: &mut Bencher) -> ()
            {
                unvector_multiply_impl::<$t>(bench)
            }

        }
    }

    vector_mult_bench!(vmulf32, f32);
    vector_mult_bench!(vmuli32, i32);
    vector_mult_bench!(vmulf64, f64);
    vector_mult_bench!(vmuli64, i64);

    unvector_mult_bench!(vmulf32_unvec, f32);
    unvector_mult_bench!(vmuli32_unvec, i32);
    unvector_mult_bench!(vmulf64_unvec, f64);
    unvector_mult_bench!(vmuli64_unvec, i64);
}

// TODO implement iterators
