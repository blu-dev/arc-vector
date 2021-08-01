#![allow(dead_code)]
use std::ops::{Range, Index, IndexMut};

pub struct ArcVector<T: Clone> {
    has_realloced: bool,
    start: *mut *mut T,
    size: *mut u32,
    cap: usize,
    additional_size: Option<*mut u32>
}

impl<T: Clone> ArcVector<T> {
    fn next_realloc(&mut self, capacity: usize) {
        use skyline::libc::{malloc, free, memcpy, c_void};
        use std::mem::size_of;
        let size_of_type = size_of::<T>();
        let cur_cap = self.capacity();
        let new_cap = capacity.max(cur_cap * 2);
        unsafe {
            let current = *self.start;
            let new = malloc(new_cap * size_of_type);
            memcpy(new, current as *mut c_void, cur_cap * size_of_type);
            *self.start = new as *mut T;
            if self.has_realloced {
                free(current as *mut c_void);
            }
        }
        self.cap = new_cap;
        self.has_realloced = true;
    }

    fn add_optional(first: *mut u32, second: &Option<*mut u32>) -> u32 {
        if let Some(second) = second.clone() {
            unsafe { *first + *second }
        } else {
            unsafe { *first }
        }
    }

    pub fn new(start: *mut *mut T, count: *mut u32, additional_size: Option<*mut u32>) -> Self {
        let cap = Self::add_optional(count, &additional_size) as usize;
        Self {
            has_realloced: false,
            start,
            size: count,
            cap,
            additional_size: additional_size
        }
    }

    pub fn reserve(&mut self, new_capacity: usize) {
        if new_capacity > self.capacity() {
            self.next_realloc(new_capacity)
        }
    }

    pub fn set_len(&mut self, new_len: usize) {
        use skyline::libc::{memset, c_void};
        if new_len > self.len() {
            if new_len >= self.capacity() {
                self.next_realloc(new_len);
            }
            let sub_len = Self::add_optional(&0 as *const i32 as *mut u32, &self.additional_size) as usize;
            unsafe {
                memset((*self.start).add(self.len()) as *mut c_void, 0x0, new_len - self.len());
                *self.size = (new_len - sub_len) as u32;
            }
        }
    }

    pub fn set_capacity(&mut self, new_cap: usize) {
        self.cap = new_cap;
    }

    pub fn push(&mut self, obj: T) {
        self.reserve(self.len() + 1);
        unsafe {
            *(*self.start).add(self.len()) = obj;
            *self.size += 1;
        }
    }

    #[track_caller]
    pub fn push_from_within(&mut self, idx: usize) {
        self.push(self.get(idx).expect("Index out of bounds!").clone());
    }

    pub fn extend(&mut self, slice: &[T]) {
        use skyline::libc::{c_void, memcpy};
        self.reserve(self.len() + slice.len());
        unsafe {
            // still sorry
            memcpy((*self.start).add(self.len()) as *mut c_void, slice.as_ptr() as *const c_void, slice.len() * std::mem::size_of::<T>());
        }
        for obj in slice.iter(){
            self.push(obj.clone())
        }
    }

    #[track_caller]
    pub fn extend_from_within(&mut self, start: usize, count: usize) {
        use skyline::libc::{c_void, memcpy};
        if start + count > self.len() {
            panic!("Index out of bounds!")
        }

        self.reserve(self.len() + count);
        unsafe {
            // I'm sorry, father
            memcpy((*self.start).add(self.len()) as *mut c_void, (*self.start).add(start) as *const c_void, count * std::mem::size_of::<T>());
            *self.size += count as u32;
        }
    }

    pub fn as_ptr(&self) -> *const T {
        unsafe { *self.start as *const T }
    }

    pub fn as_mut_ptr(&mut self) -> *mut T {
        unsafe { *self.start }
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(*self.start, self.len()) }
    }

    pub fn as_slice_mut(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(*self.start, self.len()) }
    }

    pub fn capacity(&self) -> usize {
        self.cap
    }

    pub fn len(&self) -> usize {
        Self::add_optional(self.size, &self.additional_size) as usize
    }

    pub fn get(&self, idx: usize) -> Option<&T> {
        if idx < self.len() {
            unsafe {
                Some(&*(*self.start).add(idx))
            }
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
        if idx < self.len() {
            unsafe {
                Some(&mut *(*self.start).add(idx))
            }
        } else {
            None
        }
    }

    pub fn last(&self) -> Option<&T> {
        self.get(self.len() - 1)
    }

    pub fn last_mut(&mut self) -> Option<&mut T> {
        self.get_mut(self.len() - 1)
    }

    pub fn iter(&self) -> ArcVectorIter<T> {
        self.into_iter()
    }

    pub fn iter_mut(&mut self) -> ArcVectorIterMut<T> {
        self.into_iter()
    }
}

pub struct ArcVectorIter<'a, T: Clone> {
    vector: &'a ArcVector<T>,
    index: usize
}

pub struct ArcVectorIterMut<'a, T: Clone> {
    vector: &'a mut ArcVector<T>,
    index: usize
}

impl<'a, T: Clone> Iterator for ArcVectorIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.vector.len() {
            self.index += 1;
            unsafe {
                Some(std::mem::transmute::<*mut T, Self::Item>(
                    (*self.vector.start).add(self.index - 1)
                ))
            }
        } else {
            None
        }
    }
}

impl<'a, T: Clone> Iterator for ArcVectorIterMut<'a, T> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.vector.len() {
            self.index += 1;
            unsafe {
                Some(std::mem::transmute::<*mut T, Self::Item>(
                    (*self.vector.start).add(self.index - 1)
                ))
            }
        } else {
            None
        }
    }
}

impl<'a, T: Clone> IntoIterator for &'a ArcVector<T> {
    type Item = &'a T;
    type IntoIter = ArcVectorIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        ArcVectorIter {
            vector: self,
            index: 0
        }
    }
}

impl<'a, T: Clone> IntoIterator for &'a mut ArcVector<T> {
    type Item = &'a mut T;
    type IntoIter = ArcVectorIterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        ArcVectorIterMut {
            vector: self,
            index: 0
        }
    }
}

macro_rules! index_ref_vec {
    ($($t:ty)*) => {
        $(
            impl<T: Clone> Index<$t> for ArcVector<T> {
                type Output = T;
    
                fn index(&self, idx: $t) -> &Self::Output {
                    self.get(idx as usize).unwrap()
                }
            }
    
            impl<T: Clone> IndexMut<$t> for ArcVector<T> {
                fn index_mut(&mut self, idx: $t) -> &mut Self::Output {
                    self.get_mut(idx as usize).unwrap()
                }
            }

            impl<T: Clone> Index<Range<$t>> for ArcVector<T> {
                type Output = [T];
                fn index(&self, idx: Range<$t>) -> &Self::Output {
                    let new_start = idx.start as usize;
                    let new_end = idx.end as usize;
                    &self.as_slice()[new_start..new_end]
                }
            }

            impl<T: Clone> IndexMut<Range<$t>> for ArcVector<T> {
                fn index_mut(&mut self, idx: Range<$t>) -> &mut Self::Output {
                    let new_start = idx.start as usize;
                    let new_end = idx.end as usize;
                    &mut self.as_slice_mut()[new_start..new_end]
                }
            }
        )*
    }
}

index_ref_vec!(usize u32);
