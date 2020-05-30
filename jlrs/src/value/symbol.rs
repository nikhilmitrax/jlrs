//! Symbols represent identifiers like module and function names.

use crate::global::Global;
use jl_sys::{jl_sym_t, jl_symbol_n, jl_symbol_name};
use std::ffi::CStr;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::marker::PhantomData;

/// `Symbol`s are used Julia to represent identifiers, `:x` represents the `Symbol` `x`. Things
/// that can be accessed using a `Symbol` include submodules, functions, and globals. However,
/// the methods that provide this functionality in `jlrs` can use strings instead.
///
/// This struct implements [`JuliaTypecheck`] and [`Cast`]. It can be used in combination with
/// [`DataType::is`] and [`Value::is`]; if the check returns` true` the [`Value`] can be cast to
/// `Symbol`:
///
/// ```
/// # use jlrs::prelude::*;
/// # use jlrs::util::JULIA;
/// # fn main() {
/// # JULIA.with(|j| {
/// # let mut julia = j.borrow_mut();
/// julia.frame(2, |global, frame| {
///     let symbol_func = Module::core(global).function("Symbol")?;
///     let symbol_str = Value::new(frame, "+")?;
///     let symbol_val = symbol_func.call1(frame, symbol_str)?.unwrap();
///     assert!(symbol_val.is::<Symbol>());
///
///     let symbol = symbol_val.cast::<Symbol>()?;
///     assert!(Module::base(global).function(symbol).is_ok());
///     Ok(())
/// }).unwrap();
/// # });
/// # }
/// ```
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct Symbol<'base>(*mut jl_sym_t, PhantomData<&'base ()>);

impl<'base> Symbol<'base> {
    pub(crate) unsafe fn wrap(ptr: *mut jl_sym_t) -> Self {
        Symbol(ptr, PhantomData)
    }

    #[doc(hidden)]
    #[cfg_attr(tarpaulin, skip)]
    pub unsafe fn ptr(self) -> *mut jl_sym_t {
        self.0
    }

    /// Convert the given string to a `Symbol`.
    pub fn new<S: AsRef<str>>(global: Global<'base>, symbol: S) -> Self {
        Symbol::from((global, symbol))
    }

    /// Extend the `Symbol`'s lifetime. `Symbol`s are not garbage collected, but a `Symbol`
    /// returned as a [`Value`] from a Julia function inherits the frame's lifetime when it's cast
    /// to a `Symbol`. Its lifetime can be safely extended from `'frame` to `'global` using this
    /// method.
    pub fn extend<'global>(self, _: Global<'global>) -> Symbol<'global> {
        unsafe { Symbol::wrap(self.ptr()) }
    }
}

impl<'base> Into<String> for Symbol<'base> {
    fn into(self) -> String {
        unsafe {
            let ptr = jl_symbol_name(self.ptr()).cast();
            let symbol = CStr::from_ptr(ptr);
            symbol.to_str().unwrap().into()
        }
    }
}

impl<'base, S> From<(Global<'base>, S)> for Symbol<'base>
where
    S: AsRef<str>,
{
    fn from((_, symbol): (Global<'base>, S)) -> Self {
        unsafe {
            let symbol_str = symbol.as_ref();
            let symbol_ptr = symbol_str.as_ptr();
            let symbol = jl_symbol_n(symbol_ptr.cast(), symbol_str.as_bytes().len());
            Symbol::wrap(symbol)
        }
    }
}

impl<'scope> Debug for Symbol<'scope> {
    #[cfg_attr(tarpaulin, skip)]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        unsafe {
            let ptr = jl_symbol_name(self.ptr()).cast();
            let symbol = CStr::from_ptr(ptr);
            f.debug_tuple("Symbol").field(&symbol).finish()
        }
    }
}