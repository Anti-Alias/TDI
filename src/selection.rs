/// A list of elements where one of them is considered selected if non-empty.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct SelectionList<T> {
    elements: Vec<T>,
    selected_index: Option<usize>,
}

impl<T> Default for SelectionList<T> {
    fn default() -> Self {
        Self {
            elements: vec![],
            selected_index: None,
        }
    }
}

impl<T> SelectionList<T> {

    pub fn elements(&self) -> &[T] {
        &self.elements
    }

    pub fn elements_mut(&mut self) -> &mut [T] {
        &mut self.elements
    }

    pub fn iter(&self) -> impl Iterator<Item=&T> {
        self.elements.iter()
    } 

    pub fn push(&mut self, element: T) {
        self.elements.push(element);
        if self.elements.len() == 1 {
            self.selected_index = Some(0);
        }
    }

    pub fn insert(&mut self, index: usize, element: T) {
        let index = index.min(self.elements.len());
        self.elements.insert(index, element);
        if self.elements.len() == 1 {
            self.selected_index = Some(0);
        }
    }

    pub fn remove(&mut self, index: usize) -> T {
        let result  = self.elements.remove(index);
        if self.elements.is_empty() {
            self.selected_index = None;
        }
        else {
            let selected_index = self.selected_index.unwrap();
            let last_index = self.elements.len()-1;
            if selected_index > last_index {
                self.selected_index = Some(last_index);
            }
        }
        result
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    pub fn select_first(&mut self) {
        if self.elements.is_empty() { return }
        self.selected_index = Some(0);
    }

    pub fn select_last(&mut self) {
        if self.elements.is_empty() { return };
        self.selected_index = Some(self.elements.len()-1);
    }

    pub fn select(&mut self, index: usize) {
        if self.elements.is_empty() { return };
        let index = index.min(self.elements.len() - 1);
        self.selected_index = Some(index);
    }

    pub fn select_forwards(&mut self, amount: usize) {
        let Some(selected) = &mut self.selected_index else {
            return;
        };
        *selected = (*selected + amount).min(self.elements.len()-1);
    }

    pub fn select_forwards_wrapping(&mut self, amount: usize) {
        let Some(selected) = &mut self.selected_index else {
            return;
        };
        *selected = (*selected + amount) % self.elements.len()
    }

    pub fn select_backwards(&mut self, amount: usize) {
        let Some(selected) = &mut self.selected_index else {
            return;
        };
        if amount > *selected {
            *selected = 0;
        }
        else {
            *selected -= amount;
        }
    }

    pub fn select_backwards_wrapping(&mut self, amount: usize) {
        let Some(selected) = self.selected_index else {
            return;
        };
        let len = self.elements.len() as isize;
        let selected = selected as isize - amount as isize;
        let selected = ((selected % len) + len) % len;
        self.selected_index = Some(selected as usize)
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    pub fn selected(&self) -> Option<&T> {
        self.selected_index.map(|idx|  &self.elements[idx])
    }

    pub fn selected_mut(&mut self) -> Option<&mut T> {
        self.selected_index.map(|idx| &mut self.elements[idx])
    }
}
