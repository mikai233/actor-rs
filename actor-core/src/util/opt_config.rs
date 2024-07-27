use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptConfig<T> {
    pub value: Option<T>,
}

impl<T> Deref for OptConfig<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for OptConfig<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}