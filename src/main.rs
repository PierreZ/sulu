extern crate notify;
extern crate hyper;
extern crate ansi_term;

use std::env;
use std::process;

use notify::{RecommendedWatcher, Watcher};
use std::sync::mpsc::channel;

use hyper::{ Client, Url};
use hyper::client::response;

use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;

use ansi_term::Colour::*;

use hyper::status::StatusCode;

// watch is watching folders using inotify bindings
fn watch(path: PathBuf) -> notify::Result<()> {
    // Create a channel to receive the events.
    let (tx, rx) = channel();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher: RecommendedWatcher = try!(Watcher::new(tx));

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    try!(watcher.watch(path));

    // This is a simple loop, but you may want to use more complex logic here,
    // for example to handle I/O.
    loop {
        match rx.recv() {
            Ok(notify::Event{ path: Some(path),op:Ok(_)}) => {
                handle_event(path);
                }
            Err(e) => println!("watch error {}", e),
            _ => ()
        }
    }
}

fn handle_event(path: PathBuf){

    // Is it a warpscript?
    if !path.as_os_str().to_str().unwrap().to_string().contains(".mc2") {
        return;
    }
    post_to_egress(path);
}

fn read_file(path: PathBuf) -> (String,String) {

    let display = path.display();

    // Open the path in read-only mode, returns `io::Result<File>`
    let mut file = match File::open(&path) {
        // The `description` method of `io::Error` returns a string that
        // describes the error
        Err(why) => panic!("couldn't open {}: {}", display,
                                                   why.description()),
        Ok(file) => file,
    };

    // Read the file contents into a string, returns `io::Result<usize>`
    let mut s = String::new();
    match file.read_to_string(&mut s) {
        Err(why) => panic!("couldn't read {}: {}", display,
                                                   why.description()),
        Ok(_) => (),
    }
    return (s, path.file_stem().unwrap().to_os_string().into_string().unwrap());
}

fn post_to_egress(path: PathBuf){

    // Loading Warp10 endpoint
    let key = "SULU_ENDPOINT";
    let mut endpoint = String::new();

    match env::var(key) {
        Ok(val) => endpoint = val,
        Err(e) => {
            println!("couldn't interpret {}: {}.\n\n
            Don't forget to export SULU_ENDPOINT or use http://direnv.net/", key, e);
            process::exit(1);
        }
    }

    let mut client = Client::new();
    let uri = match Url::parse(&endpoint){
        Ok(uri) => uri,
        Err(_) => panic!("Not a valid URL"),
    };

    let (body, filename) = read_file(path);

    let client = Client::new();
    let mut response = match client.post(uri).body(body.as_bytes()).send() {
        Ok(response) => response,
        Err(_) => panic!("Broke up"),
    };

    match response.status {
        StatusCode::Ok => {
            let mut buf = String::new();
            match response.read_to_string(&mut buf) {

                Ok(_) => {
                    let out = response.headers.get_raw("X-CityzenData-Elapsed").unwrap();
                    let elapsed = String::from_utf8(out[0].clone()).unwrap();
                    println!("{} WarpScript {}.mc2 executed in {} ms:\n{}", Green.paint("◉"), filename, elapsed, buf);
                },
                Err(_) => panic!("I gave up"),
            };
        },
        // Trap all status different than 200
        _ => {
            let mut buf = String::new();
            match response.read_to_string(&mut buf) {
                Ok(_) => {
                    println!("{} WarpScript {} executed with errors:\n{}", Red.paint("◉"), filename, buf);
                },
                Err(_) => panic!("I gave up"),
            };
        },
    }

}

fn main() {

    let path = env::current_dir().unwrap();
    println!("Starting watcher on {}...\n", path.display());

    if let Err(err) = watch(path) {
        println!("Error! {:?}", err)
    }
}
