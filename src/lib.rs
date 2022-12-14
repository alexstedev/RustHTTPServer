use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::prelude::*;
use std::net::TcpListener;
use std::path::Path;

mod file_parser;
mod http_parser;
pub mod request;
pub mod response;
mod threadpool;
use file_parser::FileParser;
use request::Request;
use response::Response;
use threadpool::ThreadPool;

pub struct RustHTTPServer {
    /// The amount of worker threads used to handle requests
    amount_of_threads: usize,
    // Contains all the routes for http resources on the server
    routes: HashMap<String, fn(Request, Response) -> Response>,
    // Contains all the middleware for the servers resources
    middleware: Vec<(String, fn(Request, Response) -> (Request, Response, bool))>,
}

impl RustHTTPServer {
    /// Returns a RustHTTPServer HTTP server instance with worker threads equal to the specified amount.
    ///
    /// #Panics
    ///
    /// panics if amount of threads is 0
    pub fn new(amount_of_threads: usize) -> RustHTTPServer {
        return RustHTTPServer {
            amount_of_threads: amount_of_threads,
            routes: HashMap::new(),
            middleware: Vec::new(),
        };
    }

    /// Add middleware for specified resources.
    ///
    /// The middleware function takes inn a function that returns a modified response and request, aswell as a boolean is true if the request should be forwarded or false if you wish the server to write the current response.
    pub fn middle(
        &mut self,
        path: &str,
        function: fn(Request, Response) -> (Request, Response, bool),
    ) {
        let mut path_string = String::from(path);
        // Remove trailing / so that pathing is agnostic towards /example/ or /example
        match path_string.pop() {
            Some(last_char) => {
                if last_char != '/' || path_string.len() == 0 {
                    path_string.push(last_char)
                }
            }
            None => {
                path_string.push('/');
            }
        };
        self.middleware.push((path_string, function));
    }

    /// Add a http resource route which takes in the request and a premade respons, then returns a modifed response that is written to the client
    pub fn route(&mut self, path: &str, function: fn(Request, Response) -> Response) {
        let mut path_string = String::from(path);
        // Remove trailing / so that pathing is agnostic towards /example/ or /example
        match path_string.pop() {
            Some(last_char) => {
                if last_char != '/' || path_string.len() == 0 {
                    path_string.push(last_char)
                }
            }
            None => {
                path_string.push('/');
            }
        };
        if self.routes.contains_key(&path_string) {
            println!(
                "Warning: Route defined twice ({}), using latest definition",
                path
            );
            self.routes.remove(&path_string);
        }
        self.routes.insert(path_string, function);
    }

    /// Add a file to routes, it's route is equal to the path where the file lies
    pub fn route_file(&mut self, path: &str) {
        fn function(req: Request, mut res: Response) -> Response {
            if req.method == "GET" {
                let path = req.url;
                let path_split = path.split('.');
                let file_ending = match path_split.last() {
                    Some(file_ending) => file_ending,
                    None => "",
                };
                let file_type = FileParser::get_type(file_ending);
                // remove first / from path and read metadata then file
                match fs::metadata(&path[1..]) {
                    Ok(metadata) => {
                        let mut contents = vec![0; metadata.len() as usize];
                        match fs::File::open(&path[1..]) {
                            Ok(mut file) => {
                                let result = file.read(&mut contents);
                                match result {
                                    Ok(_) => {
                                        res.status(200);
                                        res.body_bytes(contents);
                                        res.header("content-type", file_type);
                                    }
                                    Err(error) => {
                                        println!("{}", error);
                                        res.status(500);
                                    }
                                }
                            }
                            Err(error) => {
                                println!("{}", error);
                                res.status(500);
                            }
                        }
                    }
                    Err(error) => {
                        println!("{}", error);
                        res.status(500);
                    }
                }
            }
            return res;
        };
        // Replace Windows specific backslashes in path with forward slashes
        let result = path.replace("\\", "/");
        let route_path = format!("/{}", result);
        RustHTTPServer::route(self, &route_path, function);
    }

    /// Recursive function that adds all the files in the public folder to the server routes
    fn add_static_files(&mut self, directory: &Path, path: &str) {
        let dir_iter = fs::read_dir(path).unwrap();

        // Add all files to path hashmap, for each directory in the public folder we run this function recursivly
        for item in dir_iter {
            match item {
                Ok(item_uw) => {
                    let item_path = item_uw.path().into_os_string().into_string().unwrap();
                    let item_metadata = item_uw.metadata().unwrap();
                    if item_metadata.is_dir() {
                        RustHTTPServer::add_static_files(self, directory, &item_path);
                    } else {
                        RustHTTPServer::route_file(self, &item_path);
                    }
                }
                Err(error) => {
                    println!("{}", error);
                }
            };
        }
    }

    /// Make all the files in the specified directory publicly avalible
    pub fn public(&mut self, dir_name: &str) {
        let path = env::current_dir().unwrap();
        let new_root_dir = path.join(dir_name);
        // Set the specified directory as the root when reading files
        assert!(env::set_current_dir(&new_root_dir).is_ok());
        let dir = env::current_dir().unwrap();
        self.add_static_files(dir.as_path(), "");
    }

    /// Bind the server to the specified IP address and listen for inncomming http requests
    pub fn bind(&mut self, ip: &str) -> String {
        let listener = match TcpListener::bind(ip) {
            Ok(result) => result,
            Err(error) => {
                return String::from(format!("Failed to bind to ip: {}", error));
            }
        };
        // Sort middleware by length
        self.middleware.sort_by(|a, b| a.0.len().cmp(&b.0.len()));

        // clone routes and middleware
        let routes_clone = self.routes.clone();
        let middleware_clone = self.middleware.clone();

        // Create threadpool
        let pool = ThreadPool::new(self.amount_of_threads, routes_clone, middleware_clone);

        println!("RustHTTPServer server listening on: http://{}", ip);
        for stream in listener.incoming() {
            match stream {
                Ok(stream_uw) => {
                    pool.execute(stream_uw);
                }
                Err(error) => println!("{}", error),
            }
        }
        return String::from("Shutting down.");
    }
}
