use std::fmt;
use std::iter;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

pub trait Ordering {
    fn idx(major_dimension: usize, x: usize, y: usize) -> usize;
}

pub struct RowMajor {}
impl Ordering for RowMajor {
    fn idx(row_dimension: usize, x: usize, y: usize) -> usize
    {
        x + y*row_dimension
    }
}

pub struct ColumnMajor {}
impl Ordering for ColumnMajor {
    fn idx(column_dimension: usize, x: usize, y: usize) -> usize
    {
        y + x*column_dimension
    }
}

// NxN matrix
pub struct NMat<T, O: Ordering> {
    n: usize,
    values: Vec<T>,
    ordering: PhantomData<O>
}

impl<T, O: Ordering> NMat<T, O> {
    pub fn n(&self) -> usize {
        self.n
    }
}

impl<T: Default + Clone, O: Ordering> NMat<T, O> {
    pub fn new(n: usize) -> Self
    {
        Self {
            n,
            values: iter::repeat(Default::default()).take(n*n).collect(),
            ordering: PhantomData,
        }
    }
}

impl<T: Clone, O: Ordering> NMat<T, O> {
    pub fn new_with_default(n: usize, default: T) -> Self
    {
        Self {
            n,
            values: iter::repeat(default).take(n*n).collect(),
            ordering: PhantomData,
        }
    }
}

impl<T: fmt::Debug, O: Ordering> fmt::Debug for NMat<T, O> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // figure out how much space I need for every element
        let mut max_space = 0;
        for x in 0..self.n() {
            for y in 0..self.n() {
                let s = format!("{:?}", self[(x, y)]);
                if s.len() > max_space {
                    max_space = s.len()
                }
            }
        }

        write!(f, "\n");

        for yiter in (0..self.n()+1).rev() {

            if yiter == 0 {
                write!(f, "   ");
                for x in 0..self.n() {
                    write!(f, "{:<width$} ", x, width=max_space);
                }
                break;
            }

            let y = yiter - 1;
            for x in 0..self.n() {
                if x == 0 {
                    // print the current row number
                    write!(f, "{}: ", y);
                }

                write!(f, "{:<width$?} ", self[(x, y)], width=max_space);

                if x == self.n() - 1 {
                    write!(f, "\n");
                }
            }

        }
        Ok(())
    }
}

impl<T, O: Ordering> Index<(usize, usize)> for NMat<T, O> {
    type Output = T;

    fn index(&self, index: (usize, usize)) -> &Self::Output
    {
        let idx = O::idx(self.n, index.0, index.1);
        &self.values[idx]
    }
}

impl<T, O: Ordering> IndexMut<(usize, usize)> for NMat<T, O> {
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output
    {
        let idx = O::idx(self.n, index.0, index.1);
        &mut self.values[idx]
    }
}

#[cfg(test)]
mod test {
    fn test_simple_matrix_impl<O: Ordering>() {
        let mut mat: NMat<i64, O> = NMat::new(4);
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
    fn test_simple_matrix() {
        test_simple_matrix_impl::<RowMajor>();
        test_simple_matrix_impl::<ColumnMajor>();
    }
}

// TODO implement iterators
// TODO implement debug
