#![allow(non_snake_case)]
#![feature(rand)]

extern crate shaku;
#[macro_use] extern crate shaku_derive;
extern crate rand;

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use shaku::ContainerBuilder;
use rand::Rng;

trait Foo : Send {
    fn get_value(&self) -> usize;
    fn set_value(&mut self, _: usize);
}

#[derive(Component)]
#[interface(Foo)]
struct FooImpl {
    value: usize,
}

impl Foo for FooImpl {
    fn get_value(&self) -> usize {
        self.value
    }

    fn set_value(&mut self, val: usize) {
        self.value = val;
    }
}

static NB_THREADS: usize = 10;
static MAX_SLEEP_TIME: u64 = 2000;

#[test]
fn simple_multithreaded_resolve_ref() {
    // Build container
    let mut builder = ContainerBuilder::new();
    builder
        .register_type::<FooImpl>()
        .as_type::<Foo>()
        .with_named_parameter("value", 17 as usize);

    let container = builder.build().unwrap();
    let shared_container = Arc::new(Mutex::new(container));

    // Launch a few threads where each will try to resolve `Foo`
    let mut handles = Vec::new();
    for i in 0..NB_THREADS {
        let shared_container = shared_container.clone(); // local clones to be moved into the thread

        handles.push(thread::Builder::new()
            .name(format!("reader #{}", &i).into())
            .spawn(move || {
                // Inject some randomness in the test
                let sleep = Duration::from_millis(std::__rand::thread_rng().gen_range(0, MAX_SLEEP_TIME));
                let handle = thread::current();
                thread::sleep(sleep);

                // Get a handle on the container
                {
                    let mut container = shared_container.lock().unwrap();
                    let foo = container.resolve_ref::<Foo>().unwrap();
                    assert_eq!(foo.get_value(), 17);
                    println!("In thread {:?} > resolve ok > value = {}", &handle.name().unwrap(), foo.get_value());
                } // release the lock
            }).unwrap()
        );
    }

    // Wait until all the threads are done
    for i in 0..NB_THREADS {
        handles.remove(0).join().expect(format!("Couldn't join thread {}", i).as_str());
    }
}

#[test]
fn simple_multithreaded_resolve_ref_n_mut() {
    let first_value = 17 as usize;
    // Build container
    let mut builder = ContainerBuilder::new();
    builder
        .register_type::<FooImpl>()
        .as_type::<Foo>()
        .with_named_parameter("value", first_value);

    let container = builder.build().unwrap();
    let shared_container = Arc::new(Mutex::new(container));
    let latest_data : Arc<Mutex<usize>> = Arc::new(Mutex::new(first_value));

    // Launch a few threads where each will try to resolve `Foo`
    let mut handles = Vec::new();    
    for i in 0..NB_THREADS {
        let (shared_container, latest_data) = (shared_container.clone(), latest_data.clone()); // local clones to be moved into the thread

        handles.push(thread::Builder::new()
            .name(format!("reader #{}", &i).into())
            .spawn(move || {
                // Inject some randomness in the test
                let sleep = Duration::from_millis(std::__rand::thread_rng().gen_range(0, MAX_SLEEP_TIME));
                let handle = thread::current();
                thread::sleep(sleep);

                // Resolve the container
                let use_mut = std::__rand::thread_rng().gen_range(0, 10) < 5;
                {
                    let mut container = shared_container.lock().unwrap();

                    if use_mut {
                        let mut foo = container.resolve_mut::<Foo>().unwrap();
                        let new_value : usize = std::__rand::thread_rng().gen_range(0, 256);
                        foo.set_value(new_value);
                        assert_eq!(foo.get_value(), new_value);
                        
                        let mut data = latest_data.lock().unwrap();
                        *data = new_value;

                        println!("In thread {:?} > resolve ok > value changed to {}", &handle.name().unwrap(), foo.get_value());
                    }
                    else {
                        let foo = container.resolve_ref::<Foo>().unwrap();
                        let data = latest_data.lock().unwrap();

                        println!("In thread {:?} > resolve ok > value should be {}", &handle.name().unwrap(), *data);
                        assert_eq!(foo.get_value(), *data);
                    }
                } // release the lock
            }).unwrap()
        );
    }

    // Wait until all the threads are done
    for i in 0..NB_THREADS {
        handles.remove(0).join().expect(format!("Couldn't join thread {}", i).as_str());
    }
}

#[test]
fn simple_multithreaded_resolve_n_own() {
    let first_value = 17 as usize;
    // Build container
    let mut builder = ContainerBuilder::new();
    builder
        .register_type::<FooImpl>()
        .as_type::<Foo>()
        .with_named_parameter("value", first_value);

    let container = builder.build().unwrap();
    let shared_container = Arc::new(Mutex::new(container));
    let latest_data : Arc<Mutex<usize>> = Arc::new(Mutex::new(first_value));
    let was_owned = Arc::new(Mutex::new(false));

    // Launch a few threads where each will try to resolve `Foo`
    let mut handles = Vec::new();    
    let owner = std::__rand::thread_rng().gen_range(0, 10);
    println!("Owner is {}", owner);

    for i in 0..NB_THREADS {
        let (shared_container, latest_data, was_owned) = (shared_container.clone(), latest_data.clone(), was_owned.clone()); // local clones to be moved into the thread

        handles.push(thread::Builder::new()
            .name(format!("reader #{}", &i).into())
            .spawn(move || {
                // Inject some randomness in the test
                let sleep = Duration::from_millis(std::__rand::thread_rng().gen_range(0, MAX_SLEEP_TIME));
                let handle = thread::current();
                thread::sleep(sleep);

                // Resolve the container
                if i == owner {
                    let mut container = shared_container.lock().unwrap();
                    let foo = container.resolve::<Foo>().unwrap();
                    let data = latest_data.lock().unwrap();
                    println!("In thread {:?} > owner > resolve ok > value should be {}", &handle.name().unwrap(), *data);
                    assert_eq!(foo.get_value(), *data);

                    *was_owned.lock().unwrap() = true;
                } else if i != owner {
                    let use_mut = std::__rand::thread_rng().gen_range(0, 10) < 5;
                    {
                        let mut container = shared_container.lock().unwrap();

                        if *was_owned.lock().unwrap() {
                            let err = container.resolve_ref::<Foo>();
                            assert!(err.is_err());
                            println!("In thread {:?} > resolve ok > was owned", &handle.name().unwrap());
                        } else {
                            if use_mut {
                                let mut foo = container.resolve_mut::<Foo>().unwrap();
                                let new_value : usize = std::__rand::thread_rng().gen_range(0, 256);
                                foo.set_value(new_value);
                                assert_eq!(foo.get_value(), new_value);
                                
                                let mut data = latest_data.lock().unwrap();
                                *data = new_value;

                                println!("In thread {:?} > resolve ok > value changed to {}", &handle.name().unwrap(), foo.get_value());
                            }
                            else {
                                let foo = container.resolve_ref::<Foo>().unwrap();
                                let data = latest_data.lock().unwrap();

                                println!("In thread {:?} > resolve ok > value should be {}", &handle.name().unwrap(), *data);
                                assert_eq!(foo.get_value(), *data);
                            }
                        }
                    } // release the lock
                }
            }).unwrap()
        );
    }

    // Wait until all the threads are done
    for i in 0..NB_THREADS {
        handles.remove(0).join().expect(format!("Couldn't join thread {}", i).as_str());
    }
}