use jlrs::prelude::*;
use std::any::Any;

// This struct contains the data we'll need to call one or more Julia functions, in this case
// `dims` and `iters`. There's also a `sender` that is used to send back the eventual result.
// For more complex types that don't implement `Copy`, you can wrap them in `Option` and use
// `Option::take` to extract them from the struct.
struct MyTask {
    dims: isize,
    iters: isize,
    sender: crossbeam_channel::Sender<JlrsResult<Box<dyn Any + Send + Sync>>>,
}

// `MyTask` is a task we want to be executed, so we need to implement `JuliaTrait`. This requires
// `async_trait` because traits with async methods are not yet available in Rust. Because the 
// task itself is executed on a single thread, it is marked with `?Send`. 
#[async_trait(?Send)]
impl JuliaTask for MyTask {
    // If successful, the data is returned as a boxed `Any`. This way we can have different tasks
    // that return data of different types.
    type T = Box<dyn Any + Send + Sync>;
    // We use the `Sender` from the crossbeam_channel crate to send back results. Even if this
    // task doesn't return a result to the caller, `R` must be set.
    type R = crossbeam_channel::Sender<JlrsResult<Self::T>>;

    // This is the async variation of the closure you give to `Julia::frame` or
    // `Julia::dynamic_frame` when you use the synchronous runtime. The `Global` can be used to
    // access `Module`s and other static data, while the `AsyncFrame` let you create values, call
    // functions, and create nested frames.
    async fn run<'base>(
        &mut self,
        global: Global<'base>,
        frame: &mut AsyncFrame<'base>,
    ) -> JlrsResult<Self::T> {
        // Convert the two arguments to values Julia can work with.
        let dims = Value::new(frame, self.dims)?;
        let iters = Value::new(frame, self.iters)?;

        // Get `complexfunc` in `MyModule`, call it asynchronously with `call_async`, and await
        // the result before casting it to an `f64` (which that function returns). A function that
        // is called with `call_async` is executed on a thread created with `Base.threads.@spawn`.
        let v = Module::main(global)
            .submodule("MyModule")?
            .function("complexfunc")?
            .call_async(frame, [dims, iters])
            .await?
            .unwrap()
            .cast::<f64>()?;

        // Box the result
        Ok(Box::new(v))
    }

    // Override the default implementation of `return_channel` so the result of this task is sent
    // back.
    fn return_channel(&self) -> Option<&crossbeam_channel::Sender<JlrsResult<Self::T>>> {
        Some(&self.sender)
    }
}

fn main() {
    // Initialize the asynchronous runtime. We'll allow a backlog of sixteen pending messages in 
    // the channel that the runtime consumes, two tasks to run simultaneously, give each task a 
    // stack with sixteen slots to protect data from garbage collection, insert a process events
    // every millisecond, and provide the path to jlrs.jl.
    //
    // Okay, that's a lot to unpack. Let's look at those arguments a bit more closely to see why 
    // we need them.
    //
    // The runtime runs in a separate thread. In order to send it tasks and other commands, a 
    // channel is needed. The runtime will receive these messages, but a backlog can build up if a
    // long-running synchronously called function is blocking it.
    //
    // Julia can be started with one or more threads by setting the `JULIA_NUM_THREADS` 
    // environment variable. By default it's 1, and it must be set to a higher value in order for 
    // the asynchronous runtime to work. The `n_threads` argument indicates how many of these 
    // threads can be used to offload function calls to, and must be lower than the number of 
    // threads Julia has available to it.
    //
    // In order to protect the data we get from Julia from being freed by the garbage collector a 
    // stack is maintained. If the stack is too small, jlrs will eventually return an error to
    // indicate it has run out of stack space; if it is too large, you will waste memory. You can
    // find the stack space costs of different operations in the documentation of jlrs.
    //
    // When one or more functions are running in other threads but the runtime has no synchronous
    // work to do, the garbage collector can't run. Similarly, asynchronous events (such as
    // rescheduling a task that has yielded after calling `sleep` or `println`) will not be 
    // handled either. In order to solve this issue, these things are explicitly handled 
    // periodically.
    //
    // In order to use the asynchronous runtime, custom Julia code defined in jlrs.jl must be 
    // used. Things won't work without it.
    //
    // After calling this function we have a `task_sender` we can use to send tasks and requests
    // to include a file to the runtime, and a handle to the thread where the runtime is running.
    let (julia, handle) = unsafe { 
        AsyncJulia::init(16, 2, 16, 1, "../../jlrs.jl")
            .expect("Could not init Julia") 
    };

    // Let's include the custom code our task needs.
    julia.try_include("MyModule.jl").unwrap();

    // Create two channels for two tasks (this is not required but helps distinguish which result
    // belongs to which task).
    let (sender1, receiver1) = crossbeam_channel::bounded(1);
    let (sender2, receiver2) = crossbeam_channel::bounded(1);

    // Send two tasks to the runtime.
    julia.try_new_task(MyTask {
        dims: 4,
        iters: 5_000_000,
        sender: sender1,
    }).unwrap();

    julia.try_new_task(MyTask {
        dims: 6,
        iters: 5_000_000,
        sender: sender2,
    }).unwrap();

    // Receive the result of the first tasks. `Any::downcast_ref` can be used to convert the 
    // result to the appropriate type.
    let x = receiver1.recv().unwrap().unwrap();
    println!("Result of first task: {:?}", x.downcast_ref::<f64>());
    
    let y = receiver2.recv().unwrap().unwrap();
    println!("Result of second task: {:?}", y.downcast_ref::<f64>());

    // `task_sender is the only sender, dropping it will cause the runtime to shut down Julia and
    // itself. We join the handle to wait for everything to shut down cleanly.
    std::mem::drop(julia);
    handle.join().expect("Cannot join").expect("Unable to init Julia");
}
