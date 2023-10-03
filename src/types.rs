#[derive(Debug)]
pub(crate) struct InterleavingIterator<I: Iterator> {
    iterators: Vec<I>,
    container_index: usize,
}

impl<I: Iterator> InterleavingIterator<I> {
    pub(crate) fn new<II>(collections: Vec<II>) -> Self
    where
        II: IntoIterator<IntoIter = I>,
    {
        Self {
            iterators: collections
                .into_iter()
                .map(|collection| collection.into_iter())
                .collect(),
            container_index: 0,
        }
    }
}

impl<I: Iterator> Iterator for InterleavingIterator<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.iterators[self.container_index].next();
        self.container_index = (self.container_index + 1) % self.iterators.len();
        item
    }
}
