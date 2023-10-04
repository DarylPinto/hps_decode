/// An iterator that alternates through its inner iterators, ensuring elements
/// are yielded in an interleaved fashion until at least one inner iterator is
/// exhausted.
#[derive(Debug)]
pub(crate) struct InterleavingIterator<I: Iterator> {
    iterators: Vec<I>,
    collection_index: usize,
}

impl<I: Iterator> InterleavingIterator<I> {
    pub(crate) fn new<II>(collections: Vec<II>) -> Self
    where
        II: IntoIterator<IntoIter = I>,
    {
        Self {
            iterators: collections
                .into_iter()
                .map(IntoIterator::into_iter)
                .collect(),
            collection_index: 0,
        }
    }
}

impl<I: Iterator> Iterator for InterleavingIterator<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.iterators[self.collection_index].next();
        self.collection_index = (self.collection_index + 1) % self.iterators.len();
        item
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interleaves_correctly() {
        let letters = ["A", "B", "C"];
        let numbers = ["1", "2", "3"];
        let notes = ["do", "re", "mi"];

        let interleaved =
            InterleavingIterator::new(vec![letters, numbers, notes]).collect::<Vec<_>>();

        assert_eq!(
            interleaved,
            vec!["A", "1", "do", "B", "2", "re", "C", "3", "mi"]
        );
    }

    #[test]
    fn interleaves_correctly_with_one_inner_iterator() {
        let letters = ["A", "B", "C"];
        let interleaved = InterleavingIterator::new(vec![letters]).collect::<Vec<_>>();
        assert_eq!(interleaved, vec!["A", "B", "C"]);
    }
}
