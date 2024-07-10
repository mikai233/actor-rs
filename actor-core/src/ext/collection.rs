#[macro_export]
macro_rules! hashmap {
    (@single $($x:tt)*) => (());
    (@count $($rest:expr),*) => (<[()]>::len(&[$(hashmap!(@single $rest)),*]));

    ($($key:expr => $value:expr,)+) => { hashmap!($($key => $value),+) };
    ($($key:expr => $value:expr),*) => {
        {
            use ::ahash::HashMapExt;
            let _cap = hashmap!(@count $($key),*);
            let mut _map = ::ahash::HashMap::with_capacity(_cap);
            $(
                let _ = _map.insert($key, $value);
            )*
            _map
        }
    };
}

#[macro_export]
macro_rules! btree_map {
    // trailing comma case
    ($($key:expr => $value:expr,)+) => (btree_map!($($key => $value),+));

    ( $($key:expr => $value:expr),* ) => {
        {
            let mut _map = ::std::collections::BTreeMap::new();
            $(
                let _ = _map.insert($key, $value);
            )*
            _map
        }
    };
}
#[macro_export]
macro_rules! hashset {
    (@single $($x:tt)*) => (());
    (@count $($rest:expr),*) => (<[()]>::len(&[$(hashset!(@single $rest)),*]));

    ($($key:expr,)+) => { hashset!($($key),+) };
    ($($key:expr),*) => {
        {
            use ahash::HashSetExt;
            let _cap = hashset!(@count $($key),*);
            let mut _set = ::ahash::HashSet::with_capacity(_cap);
            $(
                let _ = _set.insert($key);
            )*
            _set
        }
    };
}

#[macro_export]
macro_rules! btree_set {
    ($($key:expr,)+) => (btree_set!($($key),+));

    ( $($key:expr),* ) => {
        {
            let mut _set = ::std::collections::BTreeSet::new();
            $(
                _set.insert($key);
            )*
            _set
        }
    };
}

#[macro_export]
macro_rules! convert_args {
    (keys=$kf:expr, $macro_name:ident !($($k:expr),* $(,)*)) => {
        $macro_name! { $(($kf)($k)),* }
    };
    (keys=$kf:expr, values=$vf:expr, $macro_name:ident !($($k:expr),* $(,)*)) => {
        $macro_name! { $(($kf)($k)),* }
    };
    (keys=$kf:expr, values=$vf:expr, $macro_name:ident !( $($k:expr => $v:expr),* $(,)*)) => {
        $macro_name! { $(($kf)($k) => ($vf)($v)),* }
    };
    (keys=$kf:expr, $macro_name:ident !($($rest:tt)*)) => {
        convert_args! {
            keys=$kf, values=$crate::__id,
            $macro_name !(
                $($rest)*
            )
        }
    };
    (values=$vf:expr, $macro_name:ident !($($rest:tt)*)) => {
        convert_args! {
            keys=$crate::__id, values=$vf,
            $macro_name !(
                $($rest)*
            )
        }
    };
    ($macro_name:ident ! $($rest:tt)*) => {
        convert_args! {
            keys=::std::convert::Into::into, values=::std::convert::Into::into,
            $macro_name !
            $($rest)*
        }
    };
}