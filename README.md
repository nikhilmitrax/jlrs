# jlrs

[![Build Status](https://travis-ci.com/Taaitaaiger/jlrs.svg?branch=master)](https://travis-ci.com/Taaitaaiger/jlrs)
[![Coverage Status](https://coveralls.io/repos/github/Taaitaaiger/jlrs/badge.svg?branch=master)](https://coveralls.io/github/Taaitaaiger/jlrs?branch=master)
[![License:MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)


The main goal behind `jlrs` is to provide a simple and safe interface to the Julia C API. Using 
this crate you can call arbitrary Julia code from Rust, including your own, and share data 
between the two languages. Currently this crate has only been tested on Linux, if you try to use 
it on another OS it will likely fail to generate the bindings to Julia.

## Generating the bindings
This crate depends on `jl-sys` which contains the raw bindings to the Julia C API, these are
generated by `bindgen`. The recommended way to install Julia is to download the binaries from
the official website, which is distributed in an archive containing a directory called
`julia-x.y.z`. This directory contains several other directories, including a `bin` directory
containing the `julia` executable.

In order to ensure the `julia.h` header file can be found, either `/usr/include/julia/julia.h` 
exists, or you have to set the `JL_PATH` environment variable to `/path/to/julia-x.y.z`. 
Similarly, in order to load `libjulia.so` you must add `/path/to/julia-x.y.z/lib` to the
`LD_LIBRARY_PATH` environment variable.

## Using this crate
The first thing you should do is `use` the `prelude`-module with an asterisk, this will
bring all the structs and traits you're likely to need in scope. Before you can use Julia it
must first be initialized. You do this by creating a `Julia` with `Julia::new`, this
method forces you to pick a `stack size`. You will learn how to choose this value in the
section about `memory management`. Note that this method can only be called once, if you
drop the `Julia` you won't be able to create a new one and have to restart the  entire
program.

With the `Julia` you can do two things: you can call `Julia::include` to include your
own Julia code, and `Julia::session` to interact with Julia. If you want to create arrays
with more than three dimensions or borrow arrays with more than one, you should include
`jlrs.jl` first, which you can find in the root of this crate's github repository. This is
necessary because this functionality currently depends on some Julia code defined in that file.
The latter method takes a closure with a single argument, a mutable reference to a
`Session`. Using this `Session` you can do useful things inside the closure.

In order to actually call a function, you need three things:
 - A place to store the function output that's protected from garbage collection
 - Arguments to call the function with, also protected from garbage collection
 - A handle to the function

The `Session` lets you take care of all the preliminary work; with
`Session::new_unassigned` you create a safe place for the output to go, while other methods
like `Session::new_primitive`, `Session::new_string` and `Session::new_owned_array` let
you transfer primitive datatypes like `u8` and `f32`, strings, and n-dimensional arrays to
Julia. For a full overview of the possibilities, you should take a look at the documentation
for `Session`.

In the case of named functions, ie those defined inside a module, you must first acquire a
handle to that module. You can can a handle to the `Main`, `Base` and  `Core` modules with the
methods `Session::main_module`, `Session::base_module` and `Session::core_module`
respectively. You can traverse the path to a deeper module with `Module::submodule`. Finally,
you get a handle to a function with `Module::function`. Because these are global handles
they don't need to be protected from garbage collection.

There's something a bit special about functions though: there's no real way to differentiate
between functions and other globals in a module. For example, there's nothing that prevents
you from calling `Base.pi` as a function. It's not possible to check if you call functions
with the correct arguments, either. It's up to you to ensure you call things correctly.
Failing to do so will only result in an error being returned, though, rather than crash your
program.

With all these things in hand, it is time to call `Session::execute`. This method works just
like `Julia::session` does: it takes closure with a single argument, a mutable reference
to an `ExecutionContext`. Besides letting you copy data from Julia to Rust with
`ExecutionContext::try_unbox`, you will need this reference when calling functions using the
`Call` trait.

Both `Julia::session` and `Session::execute` have generic return types, which lets you
easily return the results of your computations. As a simple example, this is how you can add
two numbers:

```rust
use jlrs::prelude::*;

fn main() {
    let mut runtime = unsafe { Julia::new(16).unwrap() };

    let output = runtime.session(|session| {
        let output = session.new_unassigned()?;
        let i = session.new_primitive(2u64)?;
        let j = session.new_primitive(1u32)?;

        session.execute(|exec_ctx| {
            let add = exec_ctx.base_module().function("+")?;
            let result = add.call2(exec_ctx, output, i, j)?;
            exec_ctx.try_unbox::<u64>(&result)
        })
    }).unwrap();

    assert_eq!(output, 3);
}
```

## Memory management
So far you've seen that you can use a `Session` to allocate data and an `ExecutionContext`
to use that data. The data allocated using a `Session` is valid until the session ends and
nothing prevents you from allocating more data and calling `Session::execute` again. The
actual allocations happen when `Session::execute` is called. If nothing was allocated,
calling this function will take one slot on the stack, otherwise it will take as many slots as
allocations plus three.

It's also possible to allocate temporary data with `Session::with_temporaries`, which works
mostly the same way as `Julia::session` and `Session::execute` do, except its argument
is an `AllocationContext` rather than a mutable reference to one. The `AllocationContext`
offers you the same interface as `Session` does, with two major differences:
 - `AllocationContext::execute` takes the context by value rather than by reference, you
   have to stop using the `AllocationContext` after calling `AllocationContext::execute`.
 - Data allocated by an `AllocationContext` is only valid within that context, rather than
   the entire session.

So, to summarize, in order to estimate how large your stack size should be, you need to check
where you call `Session::execute`, `Session::with_temporaries` and
`AllocationContext::execute` and figure out how many items you're allocating to get a rough
estimate for how many slots you need. In case your computations fail due to exceeding the
stack size, you can use `Julia::set_stack_size` to create a larger one.

## Limitations
Calling Julia is entirely single-threaded. You won't be able to use the `Julia` from
another thread and while Julia is doing stuff you won't be able to interact with it.
Support for multithreading in Julia is currently in an experimental phase, there might still
be options to use this functionality in order to build experimental support for some kind of
multithreaded task-like system, but that has not been investigated yet.
