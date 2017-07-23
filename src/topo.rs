use util;

use std::iter;
use std::collections::VecDeque;

type AdjacencyMatrix = util::nmat::NMat<bool, util::nmat::RowMajor>;

/// Remove back edges, if any, from the adjacent matrix, in place
/// Assumes the graph is fully connected, starts traversal from node 0
pub fn remove_back_edges(adj: &mut AdjacencyMatrix)
{
    let mut visited: Vec<bool> = iter::repeat(false).take(adj.n()).collect();
    let mut next = VecDeque::new();

    next.push_back(0);

    while !next.is_empty() {
        let i = next.pop_front().unwrap();
        visited[i] = true;

        // TODO implement some nicer iterator?
        for j in 0..adj.n() {
            if adj[(i, j)] {
                if visited[j] {
                    // this must be a back edge, remove it from the graph
                    adj[(i, j)] = false;
                } else {
                    next.push_back(j);
                }
            }
        }
    }
}


// TODO test helper function independently

fn insert_all_with_no_preds(
    adj: &mut AdjacencyMatrix,
    next: &mut VecDeque<usize>,
    inserted: &mut Vec<bool>)
{
    // add any entries with no predecessors
    for i in 0..adj.n() {
        // TODO optimize with some better iterators or something
        let mut preds = false;
        for j in 0..adj.n() {
            preds |= adj[(j, i)];
        }

        if !preds && !inserted[i] {
            inserted[i] = true;
            next.push_back(i);
        }
    }
}

/// Sort the given adjacency matrix in place, if possible
/// Assumes that the graph is a DAG. If it is not, this may loop indefinitely
/// (no cycle detection is performed)
/// Returns the suggested topological ordering of the nodes
/// IMPORTANT: The passed in AdjacencyMatrix will be zeroed out in the process
/// of executing the algorithm.
pub fn topological_sort(adj: &mut AdjacencyMatrix) -> Vec<usize>
{
    let mut next = VecDeque::new();
    let mut inserted = iter::repeat(false).take(adj.n()).collect();
    let mut ordering = Vec::new();

    insert_all_with_no_preds(adj, &mut next, &mut inserted);
    assert!(!next.is_empty());

    while !next.is_empty() {
        let i = next.pop_front().unwrap();
        ordering.push(i);

        // remove all outgoing edges for i
        for j in 0..adj.n() {
            adj[(i, j)] = false;
        }

        insert_all_with_no_preds(adj, &mut next, &mut inserted);
    }

    ordering
}

// TODO optimize the top sort!

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_rm_simple_edge() {
        let mut adj: AdjacencyMatrix = util::nmat::NMat::new(2);
        // no self loops
        adj[(0, 0)] = false;
        adj[(1, 1)] = false;

        // directed edge from 0 -> 1 and from 1 -> 0
        adj[(0, 1)] = true;
        adj[(1, 0)] = true;

        remove_back_edges(&mut adj);

        // one of these edges must have been removed
        assert!(!adj[(0, 1)] || !adj[(1, 0)]);

        // one of these edges must still exist
        assert!(adj[(0, 1)] || adj[(1, 0)]);
    }

    #[test]
    fn test_multi_step_cycle() {
        let mut adj: AdjacencyMatrix = util::nmat::NMat::new(3);
        // defaults to false everywhere
        // connect 0 -> 1, 1 -> 2, 2 -> 0
        adj[(0, 1)] = true;
        adj[(1, 2)] = true;
        adj[(2, 0)] = true;

        remove_back_edges(&mut adj);

        // the only edge that is valid to remove is the 2 -> 0 edge
        assert!(!adj[(2, 0)]);

        // but the other edges should still exist
        assert!(adj[(0, 1)] && adj[(1, 2)]);
    }

    #[test]
    fn test_self_loop() {
        let mut adj: AdjacencyMatrix = util::nmat::NMat::new(2);
        // no self loops
        adj[(0, 0)] = true;
        adj[(1, 1)] = false;

        // directed edge from 0 -> 1
        adj[(0, 1)] = true;
        adj[(1, 0)] = false;

        remove_back_edges(&mut adj);

        // this edge must have been disconnected
        assert!(!adj[(0, 0)]);

        // this edge must still exist
        assert!(adj[(0, 1)]);
    }

    #[test]
    fn simple_topo() {
        let mut adj: AdjacencyMatrix = util::nmat::NMat::new(3);
        // default will be all false;

        adj[(0, 1)] = true;
        adj[(1, 2)] = true;

        let res = topological_sort(&mut adj);
        assert!(res[0] == 0);
        assert!(res[1] == 1);
        assert!(res[2] == 2);
    }

    #[test]
    fn interesting_topo() {
        let mut adj: AdjacencyMatrix = util::nmat::NMat::new(5);
        // default will all be false

        adj[(0, 1)] = true;
        adj[(0, 2)] = true;
        adj[(0, 4)] = true;
        adj[(1, 3)] = true;
        adj[(2, 3)] = true;

        let res = topological_sort(&mut adj);
        println!("res: {:?}", res);

        // 0 should be the first element
        assert!(res[0] == 0);

        // 1, 2, and 4 should all be before 3 (but their ordering doesn't matter)
        let one = res.iter().position(|e| *e == 1).unwrap();
        let two = res.iter().position(|e| *e == 2).unwrap();
        let three = res.iter().position(|e| *e == 3).unwrap();
        let four = res.iter().position(|e| *e == 4).unwrap();

        assert!(one < three);
        assert!(two < three);
        assert!(four < three);
    }
}
