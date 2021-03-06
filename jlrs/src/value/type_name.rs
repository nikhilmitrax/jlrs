//! Support for values with the `Core.TypeName` type.
//!
//! The documentation for this module has been slightly adapted from the comments for this struct
//! in [`julia.h`]
//!
//! [`julia.h`]: https://github.com/JuliaLang/julia/blob/96786e22ccabfdafd073122abb1fb69cea921e17/src/julia.h#L380

use super::{
    method_table::MethodTable, module::Module, simple_vector::SimpleVector, symbol::Symbol, Value,
};

use super::array::Array;
use crate::error::{JlrsError, JlrsResult};
use crate::traits::Cast;
use crate::{impl_julia_type, impl_julia_typecheck, impl_valid_layout};
use jl_sys::{jl_typename_t, jl_typename_type};
use std::marker::PhantomData;

/// Describes the syntactic structure of a type and stores all data common to different
/// instantiations of the type, including a cache for hash-consed allocation of `DataType`s.
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
#[repr(transparent)]
pub struct TypeName<'frame>(*mut jl_typename_t, PhantomData<&'frame ()>);

impl<'frame> TypeName<'frame> {
    pub(crate) unsafe fn wrap(typename: *mut jl_typename_t) -> Self {
        TypeName(typename, PhantomData)
    }

    #[doc(hidden)]
    pub unsafe fn ptr(self) -> *mut jl_typename_t {
        self.0
    }

    pub fn name(self) -> Symbol<'frame> {
        unsafe { Symbol::wrap((&*self.ptr()).name) }
    }

    pub fn module(self) -> Module<'frame> {
        unsafe { Module::wrap((&*self.ptr()).module) }
    }

    /// Field names.
    pub fn names(self) -> SimpleVector<'frame> {
        unsafe { SimpleVector::wrap((&*self.ptr()).names) }
    }

    /// Either the only instantiation of the type (if no parameters) or a `UnionAll` accepting
    /// parameters to make an instantiation.
    pub fn wrapper(self) -> Value<'frame, 'static> {
        unsafe { Value::wrap((&*self.ptr()).wrapper) }
    }

    /// Sorted array.
    pub fn cache(self) -> SimpleVector<'frame> {
        unsafe { SimpleVector::wrap((&*self.ptr()).cache) }
    }

    /// Unsorted array.
    pub fn linearcache(self) -> SimpleVector<'frame> {
        unsafe { SimpleVector::wrap((&*self.ptr()).linearcache) }
    }

    pub fn hash(self) -> isize {
        unsafe { (&*self.ptr()).hash }
    }

    pub fn mt(self) -> MethodTable<'frame> {
        unsafe { MethodTable::wrap((&*self.ptr()).mt) }
    }

    /// Incomplete instantiations of this type.
    pub fn partial(self) -> Array<'frame, 'static> {
        unsafe { Array::wrap((&*self.ptr()).partial) }
    }
}

impl<'frame> Into<Value<'frame, 'static>> for TypeName<'frame> {
    fn into(self) -> Value<'frame, 'static> {
        unsafe { Value::wrap(self.ptr().cast()) }
    }
}

unsafe impl<'frame, 'data> Cast<'frame, 'data> for TypeName<'frame> {
    type Output = Self;
    fn cast(value: Value<'frame, 'data>) -> JlrsResult<Self::Output> {
        if value.is::<Self::Output>() {
            return unsafe { Ok(Self::cast_unchecked(value)) };
        }

        Err(JlrsError::NotASymbol)?
    }

    unsafe fn cast_unchecked(value: Value<'frame, 'data>) -> Self::Output {
        Self::wrap(value.ptr().cast())
    }
}

impl_julia_typecheck!(TypeName<'frame>, jl_typename_type, 'frame);
impl_julia_type!(TypeName<'frame>, jl_typename_type, 'frame);
impl_valid_layout!(TypeName<'frame>, 'frame);
