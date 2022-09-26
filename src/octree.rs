//! Octree rewrite partially inspired by parry's QBVH type (that one partitions space into
//! multiples of 4 rather than 8, though)

use parry3d::bounding_volume::AABB;
use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Attempted to access the parent of a root node")]
    ParentOfRoot,
    #[error("Child index out of range: {0}")]
    OutOfRange(u8),
    #[error("Attempted to access child of terminal node")]
    NoChildren,
}

pub struct Octree<T> {
    aabb: AABB,
    proxies: Vec<Proxy>,
    leaf_data: Vec<T>,
}

impl<T> Default for Octree<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Octree<T> {
    pub fn new() -> Self {
        Self {
            aabb: AABB::new_invalid(),
            proxies: vec![Proxy {
                parent: 0,
                data: ProxyData::Void,
            }],
            leaf_data: Vec::default(),
        }
    }

    pub fn view(&self) -> View<&Self> {
        View::new(self, 0)
    }

    pub fn view_mut(&mut self) -> View<&mut Self> {
        View::new(self, 0)
    }
}

pub struct Proxy {
    parent: u32,
    data: ProxyData,
}

pub enum ProxyData {
    Void,
    Leaf(u32),
    Branch([u32; 8]),
}

#[derive(Debug, Copy, Clone)]
pub struct View<'tree, Tree: 'tree> {
    tree: Tree,
    target: u32,
    _lifetime: PhantomData<&'tree ()>,
}

impl<'tree, T: 'tree, Tree: Deref<Target = Octree<T>> + 'tree> View<'tree, Tree> {
    fn new(tree: Tree, target: u32) -> Self {
        Self {
            tree,
            target,
            _lifetime: PhantomData::default(),
        }
    }

    pub fn proxy(&self) -> &Proxy {
        &self.tree.proxies[self.target as usize]
    }

    pub fn parent(&self) -> Result<Self, Error>
    where
        Tree: Copy,
    {
        Ok(Self::new(self.tree, {
            let pid = self.proxy().parent;
            if pid == self.target {
                return Err(Error::ParentOfRoot);
            }
            pid
        }))
    }

    pub fn child(&self, index: u8) -> Result<Self, Error>
    where
        Tree: Copy,
    {
        Ok(Self::new(
            self.tree,
            match self.proxy().data {
                ProxyData::Branch(c) => *c.get(index as usize).ok_or(Error::OutOfRange(index))?,
                _ => return Err(Error::NoChildren),
            },
        ))
    }

    pub fn children(&self) -> Box<dyn Iterator<Item = Self> + '_>
    where
        Tree: Copy,
    {
        match self.proxy().data {
            ProxyData::Branch(c) => {
                Box::new(c.into_iter().map(|target| Self::new(self.tree, target)))
            }
            _ => Box::new([].into_iter()),
        }
    }
    pub fn data(&self) -> Option<&T> {
        match self.proxy().data {
            ProxyData::Leaf(lid) => Some(&self.tree.leaf_data[lid as usize]),
            _ => None,
        }
    }
}

impl<'tree, T: 'tree, Tree: DerefMut<Target = Octree<T>> + 'tree> View<'tree, Tree> {
    pub fn parent_mut(&mut self) -> Result<&mut Self, Error> {
        let pid = self.proxy().parent;
        if pid == self.target {
            return Err(Error::ParentOfRoot);
        }
        self.target = pid;
        Ok(self)
    }

    pub fn child_mut(&mut self, index: u8) -> Result<&mut Self, Error> {
        self.target = match self.proxy().data {
            ProxyData::Branch(c) => *c.get(index as usize).ok_or(Error::OutOfRange(index))?,
            _ => return Err(Error::NoChildren),
        };
        Ok(self)
    }
}
