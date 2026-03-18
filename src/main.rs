//   ▄▄▄  ▄▄                             ▄▄▄▄▄▄▄          
//  █▀██  ██           █▄               █▀██▀▀▀           
//    ██  ██  ▄        ██    ▄            ██              
//    ██  ██  ███▄███▄ ████▄ ████▄▄▀▀█▄   ███▀▄█▀█▄▀██ ██▀
//    ██  ██  ██ ██ ██ ██ ██ ██   ▄█▀██ ▄ ██  ██▄█▀  ███  
//    ▀█████▄▄██ ██ ▀█▄████▀▄█▀  ▄▀█▄██ ▀██▀ ▄▀█▄▄▄▄██ ██▄
                                                                              
// write shaders, crash tab, look cute doing it <3
// Inspired by ShaderToy

// Copyright 2026 Servus Altissimi (Pseudonym)

// Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
// The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#![allow(non_snake_case)]

mod gpu;
mod highlight;
mod js;
mod app;
mod components;

use std::cell::RefCell;
use futures_channel::mpsc::{self, UnboundedReceiver, UnboundedSender};

// TX_SLOT / RX_SLOT form a one-shot channel used to hand the shader
// source from the Dioxus component tree (which owns the editor state) down
// into the render coroutine that lives outside the component lifecycle.
// Using thread_local! + RefCell here because wasm is single-threaded, so
// there is no need for Arc<Mutex<_>>; take() is used to move the receiver
// into the coroutine exactly once, leaving None behind to prevent accidental
// double-takes.

// ERR_TX / ERR_RX is the reverse path: the GPU/render coroutine
// sends compilation errors (or an empty string on success) back up so the
// component can display them in the error pane without any shared mutable
// state visible to the component layer.
thread_local! {
    static TX_SLOT: RefCell<Option<UnboundedSender<String>>>   = RefCell::new(None);
    static RX_SLOT: RefCell<Option<UnboundedReceiver<String>>> = RefCell::new(None);
    static ERR_TX:  RefCell<Option<UnboundedSender<String>>>   = RefCell::new(None);
    static ERR_RX:  RefCell<Option<UnboundedReceiver<String>>> = RefCell::new(None);
}

fn main() {
    let (tx,  rx)  = mpsc::unbounded::<String>();
    let (etx, erx) = mpsc::unbounded::<String>();
    TX_SLOT.with(|s| *s.borrow_mut() = Some(tx));
    RX_SLOT.with(|s| *s.borrow_mut() = Some(rx));
    ERR_TX.with(|s|  *s.borrow_mut() = Some(etx));
    ERR_RX.with(|s|  *s.borrow_mut() = Some(erx));
    dioxus::launch(app::App);
}
