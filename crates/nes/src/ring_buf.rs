#![allow(dead_code)]

pub struct RingBuf<T> {
    items: Vec<T>,
    capacity: usize,
    reader_idx: usize,
    writer_idx: usize,
}

impl<T: Default + Clone> RingBuf<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            items: vec![T::default(); capacity],
            capacity,
            reader_idx: 0,
            writer_idx: 0,
        }
    }
}

impl<T: Clone> RingBuf<T> {
    pub fn push(&mut self, item: T) {
        self.items[self.writer_idx] = item;
        self.advance_writer();
        if self.reader_idx == self.writer_idx {
            self.advance_reader();
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.reader_idx == self.writer_idx {
            None
        } else {
            let item = self.items[self.reader_idx].clone();
            self.advance_reader();
            Some(item)
        }
    }

    pub fn iter(&mut self) -> impl Iterator<Item = T> + '_ {
        std::iter::from_fn(|| self.pop())
    }

    pub fn take_iter(&mut self) -> RingIter<'_, T> {
        let iter = RingIter {
            items: &self.items,
            cursor: RingIterCursor {
                done: false,
                capacity: self.capacity,
                reader_idx: self.reader_idx,
                writer_idx: self.writer_idx,
            },
        };

        self.reader_idx = self.writer_idx;

        iter
    }

    pub fn iter_ref(&self) -> RingIterRef<'_, T> {
        let iter = RingIterRef {
            items: &self.items,
            cursor: RingIterCursor {
                done: false,
                capacity: self.capacity,
                reader_idx: self.reader_idx,
                writer_idx: self.writer_idx,
            },
        };

        iter
    }

    fn advance_reader(&mut self) {
        self.reader_idx += 1;
        self.reader_idx *= (self.reader_idx < self.capacity) as usize;
    }

    fn advance_writer(&mut self) {
        self.writer_idx += 1;
        self.writer_idx *= (self.writer_idx < self.capacity) as usize;
    }
}

struct RingIterCursor {
    done: bool,
    capacity: usize,
    reader_idx: usize,
    writer_idx: usize,
}

impl RingIterCursor {
    fn advance_reader(&mut self) {
        self.reader_idx += 1;
        self.reader_idx *= (self.reader_idx < self.capacity) as usize;
    }

    fn retreat_writer(&mut self) {
        if self.writer_idx == 0 {
            self.writer_idx = self.capacity - 1;
        } else {
            self.writer_idx -= 1;
        }
    }
}

impl Iterator for RingIterCursor {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.reader_idx == self.writer_idx || self.done {
            self.done = true;
            None
        } else {
            let item = self.reader_idx;
            self.advance_reader();
            Some(item)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = if self.done {
            0
        } else if self.reader_idx < self.writer_idx {
            self.writer_idx - self.reader_idx
        } else {
            self.capacity - (self.reader_idx - self.writer_idx)
        };

        (len, Some(len))
    }
}

impl DoubleEndedIterator for RingIterCursor {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.done {
            None
        } else {
            self.retreat_writer();
            if self.reader_idx == self.writer_idx {
                self.done = true;
            }
            Some(self.writer_idx)
        }
    }
}

impl ExactSizeIterator for RingIterCursor {}

pub struct RingIter<'a, T> {
    items: &'a [T],
    cursor: RingIterCursor,
}

impl<'a, T: Clone> Iterator for RingIter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.cursor
            .next()
            .and_then(|idx| self.items.get(idx))
            .cloned()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.cursor.size_hint()
    }
}

impl<'a, T: Clone> DoubleEndedIterator for RingIter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.cursor
            .next_back()
            .and_then(|idx| self.items.get(idx))
            .cloned()
    }
}

impl<'a, T: Clone> ExactSizeIterator for RingIter<'a, T> {}

pub struct RingIterRef<'a, T> {
    items: &'a [T],
    cursor: RingIterCursor,
}

impl<'a, T> Iterator for RingIterRef<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.cursor.next().and_then(|idx| self.items.get(idx))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.cursor.size_hint()
    }
}

impl<'a, T> DoubleEndedIterator for RingIterRef<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.cursor.next_back().and_then(|idx| self.items.get(idx))
    }
}

impl<'a, T: Clone> ExactSizeIterator for RingIterRef<'a, T> {}
