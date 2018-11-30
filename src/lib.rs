#![feature(box_into_raw_non_null)]
use core::borrow::Borrow;
use core::cmp::Ordering::{Less, Equal, Greater};
use core::fmt;
use core::iter::FusedIterator;
use core::ptr::{self, NonNull};
use core::mem;
use core::marker::PhantomData;

const MAX_HEIGHT: usize = 12;

type Tower<K, V> = Vec<Option<NonNull<Node<K, V>>>>; 

pub struct SkipMap<K: Ord, V> {
    len: usize,
    tower: Tower<K, V>,
    _marker: PhantomData<Box<Node<K, V>>>,
}

impl<K: Ord, V> SkipMap<K, V> {
    pub fn new() -> Self {
        Self {
            len: 0,
            tower: vec![None; MAX_HEIGHT],
            _marker: PhantomData
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    } 

    pub fn iter(&self) -> Iter<K, V> {
        Iter {
            head: self.tower[0],
            len: self.len,
            _marker: PhantomData
        }
    }
}

impl<K: Ord, V> SkipMap<K, V> {
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let mut cur_tower = &self.tower as *const _ as *mut Tower<K, V>;
        let mut h = self.tower.len() - 1;
        let new_height = next_height();
        let mut to_modify = Vec::with_capacity(new_height);
        loop {
            let mut seek_down = false;
            if let Some(ref mut next_ptr) = unsafe { &mut *cur_tower }[h] {
                let next = unsafe { next_ptr.as_ref() };
                match next.key.cmp(&key) {
                    Less => cur_tower = &next.tower as *const _ as *mut _,
                    Greater => seek_down = true,
                    Equal => {
                        let mut ans = value;
                        unsafe { next_ptr.as_mut() }.key = key;
                        mem::swap(&mut unsafe { next_ptr.as_mut() }.value, &mut ans);
                        return Some(ans);
                    },
                }
            } else {
                seek_down = true;
            }
            if seek_down {
                if h < new_height {
                    to_modify.push((cur_tower, h));
                }
                if h > 0 { 
                    h -= 1;
                } else { 
                    break;
                }
            }
        }
        let mut new_node = Box::new(Node::new(key, value, new_height));
        for (prev_tower_ptr, h) in to_modify.iter() {
            if let Some(place_mut) = new_node.tower.get_mut(*h) {
                *place_mut = unsafe { &**prev_tower_ptr }[*h];
            }
        }
        let new_node_ptr = Box::into_raw_non_null(new_node);
        for (prev_tower_ptr, h) in to_modify.iter() {
            unsafe { (**prev_tower_ptr)[*h] = Some(new_node_ptr) };
        }
        self.len += 1;
        None
    }

    pub fn clear(&mut self) {
        let mut cur_opt = self.tower[0];
        self.tower = vec![None; MAX_HEIGHT];
        while let Some(next_ptr) = cur_opt {
            let next_node = unsafe { Box::from_raw(next_ptr.as_ptr()) };
            cur_opt = next_node.tower[0];
            drop(next_node);
            self.len -= 1;
        }
    }

    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&V> 
    where K: Borrow<Q>, Q: Ord {
        let mut cur_tower = &self.tower as *const _ as *mut Tower<K, V>;
        let mut h = self.tower.len() - 1;
        loop {
            let mut seek_down = false;
            if let Some(ref mut next_ptr) = unsafe { &mut *cur_tower }[h] {
                let next = unsafe { next_ptr.as_ref() };
                match next.key.borrow().cmp(&key) {
                    Less => cur_tower = &next.tower as *const _ as *mut _,
                    Greater => seek_down = true,
                    Equal => return Some(&next.value)
                }
            } else {
                seek_down = true;
            }
            if seek_down {
                if h > 0 { 
                    h -= 1;
                } else { 
                    return None;
                }
            }
        }
    }

    pub fn remove<Q: ?Sized>(&mut self, key: &Q) -> Option<V> 
    where K: Borrow<Q>, Q: Ord {
        let mut cur_tower = &self.tower as *const _ as *mut Tower<K, V>;
        let mut h = self.tower.len() - 1;
        let cur_height = MAX_HEIGHT;
        let mut to_modify: Vec<(*mut Tower<K, V>, usize)> = Vec::with_capacity(cur_height);
        loop {
            let mut seek_down = false;
            if let Some(ref mut next_ptr) = unsafe { &mut *cur_tower }[h] {
                let next = unsafe { next_ptr.as_ref() };
                match next.key.borrow().cmp(&key) {
                    Less => cur_tower = &next.tower as *const _ as *mut _,
                    Greater => seek_down = true,
                    Equal => {
                        let value = unsafe { ptr::read(next_ptr.as_ptr()) }.value;
                        
                        for (prev_tower_ptr, h) in to_modify.iter() {
                            if *h < unsafe { next_ptr.as_ref() }.tower.len() {
                                unsafe { (**prev_tower_ptr)[*h] = next_ptr.as_ref().tower[*h] };
                            }
                        }
                        self.len -= 1;
                        return Some(value);
                    },
                }
            } else {
                seek_down = true;
            }
            if seek_down {
                if h < cur_height {
                    to_modify.push((cur_tower, h));
                }
                if h > 0 { 
                    h -= 1;
                } else { 
                    return None;
                }
            }
        }
    }

    // fn find_node<Q: ?Sized>(&mut self, key: &Q) 
    // -> Option<(Vec<(*mut Tower<K, V>, usize)>, &Node<K, V>)> 
    // where K: Borrow<Q>, Q: Ord {
        
    // }
}

impl<K: Ord + fmt::Debug, V: fmt::Debug> fmt::Debug for SkipMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<K: Ord, V> Drop for SkipMap<K, V> {
    fn drop(&mut self) {
        self.clear();
    }
}

pub struct Iter<'a, K: 'a + Ord, V: 'a> {
    head: Option<NonNull<Node<K, V>>>,
    len: usize,
    _marker: PhantomData<(&'a K, &'a V)>,
}

impl<'a, K: Ord, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);
    
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next_ptr) = self.head {
            let next_node = unsafe { &*next_ptr.as_ptr() };
            self.head = next_node.tower[0];
            self.len -= 1;
            Some((&next_node.key, &next_node.value))
        } else {
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, K: Ord + fmt::Debug, V> fmt::Debug for Iter<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Iter").field(&self.len).finish()
    }
}

impl<'a, K: Ord, V> ExactSizeIterator for Iter<'a, K, V> {}

impl<'a, K: Ord, V> FusedIterator for Iter<'a, K, V> {}

#[derive(Debug)]
struct Node<K: Ord, V> {
    key: K,
    value: V,
    tower: Tower<K, V>,
    _marker: PhantomData<Box<Node<K, V>>>,
}

impl<K: Ord, V> Node<K, V> {
    fn new(key: K, value: V, height: usize) -> Self {
        Self {
            key,
            value, 
            tower: vec![None; height],
            _marker: PhantomData,
        }
    }
}

// returns a height within [1, MAX_LEVEL]
// where P(h=1)=0.5, P(h=2)=0.25, etc.
fn next_height() -> usize {
    use rand::distributions::{Distribution, Bernoulli};
    let mut ans = 1;
    let mut rng = rand::thread_rng(); 
    let half_half = Bernoulli::from_ratio(1, 2);
    while half_half.sample(&mut rng) {
        ans += 1;
        if ans == MAX_HEIGHT {
            break;
        }
    }
    ans
}

#[test]
fn skip_map_insert_test() {
    let generate_value = |key| key + 10000;
    let test = |times, numbers| {
        for _ in 0..times {
            let mut map = SkipMap::new();
            assert!(map.is_empty());
            for i in (0..numbers).rev() {
                map.insert(i, generate_value(i));
            }
            assert_eq!(map.len(), numbers);
            let mut i = 0;
            for (key, value) in map.iter() {
                assert_eq!(*key, i);
                assert_eq!(*value, generate_value(i));
                i += 1;
            }
            map.clear();
            assert_eq!(map.len(), 0);
        }
    };
    test(1000, 10);
    test(10, 1000);
}

#[test]
fn skip_map_remove_test() {
    for _ in 0..1000 {
        let mut map = SkipMap::new();
        assert!(map.is_empty());
        for i in (0..10).rev() {
            map.insert(i, i + 1000);
        }
        println!("{:?}", map);
        for i in (3..8).rev() {
            map.remove(&i);
        }
        println!("{:?}", map);
    }
}

// fn main() {
//     let mut map = SkipMap::new();
//     loop {
//         let mut input = String::new();
//         std::io::stdin().read_line(&mut input).unwrap();
//         let mut iter = input.split_whitespace();
//         match iter.next() {
//             Some("i") => {
//                 let key = iter.next().unwrap();
//                 let value = iter.next().unwrap();
//                 println!("{}, {:?}", map.len(), map.insert(key.to_owned(), value.to_owned()));
//             },
//             Some("p") => {
//                 println!("{}, {:?}", map.len(), map);
//             },
//             Some("r") => {
//                 let key = iter.next().unwrap();
//                 println!("{}, {:?}", map.len(), map.remove(key));
//             },
//             Some("g") => {
//                 let key = iter.next().unwrap();
//                 println!("{:?}", map.get(key));
//             },
//             _ => {}
//         }
//     }
// }
