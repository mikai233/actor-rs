use std::any::type_name;

use eyre::anyhow;

pub trait OptionExt<T> {
    fn foreach<F, U>(&self, f: F) where F: FnOnce(&T) -> U;

    fn foreach_mut<F, U>(&mut self, f: F) where F: FnOnce(&mut T) -> U;

    fn into_foreach<F, U>(self, f: F) where F: FnOnce(T) -> U;

    fn as_result(&self) -> eyre::Result<&T>;

    fn as_result_mut(&mut self) -> eyre::Result<&mut T>;

    fn into_result(self) -> eyre::Result<T>;
}

impl<T> OptionExt<T> for Option<T> {
    fn foreach<F, U>(&self, f: F) where F: FnOnce(&T) -> U {
        if let Some(v) = self {
            f(v);
        }
    }

    fn foreach_mut<F, U>(&mut self, f: F) where F: FnOnce(&mut T) -> U {
        if let Some(v) = self {
            f(v);
        }
    }

    fn into_foreach<F, U>(self, f: F) where F: FnOnce(T) -> U {
        if let Some(v) = self {
            f(v);
        }
    }

    fn as_result(&self) -> eyre::Result<&T> {
        let name = type_name::<T>();
        Ok(self.as_ref().ok_or(anyhow!("type of {} is none", name))?)
    }

    fn as_result_mut(&mut self) -> eyre::Result<&mut T> {
        let name = type_name::<T>();
        Ok(self.as_mut().ok_or(anyhow!("type of {} is none", name))?)
    }

    fn into_result(self) -> eyre::Result<T> {
        let name = type_name::<T>();
        Ok(self.ok_or(anyhow!("type of {} is none", name))?)
    }
}