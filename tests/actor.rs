#![feature(box_syntax)]

extern crate syd;
extern crate rustc_serialize;
use syd::actors::Actor;

#[derive(RustcDecodable, RustcEncodable, PartialEq, Debug)]
pub enum Return {
    Foo(u8),
    Bar(u8),
}

#[derive(RustcDecodable, RustcEncodable)]
pub enum Message {
    Foo { x: u8, y: u8 },
    Bar { z: u8 },
}

struct ExampleActor {}

impl Actor<Message, Return> for ExampleActor {
    fn handle_msg(&self, message: Message) -> Return {
        match message {
            Message::Foo { x, y } => Return::Foo(x+y),
            Message::Bar { z } => Return::Bar(10+z),
        }
    }
}

#[test]
fn sends_messages() {
    let actor1 = ExampleActor {};
    let actor2 = ExampleActor {};
    let result = actor1.send_msg(Message::Foo {x:2, y:5}, box actor2);
    assert_eq!(Return::Foo(7), result);
}
