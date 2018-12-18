use core::ptr::NonNull;
use core::marker::PhantomData;
use core::borrow::Borrow;
use core::mem;
use core::cmp::Ordering::{self, Less, Equal, Greater};

pub struct SkipMap<K, V, T> 
where K: Ord, T: Tower<K, V> {
    head_tower: T,
    tail: Option<NonNull<Node<K, V, T>>>,
    len: usize,
    _marker: PhantomData<Box<Node<K, V, T>>>,
}

impl<K, V, T> SkipMap<K, V, T>
where K: Ord, T: Tower<K, V> {
    pub fn new() -> SkipMap<K, V, T> {
        SkipMap {
            head_tower: T::new(),
            tail: None,
            len: 0,
            _marker: PhantomData
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<K, V, T> SkipMap<K, V, T>
where K: Ord, T: Tower<K, V> {
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        match self.head_tower.find_mut(&key) {
            FindMut::BeforeHead => {unimplemented!()}, // new head
            FindMut::EqualsValue(place) => return Some(mem::replace(place, value)),
            FindMut::AfterValue(place) => {unimplemented!()}, // new internal node
        }
    }

    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where K: Borrow<Q>, Q: Ord + ?Sized {
        match self.head_tower.find_mut(&key) {
            FindMut::BeforeHead | FindMut::AfterValue(_) => None,
            FindMut::EqualsValue(place) => unimplemented!(), // remove value
        }
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V> 
    where K: Borrow<Q>, Q: Ord + ?Sized {
        match self.head_tower.find(&key) {
            Find::BeforeHead | Find::AfterValue(_) => None,
            Find::EqualsValue(place) => Some(place), 
        }
    } 

    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V> 
    where K: Borrow<Q>, Q: Ord + ?Sized {
        match self.head_tower.find_mut(&key) {
            FindMut::BeforeHead | FindMut::AfterValue(_) => None,
            FindMut::EqualsValue(place) => Some(place), 
        }
    } 

    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where K: Borrow<Q>, Q: Ord + ?Sized {
        match self.head_tower.find(&key) {
            Find::BeforeHead | Find::AfterValue(_) => false,
            Find::EqualsValue(_) => true, 
        }
    }

    pub fn clear(&mut self) {
        unimplemented!()
    }
}

pub struct Node<K, V, T> 
where K: Ord, T: Tower<K, V> {
    key: K,
    value: V,
    tower: T,
}

impl<K, V, T> Node<K, V, T> 
where K: Ord, T: Tower<K, V> {
    pub fn new(key: K, value: V) -> Self {
        Self {
            key,
            value,
            tower: T::new()
        }
    }
}

pub enum Find<'a, V> {
    BeforeHead,
    AfterValue(&'a V),
    EqualsValue(&'a V),
}

pub enum FindMut<'a, V> {
    BeforeHead,
    AfterValue(&'a mut V),
    EqualsValue(&'a mut V),
}

pub trait Tower<K, V>
where K: Ord, Self: Sized {
    fn new() -> Self;

    fn len(&self) -> usize;

    //  1  [3]  5   7   9
    //  2: Less
    //  3: Equal
    //  4: Greater    
    fn cmp_key<Q>(&self, index: usize, key: &Q) -> Option<Ordering>
    where K: Borrow<Q>, Q: Ord + ?Sized; 
    
    fn next_value_tower_ref(&self, index: usize) -> Option<(&V, &Self)>;

    fn next_value_tower_mut(&mut self, index: usize) -> Option<(&mut V, &mut Self)>;

    //todo: macro

    fn find<Q>(&self, key: &Q) -> Find<V>
    where K: Borrow<Q>, Q: Ord + ?Sized {
        let mut h = self.len() - 1;
        let mut cur_tower = self;
        let mut ans = None;
        loop {
            match (h, cur_tower.cmp_key(h, key)) {
                (0, Some(Greater)) => break ans.map(|place| Find::AfterValue(place)).unwrap_or(Find::BeforeHead),
                (_, Some(Equal)) => break ans.map(|place| Find::EqualsValue(place))
                    .unwrap_or_else(|| unreachable!("ans is absent but key equals next[h].key")),
                (_, Some(Greater)) | (_, None) => h -= 1,
                (_, Some(Less)) => if let Some((next_value, next_tower)) = cur_tower.next_value_tower_ref(h) {
                    ans = Some(next_value);
                    cur_tower = next_tower;
                } else { unreachable!("tower present but value absent") },
            }
        }
    }

    fn find_mut<Q>(&mut self, key: &Q) -> FindMut<V>
    where K: Borrow<Q>, Q: Ord + ?Sized {
        let mut h = self.len() - 1;
        let mut cur_tower = self;
        let mut ans = None;
        loop {
            match (h, cur_tower.cmp_key(h, key)) {
                (0, Some(Greater)) => break ans.map(|place| FindMut::AfterValue(place)).unwrap_or(FindMut::BeforeHead),
                (_, Some(Equal)) => break ans.map(|place| FindMut::EqualsValue(place))
                    .unwrap_or_else(|| unreachable!("ans is absent but key equals next[h].key")),
                (_, Some(Greater)) | (_, None) => h -= 1,
                (_, Some(Less)) => if let Some((next_value, next_tower)) = cur_tower.next_value_tower_mut(h) {
                    ans = Some(next_value);
                    cur_tower = next_tower;
                } else { unreachable!("tower present but value absent") },
            }
        }
    }
}

pub trait IndexTower<K, V>
where K: Ord, Self: Tower<K, V> {
    fn find_mut<Q>(&mut self, key: &Q) -> Option<(&K, &mut V, &mut Self)>
    where K: Borrow<Q>, Q: Ord + ?Sized {
        unimplemented!("233")
    }
}

const ARRAY_TOWER_MAX: usize = 12;

pub struct ArrayTower<K, V> 
where K: Ord {
    len: usize,
    inner: [Option<NonNull<Node<K, V, ArrayTower<K, V>>>>; ARRAY_TOWER_MAX],
}

impl<K, V> Tower<K, V> for ArrayTower<K, V>
where K: Ord {
    fn new() -> Self {
        Self { 
            len: gen_height(),
            inner: [None; ARRAY_TOWER_MAX]
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    //  1  [3]  5   7   9
    //  2: Less
    //  3: Equal
    //  4: Greater  
    fn cmp_key<Q>(&self, index: usize, key: &Q) -> Option<Ordering>
    where K: Borrow<Q>, Q: Ord + ?Sized {
        self.inner[index].map(|node_ptr| unsafe {
            key.cmp(node_ptr.as_ref().key.borrow())
        })
    }
    
    fn next_value_tower_ref(&self, index: usize) -> Option<(&V, &Self)> {
        self.inner[index].map(|node_ptr| unsafe {
            let node = &*node_ptr.as_ptr();
            (&node.value, &node.tower)
        })
    }

    fn next_value_tower_mut(&mut self, index: usize) -> Option<(&mut V, &mut Self)> {
        self.inner[index].map(|node_ptr| unsafe {
            let node = &mut *node_ptr.as_ptr();
            (&mut node.value, &mut node.tower)
        })
    }

}

// returns a height within [1, MAX_HEIGHT]
// where P(h=1)=0.5, P(h=2)=0.25, etc.
fn gen_height() -> usize {
    use rand::distributions::{Distribution, Bernoulli};
    let mut ans = 1;
    let mut rng = rand::thread_rng(); 
    let half_half = Bernoulli::from_ratio(1, 2);
    while half_half.sample(&mut rng) {
        ans += 1;
        if ans == ARRAY_TOWER_MAX {
            break;
        }
    }
    ans
}
