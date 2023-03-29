use std::slice::Iter;

#[derive(Clone)]
pub struct ActiveVec<T> {
    items: Vec<T>,
    active_index: Option<usize>,
}

impl<T> ActiveVec<T> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            active_index: None,
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn push(&mut self, item: T) {
        self.items.push(item);
        if self.active_index.is_none() {
            self.active_index = Some(0);
        }
    }

    pub fn get_active_index(&self) -> Option<usize> {
        self.active_index
    }

    pub fn get_active(&self) -> Option<&T> {
        self.active_index.and_then(|i| self.items.get(i))
    }

    pub fn get_active_mut(&mut self) -> Option<&mut T> {
        self.active_index.and_then(|i| self.items.get_mut(i))
    }

    pub fn iter(&self) -> Iter<'_, T> {
        self.items.iter()
    }

    pub fn next(&mut self) {
        if let Some(index) = self.active_index {
            if index + 1 >= self.items.len() {
                self.active_index = Some(0);
            } else {
                self.active_index = Some(index + 1);
            }
        }
    }

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
