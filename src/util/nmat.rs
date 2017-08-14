use std::marker::PhantomData;
use std::ops::{Index, IndexMut, Mul, Add};
use std::fmt;
use std::fmt::Debug;

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

impl<T, O: Ordering> Matrix<T, O> {
    pub fn dim(&self) -> (usize, usize) {
        self.dim
    }
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
    fn eq(&self, other: &Matrix<T2, O2>) -> bool
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
    fn mul(self, rhs: &'b Matrix<T2, O2>) -> Self::Output
    {
        let (n, m1) = self.dim();
        let (m2, p) = rhs.dim();

        // can't put dimension in the type without the ability to perform
        // arbitrary arithmetic on it. As we can see, generics are already much
        // too painful
        assert!(m1 == m2);

        // TODO evaluate removing the Default requirement
        let mut output: Self::Output = Matrix::new((n, p));

        // TODO iterators?
        for i in 0..n {
            for j in 0..p {
                for k in 0..m1 {
                    // don't use AddAssign, not all types implement it
                    output[(i,j)] = output[(i,j)].clone()
                        + (self[(i,k)].clone() * rhs[(k,j)].clone())
                }
            }
        }

        output
    }
}

// TODO find a way to reduce all of the types needed in the overrides

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

#[cfg(test)]
mod test {
    use super::*;
    use test;
    use test::Bencher;

    fn make_big_matrix<O: Ordering>() -> Matrix<i64, O>
    {
        // I have an 8 meg cache, 512x512 is larger than the entire cache
        let mut m = Matrix::<i64, O>::new((512, 512));
        for i in 0..m.dim().0 {
            for j in 0..m.dim().1 {
                m[(i,j)] = (i + j) as i64;
            }
        }

        m
    }

    fn test_simple_matrix_impl<O: Ordering>()
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

    fn test_eq_impl<O1: Ordering, O2: Ordering>()
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

    fn test_square_mul_impl<O1: Ordering, O2: Ordering>()
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

        assert_eq!(m3, expected); // TODO
    }

    #[test]
    fn test_square_mul()
    {
        test_square_mul_impl::<RowMajor,    RowMajor>();
        test_square_mul_impl::<RowMajor,    ColumnMajor>();
        test_square_mul_impl::<ColumnMajor, RowMajor>();
        test_square_mul_impl::<ColumnMajor, ColumnMajor>();
    }

    fn test_vector_mul1_impl<O1: Ordering, O2: Ordering>()
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
        // TODO what should the ordering of m3 be?
    }

    #[test]
    fn test_vector_mut1()
    {
        test_vector_mul1_impl::<RowMajor,    RowMajor>();
        test_vector_mul1_impl::<RowMajor,    ColumnMajor>();
        test_vector_mul1_impl::<ColumnMajor, RowMajor>();
        test_vector_mul1_impl::<ColumnMajor, ColumnMajor>();
    }

    fn test_vector_mul2_impl<O1: Ordering, O2: Ordering>()
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
        // TODO what should the ordering of m3 be?
    }

    #[test]
    fn test_vector_mut2()
    {
        test_vector_mul2_impl::<RowMajor,    RowMajor>();
        test_vector_mul2_impl::<RowMajor,    ColumnMajor>();
        test_vector_mul2_impl::<ColumnMajor, RowMajor>();
        test_vector_mul2_impl::<ColumnMajor, ColumnMajor>();
    }

    fn add_all_rm<O: Ordering>(m: &Matrix<i64, O>) -> i64
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

    fn add_all_cm<O: Ordering>(m: &Matrix<i64, O>) -> i64
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

    #[bench]
    fn fast_add_all1(bench: &mut Bencher) -> ()
    {
        let m = make_big_matrix::<RowMajor>();
        bench.iter(|| test::black_box(add_all_rm(&m)));
    }

    #[bench]
    fn fast_add_all2(bench: &mut Bencher) -> ()
    {
        let m = make_big_matrix::<ColumnMajor>();
        bench.iter(|| test::black_box(add_all_cm(&m)));
    }

    #[bench]
    fn slow_add_all1(bench: &mut Bencher) -> ()
    {
        let m = make_big_matrix::<RowMajor>();
        bench.iter(|| test::black_box(add_all_cm(&m)));
    }

    #[bench]
    fn slow_add_all2(bench: &mut Bencher) -> ()
    {
        let m = make_big_matrix::<ColumnMajor>();
        bench.iter(|| test::black_box(add_all_rm(&m)));
    }

    // this bench should always run slower than fast_multiply
    #[bench]
    fn slow_multiply(bench: &mut Bencher) -> ()
    {
        let a = make_big_matrix::<ColumnMajor>();
        let b = make_big_matrix::<RowMajor>();

        bench.iter(|| &a * &b);
    }

    #[bench]
    fn fast_multiply(bench: &mut Bencher) -> ()
    {
        let a = make_big_matrix::<RowMajor>();
        let b = make_big_matrix::<ColumnMajor>();

        bench.iter(|| &a * &b);
    }
}

// TODO implement iterators
