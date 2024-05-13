use std::any::type_name;

use anyhow::anyhow;

pub trait OptionExt<T> {
    fn foreach<F, U>(&self, f: F) where F: FnOnce(&T) -> U;

    fn foreach_mut<F, U>(&mut self, f: F) where F: FnOnce(&mut T) -> U;

    fn into_foreach<F, U>(self, f: F) where F: FnOnce(T) -> U;

    fn as_result(&self) -> anyhow::Result<&T>;

    fn as_result_mut(&mut self) -> anyhow::Result<&mut T>;

    fn into_result(self) -> anyhow::Result<T>;
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

    fn as_result(&self) -> anyhow::Result<&T> {
        Ok(self.as_ref().ok_or(anyhow!("Option<{}> is none", type_name::<T>()))?)
    }

    fn as_result_mut(&mut self) -> anyhow::Result<&mut T> {
        Ok(self.as_mut().ok_or(anyhow!("Option<{}> is none", type_name::<T>()))?)
    }

    fn into_result(self) -> anyhow::Result<T> {
        Ok(self.ok_or(anyhow!("Option<{}> is none", type_name::<T>()))?)
    }
}