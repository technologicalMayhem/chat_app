use std::slice::Iter;

/// A wrapper around ``Vec<T>`` holding the index of a element to be considered 'active'.
#[derive(Clone)]
pub struct ActiveVec<T> {
    items: Vec<T>,
    active_index: Option<usize>,
}

impl<T> ActiveVec<T> {
    /// Creates a new instance of ``ActiveVec<T>``.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            active_index: None,
        }
    }

    /// Appends an element to the back of the collection. Marks the element also as active, if it's the only element in the collection.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds `isize::MAX` bytes.
    pub fn push(&mut self, item: T) {
        self.items.push(item);
        if self.active_index.is_none() {
            self.active_index = Some(0);
        }
    }

    /// Get the index for the currently active element. Returns ``None`` if the collection is empty.
    pub fn get_active_index(&self) -> Option<usize> {
        self.active_index
    }

    /// Get a reference to the active element. Returns ``None`` if the collection is empty.
    pub fn get_active(&self) -> Option<&T> {
        self.active_index.and_then(|i| self.items.get(i))
    }

    /// Get a mutable reference to the active element. Returns ``None`` if the collection is empty.
    pub fn get_active_mut(&mut self) -> Option<&mut T> {
        self.active_index.and_then(|i| self.items.get_mut(i))
    }

    /// Returns an iterator over the elements in the collection.
    pub fn iter(&self) -> Iter<'_, T> {
        self.items.iter()
    }

    /// Increments the index of the active element. Wraps around to the start if the end has been reached.
    /// If no elemnts are in the collection, nothing happens.
    pub fn next(&mut self) {
        if let Some(index) = self.active_index {
            if index + 1 >= self.items.len() {
                self.active_index = Some(0);
            } else {
                self.active_index = Some(index + 1);
            }
        }
    }

    /// Decrements the index of the active element. Wraps around to the end if the index is at the start.
    /// If no elemnts are in the collection, nothing happens.
    pub fn prev(&mut self) {
        if let Some(index) = self.active_index {
            if index == 0 {
                self.active_index = Some(self.items.len() - 1);
            } else {
                self.active_index = Some(index - 1);
            }
        }
    }
}

impl<T> FromIterator<T> for ActiveVec<T> {
    fn from_iter<Iter: IntoIterator<Item = T>>(iter: Iter) -> Self {
        let items: Vec<T> = iter.into_iter().collect();
        let active_index = if items.is_empty() { None } else { Some(0) };
        Self {
            items,
            active_index,
        }
    }
}

impl<T> From<Vec<T>> for ActiveVec<T> {
    fn from(value: Vec<T>) -> Self {
        let mut active_vec = ActiveVec::new();
        for ele in value {
            active_vec.push(ele);
        }

        active_vec
    }
}
