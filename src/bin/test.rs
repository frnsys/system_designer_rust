#![feature(box_syntax)]

extern crate syd;
extern crate futures;
extern crate threadpool;
extern crate futures_cpupool;
extern crate rustc_serialize;
use std::fmt::Debug;
use std::thread;
use std::cmp::min;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use threadpool::ThreadPool;
use futures::{BoxFuture, Future, collect, finished};
use futures_cpupool::CpuPool;
use syd::actors::{Actor, Inbox, Message};
use rustc_serialize::{json, Decodable, Encodable};

// max amount of messages to process per actor
// during an activation
const THROUGHPUT: usize = 100;

#[derive(RustcDecodable, RustcEncodable, PartialEq, Debug)]
pub enum ExampleReturn {
    Foo(u8),
    Bar(u8),
}

#[derive(RustcDecodable, RustcEncodable)]
pub enum ExampleMessage {
    Foo { x: u8, y: u8 },
    Bar { z: u8 },
}

enum ActorPath {
    Local { id: usize },
    Remote { addr: SocketAddr, id: usize }
}

struct ExampleActor {
    id: usize,
    name: String,
    inbox: Inbox<ExampleMessage>
}
impl ExampleActor {
    pub fn new(id: usize) -> ExampleActor {
        ExampleActor {
            id: id,
            name: format!("actor-{}", id),
            inbox: Arc::new(RwLock::new(Vec::new()))
        }
    }

    // TODO see if we can avoid using boxes here
    // <https://github.com/alexcrichton/futures-rs/blob/master/TUTORIAL.md#returning-futures>
    pub fn decide(&self) -> BoxFuture<String, String> {
        finished(self.name.clone()).boxed()
    }
}

impl Actor for ExampleActor {
    type M = ExampleMessage;
    type R = ExampleReturn;
    fn handle_msg(&self, message: Self::M) -> Self::R {
        match message {
            ExampleMessage::Foo { x, y } => ExampleReturn::Foo(x+y),
            ExampleMessage::Bar { z } => ExampleReturn::Bar(10+z),
        }
    }

    fn inbox(&self) -> &Inbox<Self::M> {
        &self.inbox
    }
}

type ActorRef<A: Actor> = Arc<RwLock<Box<A>>>;
type ActorVec<A: Actor> = Vec<ActorRef<A>>;
type ActorVecRef<A: Actor> = Arc<RwLock<ActorVec<A>>>;

/// Manages a pool of actors,
/// dispatching them to threads when they have queued messages.
fn dispatcher<A>(actors : ActorVecRef<A>, n_threads : usize) where A : Actor + 'static {
    thread::spawn(move || {
        // TODO should this just be another futures cpupool?
        let pool = ThreadPool::new(n_threads);
        loop {
            let actors = actors.clone();
            let actors = actors.read().unwrap();
            for actor in actors.iter() {
                let actor_r = actor.read().unwrap();
                let n_queued = {
                    let inbox = actor_r.inbox().read().unwrap();
                    inbox.len()
                };
                if n_queued > 0 {
                    let n_messages = min(n_queued, THROUGHPUT);
                    let mut inbox = actor_r.inbox().write().unwrap();
                    let chunk : Vec<A::M> = inbox.drain(0..n_messages).collect();
                    let actor = actor.clone();
                    pool.execute(move || {
                        for msg in chunk {
                            let actor = actor.write().unwrap();
                            let _ = actor.handle_msg(msg);
                        }
                    });
                }
            }
        }
    });
}

fn main() {
    let n_workers = 10;
    let pool = CpuPool::new(n_workers);
    let actors = Arc::new(RwLock::new(Vec::new()));
    {
        let mut actors_ = actors.write().unwrap();
        for i in 0..10 {
            let actor = box ExampleActor::new(i);
            actors_.push(Arc::new(RwLock::new(actor)));
        }
    }

    dispatcher(actors.clone(), 10);

    // call decide methods on agents
    // this is basically the manager (aside from manager-leader communication)
    let mut futs = Vec::new();
    for actor in actors.read().unwrap().iter() {
        let actor = actor.write().unwrap();
        futs.push(pool.spawn(actor.decide()));
    }
    let f = collect(futs);
    let f = f.then(|x| {
        println!("{:?}", x);
        x
    });
    let _ = f.wait();
}