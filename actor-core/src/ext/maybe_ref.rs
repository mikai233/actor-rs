use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;

pub enum MaybeRef<'a, T> {
    Ref(&'a T),
    Own(T),
}

impl<'a, T> Deref for MaybeRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            MaybeRef::Ref(value) => value,
            MaybeRef::Own(value) => value,
        }
    }
}

impl<'a, T> Debug for MaybeRef<'a, T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MaybeRef::Ref(value) => f.debug_struct("MaybeRef").field("ref", value).finish(),
            MaybeRef::Own(value) => f.debug_struct("MaybeRef").field("own", value).finish(),
        }
    }
}

impl<'a, T> Display for MaybeRef<'a, T>
where
    T: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MaybeRef::Ref(value) => {
                write!(f, "ref:{}", value)
            }
            MaybeRef::Own(value) => {
                write!(f, "own:{}", value)
            }
        }
    }
}

impl<'a, T> PartialEq for MaybeRef<'a, T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.deref().eq(other)
    }
}

impl<'a, T> Eq for MaybeRef<'a, T> where T: Eq {}

impl<'a, T> PartialOrd for MaybeRef<'a, T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.deref().partial_cmp(other)
    }
}

impl<'a, T> Ord for MaybeRef<'a, T>
where
    T: Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.deref().cmp(other)
    }
}
