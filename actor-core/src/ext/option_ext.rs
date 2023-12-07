pub trait OptionExt<T> {
    fn foreach<F, U>(&self, f: F) where F: Fn(&T) -> U;
    fn foreach_mut<F, U>(&mut self, f: F) where F: Fn(&mut T) -> U;
    fn into_foreach<F, U>(self, f: F) where F: FnOnce(T) -> U;
}

impl<T> OptionExt<T> for Option<T> {
    fn foreach<F, U>(&self, f: F) where F: Fn(&T) -> U {
        if let Some(v) = self {
            f(v);
        }
    }

    fn foreach_mut<F, U>(&mut self, f: F) where F: Fn(&mut T) -> U {
        if let Some(v) = self {
            f(v);
        }
    }

    fn into_foreach<F, U>(self, f: F) where F: FnOnce(T) -> U {
        if let Some(v) = self {
            f(v);
        }
    }
}